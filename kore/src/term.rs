// Terminal layer over sys.rs: Cell grid, frame composition, one assembled frame buffer, and
// input decoding for keys, SGR mouse, and UTF-8. Escape strings and palette values are protocol.

use crate::sys;
use crate::text;

// Attribute bits (kore.c).
pub const A_DIM: u8 = 1;
pub const A_BOLD: u8 = 2;
pub const A_REV: u8 = 4;

// The palette, xterm-256 (kore.c). Cell fg/bg 0 mean the canvas defaults.
pub const C_BG: u8 = 234;
pub const C_TEXT: u8 = 252;
pub const C_FRAME: u8 = 245;
pub const C_RAILC: u8 = 141;
pub const C_CODEC: u8 = 179;
pub const C_WORLDC: u8 = 110;
pub const C_OUTC: u8 = 108;
pub const C_PROMPTC: u8 = 114;
pub const C_OUTPUTSC: u8 = 218;
pub const C_GLOW: u8 = 222;
pub const C_DIRECTIVE: u8 = 66;
pub const C_NUMLIT: u8 = 151;
pub const C_OP: u8 = 117;
pub const C_DEF: u8 = 216;
pub const C_SYM: u8 = 183;
pub const C_REL: u8 = 210;
pub const C_BOOL: u8 = 115;
pub const C_CHAR: u8 = 223;
pub const C_HDR: u8 = 117;
pub const C_ROWLBL: u8 = 242;
pub const C_OK: u8 = 114;
pub const C_ERR: u8 = 203;
pub const C_NIHONGO: u8 = 176;
pub const C_AT: u8 = 213;
pub const C_SEARCH: u8 = 220;
pub const C_CASEUP: u8 = 230;
pub const C_CASELO: u8 = 168;

#[derive(Clone, Copy, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

// g is a NUL-terminated UTF-8 glyph clamped to 7 bytes; cont marks the second column of a
// wide glyph (no glyph of its own, skipped at flush). fg/bg 0 = C_TEXT/C_BG at flush.
#[derive(Clone, Copy)]
pub struct Cell {
    pub g: [u8; 8],
    pub attr: u8,
    pub fg: u8,
    pub bg: u8,
    pub cont: bool,
}

impl Cell {
    // The frame_clear cell: a single space, everything else zero.
    pub fn blank() -> Cell {
        let mut g = [0u8; 8];
        g[0] = b' ';
        Cell {
            g,
            attr: 0,
            fg: 0,
            bg: 0,
            cont: false,
        }
    }

    // The glyph's NUL-terminated byte run.
    pub fn glyph(&self) -> &[u8] {
        let n = self.g.iter().position(|&b| b == 0).unwrap_or(8);
        &self.g[..n]
    }
}

// All-zero cell used for fresh grids and as the base of continuation cells.
fn cell_zeroed() -> Cell {
    Cell {
        g: [0; 8],
        attr: 0,
        fg: 0,
        bg: 0,
        cont: false,
    }
}

// The terminal state (kore.c `T`): grid row-major grid[y*cols+x]; out is the frame
// buffer; cur_shape is the frame's DECSCUSR request, 0 = cursor hidden. The saved termios,
// raw flag, and resize flag live in sys.rs statics.
pub struct Term {
    pub rows: i32,
    pub cols: i32,
    pub grid: Vec<Cell>,
    pub out: Vec<u8>,
    pub cur_x: i32,
    pub cur_y: i32,
    pub cur_shape: i32,
}

// Event model (kore.c). C-style discriminants: the decode tree ports 1:1.
pub const EV_NONE: i32 = 0;
pub const EV_CHAR: i32 = 1;
pub const EV_KEY: i32 = 2;
pub const EV_MOUSE: i32 = 3;

pub const K_UP: i32 = 1;
pub const K_DOWN: i32 = 2;
pub const K_LEFT: i32 = 3;
pub const K_RIGHT: i32 = 4;
pub const K_ENTER: i32 = 5;
pub const K_ESC: i32 = 6;
pub const K_TAB: i32 = 7;
pub const K_BS: i32 = 8;
pub const K_DEL: i32 = 9;
pub const K_HOME: i32 = 10;
pub const K_END: i32 = 11;
pub const K_PGUP: i32 = 12;
pub const K_PGDN: i32 = 13;
pub const K_NEWLINE: i32 = 14; // Alt+Enter, kitty CSI-u mod>=2, modifyOtherKeys mod>=2
pub const K_RESETALL: i32 = 15; // Ctrl+Shift+R: CSI-u / modifyOtherKeys code 114|82 mod 6

pub const M_PRESS: i32 = 0;
pub const M_RELEASE: i32 = 1;
pub const M_DRAG: i32 = 2;
pub const M_WHEELUP: i32 = 3;
pub const M_WHEELDN: i32 = 4;

// For EV_CHAR, u8b holds the raw input bytes (text insertion splices them verbatim) and
// ch the decoded codepoint (0xFFFD on malformed). mx/my are 0-based cell coords.
#[derive(Clone, Copy)]
pub struct Ev {
    pub etype: i32,
    pub key: i32,
    pub mkind: i32,
    pub mx: i32,
    pub my: i32,
    pub ch: u32,
    pub u8b: [u8; 8],
}

impl Ev {
    pub fn none() -> Ev {
        Ev {
            etype: EV_NONE,
            key: 0,
            mkind: 0,
            mx: 0,
            my: 0,
            ch: 0,
            u8b: [0; 8],
        }
    }
}

impl Term {
    // A zero-size Term; size()/headless() allocate the grid.
    pub fn new() -> Term {
        Term {
            rows: 0,
            cols: 0,
            grid: Vec::new(),
            out: Vec::new(),
            cur_x: 0,
            cur_y: 0,
            cur_shape: 0,
        }
    }

    // The --check render target: an in-memory grid, no tty (C sets T.rows=200, T.cols=400).
    pub fn headless(rows: i32, cols: i32) -> Term {
        let mut t = Term::new();
        t.rows = rows;
        t.cols = cols;
        t.grid = vec![Cell::blank(); (rows * cols) as usize];
        t
    }

    // Raw mode + hello via sys::term_enter, then size(). false when not a terminal.
    pub fn enter(&mut self) -> bool {
        if !sys::term_enter() {
            return false;
        }
        self.size();
        true
    }

    // ioctl winsize (fallback 24x80), floors rows>=12 cols>=20, grid reallocated zeroed
    // (contents discarded; the next frame_clear repaints). kore.c term_size.
    pub fn size(&mut self) {
        let (mut r, mut c) = sys::win_size().unwrap_or((24, 80));
        if r < 12 {
            r = 12;
        }
        if c < 20 {
            c = 20;
        }
        self.rows = r;
        self.cols = c;
        self.grid = vec![cell_zeroed(); (r * c) as usize];
    }

    // Every cell becomes Cell::blank(); cur_shape = 0 (each frame starts cursor-hidden).
    pub fn frame_clear(&mut self) {
        self.cur_shape = 0;
        for c in self.grid.iter_mut() {
            *c = Cell::blank();
        }
    }

    // Write UTF-8 text into the grid at (x,y), clipped to maxw (-1 unbounded) and the grid
    // edge; returns width consumed. Wide glyphs take two cells (second cont); overwriting a
    // continuation blanks its owner, burying a wide head blanks its orphan; codepoints < 0x20
    // are skipped but still consume width; glyph bytes clamp at 7. kore.c putp.
    pub fn putp(&mut self, x: i32, y: i32, attr: u8, fg: u8, bg: u8, s: &[u8], maxw: i32) -> i32 {
        if y < 0 || y >= self.rows {
            return 0;
        }
        let mut w = 0i32;
        let mut i = 0usize;
        while i < s.len() && s[i] != 0 {
            let at = i;
            let c = text::u8next(s, &mut i);
            let gw = text::cw(c);
            if maxw >= 0 && w + gw > maxw {
                break;
            }
            if x + w + gw > self.cols {
                break;
            }
            if x + w >= 0 && c >= 0x20 {
                let idx = (y * self.cols + x + w) as usize;
                // overwriting half of a wide glyph must not shift the row: writing onto a
                // continuation blanks its owner, and burying a wide head blanks its orphan
                if self.grid[idx].cont && x + w > 0 {
                    let own = &mut self.grid[idx - 1];
                    own.g[0] = b' ';
                    own.g[1] = 0;
                }
                let mut bl = i - at;
                if bl > 7 {
                    bl = 7;
                }
                let cl = &mut self.grid[idx];
                cl.g[..bl].copy_from_slice(&s[at..at + bl]);
                cl.g[bl] = 0;
                cl.attr = attr;
                cl.fg = fg;
                cl.bg = bg;
                cl.cont = false;
                if gw == 2 && x + w + 1 < self.cols {
                    let mut c2 = cell_zeroed();
                    c2.cont = true;
                    self.grid[idx + 1] = c2;
                } else if gw == 1 && x + w + 1 < self.cols && self.grid[idx + 1].cont {
                    self.grid[idx + 1] = Cell::blank();
                }
            }
            w += gw;
        }
        w
    }

    // putp with bg 0.
    pub fn put(&mut self, x: i32, y: i32, attr: u8, fg: u8, s: &[u8], maxw: i32) -> i32 {
        self.putp(x, y, attr, fg, 0, s, maxw)
    }

    // Stamp one glyph into every cell of the rect (bg 0, cont cleared), fixing straddled
    // wide glyphs at both edges. kore.c fill.
    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, g: &[u8], attr: u8, fg: u8) {
        // snprintf "%s": stops at NUL, clamps to 7 bytes.
        let gl = g.iter().position(|&b| b == 0).unwrap_or(g.len()).min(7);
        for j in y..y + h {
            if j < 0 || j >= self.rows {
                continue;
            }
            // wide glyphs straddling the region's edges must not shift the row
            if x > 0 && x < self.cols && self.grid[(j * self.cols + x) as usize].cont {
                let own = &mut self.grid[(j * self.cols + x - 1) as usize];
                own.g[0] = b' ';
                own.g[1] = 0;
            }
            if x + w >= 0 && x + w < self.cols && self.grid[(j * self.cols + x + w) as usize].cont {
                self.grid[(j * self.cols + x + w) as usize] = Cell::blank();
            }
            for i in x..x + w {
                if i >= 0 && i < self.cols {
                    let c = &mut self.grid[(j * self.cols + i) as usize];
                    c.g = [0; 8];
                    c.g[..gl].copy_from_slice(&g[..gl]);
                    c.attr = attr;
                    c.fg = fg;
                    c.bg = 0;
                    c.cont = false;
                }
            }
        }
    }

    // OR A_REV into the cell, stepping a continuation back to its wide head; bounds-checked.
    // The one door for cursor overlays. kore.c rev_cell.
    pub fn rev_cell(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 || y >= self.rows || x >= self.cols {
            return;
        }
        let mut idx = (y * self.cols + x) as usize;
        if self.grid[idx].cont && x > 0 {
            idx -= 1;
        }
        self.grid[idx].attr |= A_REV;
    }

    // Border ┌ ┐ └ ┘ ─ │; focused A_BOLD accent, unfocused A_DIM C_FRAME; title embedded in
    // the top rule as ╴title╶ from x+2, maxw w-6. No-op when w<2 or h<2. kore.c box.
    pub fn draw_box(&mut self, r: Rect, title: &str, accent: u8, focused: bool) {
        let (x, y, w, h) = (r.x, r.y, r.w, r.h);
        if w < 2 || h < 2 {
            return;
        }
        let a = if focused { A_BOLD } else { A_DIM };
        let fg = if focused { accent } else { C_FRAME };
        self.put(x, y, a, fg, "┌".as_bytes(), 1);
        self.put(x + w - 1, y, a, fg, "┐".as_bytes(), 1);
        self.put(x, y + h - 1, a, fg, "└".as_bytes(), 1);
        self.put(x + w - 1, y + h - 1, a, fg, "┘".as_bytes(), 1);
        for i in 1..w - 1 {
            self.put(x + i, y, a, fg, "─".as_bytes(), 1);
            self.put(x + i, y + h - 1, a, fg, "─".as_bytes(), 1);
        }
        for j in 1..h - 1 {
            self.put(x, y + j, a, fg, "│".as_bytes(), 1);
            self.put(x + w - 1, y + j, a, fg, "│".as_bytes(), 1);
        }
        if !title.is_empty() {
            self.put(x + 2, y, a | A_BOLD, fg, "╴".as_bytes(), 1);
            let tw = self.put(
                x + 3,
                y,
                if focused { A_BOLD } else { 0 },
                fg,
                title.as_bytes(),
                w - 6,
            );
            self.put(x + 3 + tw, y, a | A_BOLD, fg, "╶".as_bytes(), 1);
        }
    }

    // Right-border thumb, only when total > vis && r.h > 3 && vis > 0: track h-2, thumb
    // vis*track/total min 1, at top*(track-thumb)/maxTop clamped; ┃ accent on-thumb, │ dim
    // C_FRAME off. Integer math exact. kore.c scrollbar.
    pub fn scrollbar(&mut self, r: Rect, top: i32, vis: i32, total: i32, accent: u8) {
        if total <= vis || r.h <= 3 || vis <= 0 {
            return;
        }
        let track = r.h - 2;
        let mut thumb = vis * track / total;
        if thumb < 1 {
            thumb = 1;
        }
        let max_top = total - vis;
        let mut at = if max_top > 0 {
            top * (track - thumb) / max_top
        } else {
            0
        };
        if at > track - thumb {
            at = track - thumb;
        }
        for j in 0..track {
            let on = j >= at && j < at + thumb;
            self.put(
                r.x + r.w - 1,
                r.y + 1 + j,
                if on { 0 } else { A_DIM },
                if on { accent } else { C_FRAME },
                if on { "┃" } else { "│" }.as_bytes(),
                1,
            );
        }
    }

    // Compose the whole frame into self.out and write(1) once: head "\x1b[?25l\x1b[H", rows
    // joined by "\r\n", cont cells skipped, SGR runs coalesced over (attr,fg,bg) seeded
    // invalid — on change emit "\x1b[0;48;5;<bg>" + ";2"? + ";1"? + ";7"? + ";38;5;<fg>" + "m"
    // with bg?:C_BG, fg?:C_TEXT; tail "\x1b[0m"; then when cur_shape != 0:
    // "\x1b[<y+1>;<x+1>H\x1b[<shape> q\x1b[?25h". No diffing. kore.c flush_frame.
    pub fn flush_frame(&mut self) {
        self.out.clear();
        self.out.extend_from_slice(b"\x1b[?25l\x1b[H");
        let mut cur: (i32, i32, i32) = (-1, -1, -1);
        for y in 0..self.rows {
            if y > 0 {
                self.out.extend_from_slice(b"\r\n");
            }
            for x in 0..self.cols {
                let c = self.grid[(y * self.cols + x) as usize];
                if c.cont {
                    continue;
                }
                let key = (c.attr as i32, c.fg as i32, c.bg as i32);
                if key != cur {
                    let bg = if c.bg != 0 { c.bg } else { C_BG };
                    let fg = if c.fg != 0 { c.fg } else { C_TEXT };
                    self.out.extend_from_slice(b"\x1b[0;48;5;");
                    self.out.extend_from_slice(bg.to_string().as_bytes());
                    if c.attr & A_DIM != 0 {
                        self.out.extend_from_slice(b";2");
                    }
                    if c.attr & A_BOLD != 0 {
                        self.out.extend_from_slice(b";1");
                    }
                    if c.attr & A_REV != 0 {
                        self.out.extend_from_slice(b";7");
                    }
                    self.out.extend_from_slice(b";38;5;");
                    self.out.extend_from_slice(fg.to_string().as_bytes());
                    self.out.push(b'm');
                    cur = key;
                }
                self.out.extend_from_slice(c.glyph());
            }
        }
        self.out.extend_from_slice(b"\x1b[0m");
        if self.cur_shape != 0 {
            let park = format!(
                "\x1b[{};{}H\x1b[{} q\x1b[?25h",
                self.cur_y + 1,
                self.cur_x + 1,
                self.cur_shape
            );
            self.out.extend_from_slice(park.as_bytes());
        }
        sys::write_stdout(&self.out);
    }
}

// sscanf "%d[;%d...]" over a byte prefix: each %d skips C whitespace, then an optional sign
// and a digit run — no digits is a matching failure and stops; slots after the first must be
// preceded by a literal ';' matched exactly. Unmatched slots keep 0 (the C's zeroed locals).
fn scan_ints(s: &[u8], out: &mut [i32]) {
    let mut i = 0usize;
    for k in 0..out.len() {
        if k > 0 {
            if i >= s.len() || s[i] != b';' {
                return;
            }
            i += 1;
        }
        while i < s.len() && (s[i] == b' ' || (0x09..=0x0d).contains(&s[i])) {
            i += 1;
        }
        let mut neg = false;
        if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
            neg = s[i] == b'-';
            i += 1;
        }
        let mut v: i64 = 0;
        let mut any = false;
        while i < s.len() && s[i].is_ascii_digit() {
            v = v * 10 + (s[i] - b'0') as i64;
            if v > i32::MAX as i64 {
                v = i32::MAX as i64;
            }
            i += 1;
            any = true;
        }
        if !any {
            return;
        }
        out[k] = if neg { -(v as i32) } else { v as i32 };
    }
}

// atoi over a byte prefix: skip C whitespace, optional sign, digit run; 0 when none.
fn atoi_prefix(s: &[u8]) -> i32 {
    let mut v = [0i32; 1];
    scan_ints(s, &mut v);
    v[0]
}

// The full decode tree (kore.c ev_read): outer sys::rbyte(100), continuation bytes
// sys::rbyte(25). ESC alone -> K_ESC; ESC+\r|\n -> K_NEWLINE; ESC+other-non-[O swallowed.
// CSI accumulates into a 47-byte seq draining to a final in @..~ except '['. SGR mouse
// '<': base = mb & !28, 64/65 wheel, >=66 drop, motion (base&3)==3 drop else drag; 'm'
// release / 'M' press. Finals A B C D H F; 'u' kitty CSI-u (13 -> Enter/Newline by mod>=2,
// 114|82 mod 6 -> K_RESETALL); '~' atoi: 27 -> modifyOtherKeys, 3 DEL 5 PGUP 6 PGDN 1|7
// HOME 4|8 END. Plain: \r \n Enter, \t Tab, 0x7f|0x08 BS, other <0x20 drop, else UTF-8
// char (a stray 0x80..0xBF lead eats one follower, ch = 0xFFFD — keep the quirk).
pub fn ev_read() -> Ev {
    let mut e = Ev::none();
    let b = sys::rbyte(100);
    if b < 0 {
        return e;
    }
    if b == 0x1b {
        let b2 = sys::rbyte(25);
        if b2 < 0 {
            e.etype = EV_KEY;
            e.key = K_ESC;
            return e;
        }
        if b2 == b'\r' as i32 || b2 == b'\n' as i32 {
            // Alt+Enter
            e.etype = EV_KEY;
            e.key = K_NEWLINE;
            return e;
        }
        if b2 != b'[' as i32 && b2 != b'O' as i32 {
            return e; // Alt chord: swallowed whole
        }
        let mut seq = [0u8; 48];
        let mut n = 0usize;
        let mut trunc = false;
        loop {
            let c = sys::rbyte(25);
            if c < 0 {
                return e;
            }
            let cb = c as u8;
            if n < seq.len() - 1 {
                seq[n] = cb;
                n += 1;
            } else {
                trunc = true; // overlong: drain to the final byte
            }
            if cb >= b'@' && cb <= b'~' && cb != b'[' {
                break;
            }
        }
        if trunc {
            return e;
        }
        let fin = seq[n - 1];
        if seq[0] == b'<' {
            // SGR mouse; modifier bits 4/8/16 strip, unknown codes drop
            let mut v = [0i32; 3];
            scan_ints(&seq[1..n], &mut v);
            let (mb, mx, my) = (v[0], v[1], v[2]);
            e.etype = EV_MOUSE;
            e.mx = mx - 1;
            e.my = my - 1;
            let base = mb & !28;
            if base == 64 {
                e.mkind = M_WHEELUP;
            } else if base == 65 {
                e.mkind = M_WHEELDN;
            } else if base >= 66 {
                e.etype = EV_NONE; // horizontal wheel and beyond
            } else if base >= 32 {
                if (base & 3) == 3 {
                    e.etype = EV_NONE; // motion without a button
                } else {
                    e.mkind = M_DRAG;
                }
            } else if (base & 3) == 3 {
                e.etype = EV_NONE;
            } else {
                e.mkind = if fin == b'm' { M_RELEASE } else { M_PRESS };
            }
            return e;
        }
        e.etype = EV_KEY;
        match fin {
            b'A' => e.key = K_UP,
            b'B' => e.key = K_DOWN,
            b'C' => e.key = K_RIGHT,
            b'D' => e.key = K_LEFT,
            b'H' => e.key = K_HOME,
            b'F' => e.key = K_END,
            b'u' => {
                // kitty CSI-u: modified Enter is a newline
                let mut v = [0i32; 2];
                scan_ints(&seq[..n], &mut v);
                let (code, md) = (v[0], v[1]);
                if code == 13 {
                    e.key = if md >= 2 { K_NEWLINE } else { K_ENTER };
                } else if (code == 114 || code == 82) && md == 6 {
                    e.key = K_RESETALL; // ctrl+shift+r
                } else {
                    e.etype = EV_NONE;
                }
            }
            b'~' => {
                let code = atoi_prefix(&seq[..n]); // "15~" is F5, not Home
                let mut handled = false;
                if code == 27 {
                    // xterm modifyOtherKeys: "27;mod;13~"
                    let mut v = [0i32; 3];
                    scan_ints(&seq[..n], &mut v);
                    let (md, key) = (v[1], v[2]);
                    if key == 13 {
                        e.key = if md >= 2 { K_NEWLINE } else { K_ENTER };
                        handled = true;
                    } else if (key == 114 || key == 82) && md == 6 {
                        e.key = K_RESETALL;
                        handled = true;
                    }
                }
                if !handled {
                    e.key = match code {
                        3 => K_DEL,
                        5 => K_PGUP,
                        6 => K_PGDN,
                        1 | 7 => K_HOME,
                        4 | 8 => K_END,
                        _ => 0,
                    };
                    if e.key == 0 {
                        e.etype = EV_NONE;
                    }
                }
            }
            _ => e.etype = EV_NONE,
        }
        return e;
    }
    if b == b'\r' as i32 || b == b'\n' as i32 {
        e.etype = EV_KEY;
        e.key = K_ENTER;
        return e;
    }
    if b == b'\t' as i32 {
        e.etype = EV_KEY;
        e.key = K_TAB;
        return e;
    }
    if b == 0x7f || b == 0x08 {
        e.etype = EV_KEY;
        e.key = K_BS;
        return e;
    }
    if b < 0x20 {
        return e;
    }
    // UTF-8 continuation
    e.etype = EV_CHAR;
    e.u8b[0] = b as u8;
    let need = if b < 0x80 {
        0
    } else if b < 0xE0 {
        1
    } else if b < 0xF0 {
        2
    } else {
        3
    };
    for i in 0..need {
        let c = sys::rbyte(25);
        if c < 0 {
            break;
        }
        e.u8b[1 + i] = c as u8;
    }
    e.u8b[1 + need] = 0;
    let mut i = 0usize;
    e.ch = text::u8next(&e.u8b, &mut i);
    e
}
