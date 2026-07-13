// Panels, layout, key/mouse dispatch, and the code editor. Rendering clamps and follows
// scroll state, so that mutation stays inside draw functions.
// Headless checks call only draw_space and draw_bitmap; both render into the supplied grid.

use crate::app::{App, CodeSnap, Focus, Mode, VRow, KCUNDO};
use crate::sys;
use crate::term::{
    Ev, Rect, Term, A_BOLD, A_DIM, C_BG, C_CHAR, C_CODEC, C_DEF, C_DIRECTIVE, C_ERR, C_FRAME,
    C_GLOW, C_HDR, C_NIHONGO, C_NUMLIT, C_OK, C_OP, C_OUTC, C_OUTPUTSC, C_PROMPTC, C_RAILC,
    C_REL, C_ROWLBL, C_SEARCH, C_SYM, C_WORLDC, C_BOOL, EV_CHAR, EV_KEY, EV_MOUSE, EV_NONE,
    K_BS, K_DOWN, K_END, K_ENTER, K_ESC, K_HOME, K_LEFT, K_NEWLINE, K_PGDN, K_PGUP,
    K_RESETALL, K_RIGHT, K_TAB, K_UP, M_DRAG, M_PRESS, M_RELEASE, M_WHEELDN, M_WHEELUP,
};
use crate::text;
use crate::world::{self, EKind, VType, FIELD_PAL};

// The EV_CHAR raw bytes: the NUL-terminated run of e.u8b (kore.c e.u8 semantics).
fn ev_bytes(e: &Ev) -> &[u8] {
    let n = e.u8b.iter().position(|&b| b == 0).unwrap_or(8);
    &e.u8b[..n]
}

// Byte-substring hit, the strstr of the history tints.
fn contains(hay: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle)
}

// kore.c split lines: \n-separated, the post-final-newline empty tail dropped once a line
// exists; strip_cr mirrors code_load's CRLF strip (the undo-pop splitter keeps \r).
fn split_lines(src: &[u8], strip_cr: bool) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut at = 0usize;
    loop {
        let rel = src[at..].iter().position(|&b| b == b'\n');
        let end = rel.map(|k| at + k).unwrap_or(src.len());
        let mut line = &src[at..end];
        if rel.is_none() && line.is_empty() && !out.is_empty() {
            break;
        }
        if strip_cr && line.last() == Some(&b'\r') {
            line = &line[..line.len() - 1];
        }
        out.push(line.to_vec());
        match rel {
            Some(_) => at = end + 1,
            None => break,
        }
    }
    out
}

// Segment count: 1 (the entity table) + one per E_FIELD. kore.c nsegs.
fn nsegs(app: &App) -> i32 {
    1 + app.world.ents.iter().filter(|e| e.kind == EKind::Field).count() as i32
}

fn hit(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
}

// ---------- layout and the frame ----------

// kore.c layout: railW = MODE_RAIL ? clamp(W/4, 20, 34) : 0; promptH = prompt line
// count + 2 capped at max(H/3, 3); statusH 1; outH 5; outputsH = (H-promptH-statusH-outH)
// * 2/5; codeH = MODE_REG ? 0 : remainder * 2/5; rects per the map.
pub fn layout(app: &mut App, t: &Term) {
    let big_w = t.cols;
    let h = t.rows;
    let rail_w = if app.mode == Mode::Rail {
        if big_w / 4 < 34 { if big_w / 4 > 20 { big_w / 4 } else { 20 } } else { 34 }
    } else {
        0
    };
    let mut plines = 1;
    for &b in &app.prompt {
        if b == b'\n' {
            plines += 1;
        }
    }
    let status_h = 1;
    let mut prompt_h = plines + 2;
    let max_ph = if h / 3 > 3 { h / 3 } else { 3 };
    if prompt_h > max_ph {
        prompt_h = max_ph;
    }
    let out_h = 5;
    let outputs_h = (h - prompt_h - status_h - out_h) * 2 / 5;
    let code_h = if app.mode == Mode::Reg { 0 } else { (h - prompt_h - status_h - out_h - outputs_h) * 2 / 5 };
    app.rail_r = Rect { x: 0, y: 0, w: rail_w, h: h - prompt_h - status_h };
    let x = rail_w;
    let w = big_w - rail_w;
    app.code_r = Rect { x, y: 0, w, h: code_h };
    app.world_r = Rect { x, y: code_h, w, h: h - prompt_h - status_h - out_h - outputs_h - code_h };
    app.outputs_r = Rect { x, y: h - prompt_h - status_h - out_h - outputs_h, w, h: outputs_h };
    app.out_r = Rect { x, y: h - prompt_h - status_h - out_h, w, h: out_h };
    app.prompt_r = Rect { x: 0, y: h - prompt_h - status_h, w: big_w, h: prompt_h };
}

// Frame order (kore.c draw): frame_clear, layout, draw_rail, draw_code, draw_world,
// draw_outputs, draw_out, draw_prompt, draw_status, flush_frame. Runs unconditionally
// every loop pass — no dirty tracking, no diffing.
pub fn draw(app: &mut App, t: &mut Term) {
    t.frame_clear();
    layout(app, t);
    draw_rail(app, t);
    draw_code(app, t);
    draw_world(app, t);
    draw_outputs(app, t);
    draw_out(app, t);
    draw_prompt(app, t);
    draw_status(app, t);
    t.flush_frame();
}

// ---------- the demos rail ----------

// Title `demos %d`, accent C_RAILC; railTop follows railSel; entries strip a leading
// "demos/", directory part dim, filename C_NIHONGO on a "-nihongo" hit; selected row
// pre-filled A_REV. kore.c draw_rail.
pub fn draw_rail(app: &mut App, t: &mut Term) {
    let r = app.rail_r;
    if r.w <= 0 {
        return;
    }
    let n_demos = app.demo_list.len() as i32;
    let title = format!("demos {}", n_demos);
    t.draw_box(r, &title, C_RAILC, app.focus == Focus::Rail);
    let vis = r.h - 2;
    if app.rail_sel < app.rail_top {
        app.rail_top = app.rail_sel;
    }
    if app.rail_sel >= app.rail_top + vis {
        app.rail_top = app.rail_sel - vis + 1;
    }
    let mut i = 0;
    while i < vis && app.rail_top + i < n_demos {
        let di = app.rail_top + i;
        let full: &[u8] = app.demo_list[di as usize].as_bytes();
        let p: &[u8] = if full.starts_with(b"demos/") { &full[6..] } else { full };
        let sel = di == app.rail_sel;
        let y = r.y + 1 + i;
        if sel {
            t.fill(r.x + 1, y, r.w - 2, 1, b" ", crate::term::A_REV, 0);
        }
        // the directory dims, the file carries the color; -nihongo twins tint violet
        let slash = p.iter().rposition(|&b| b == b'/');
        let mut xx = r.x + 2;
        if let Some(sl) = slash {
            let dl = (sl + 1).min(159); // the C's 160-byte dir buffer
            xx += t.put(xx, y, (if sel { crate::term::A_REV } else { 0 }) | A_DIM, 0, &p[..dl], r.w - 3);
        }
        let fname = match slash {
            Some(sl) => &p[sl + 1..],
            None => p,
        };
        let fg = if contains(fname, b"-nihongo") { C_NIHONGO } else { 0 };
        t.put(xx, y, if sel { crate::term::A_REV } else { 0 }, fg, fname, r.w - 2 - (xx - r.x));
        i += 1;
    }
    t.scrollbar(r, app.rail_top, vis, n_demos, C_RAILC);
}

// j/k/arrows move selection, PGUP/PGDN ±10 clamped, Enter opens and focuses F_WORLD.
pub fn rail_key(app: &mut App, e: &Ev) {
    let ch = if e.etype == EV_CHAR { e.ch } else { 0 };
    let key = if e.etype == EV_KEY { e.key } else { 0 };
    let n = app.demo_list.len() as i32;
    if (ch == b'j' as u32 || key == K_DOWN) && app.rail_sel < n - 1 {
        app.rail_sel += 1;
    } else if (ch == b'k' as u32 || key == K_UP) && app.rail_sel > 0 {
        app.rail_sel -= 1;
    } else if key == K_PGDN {
        app.rail_sel += 10;
        if app.rail_sel >= n {
            app.rail_sel = n - 1;
        }
    } else if key == K_PGUP {
        app.rail_sel -= 10;
        if app.rail_sel < 0 {
            app.rail_sel = 0;
        }
    } else if key == K_ENTER && n > 0 {
        let path = app.demo_list[app.rail_sel as usize].clone();
        world::open_demo(app, &path);
        app.focus = Focus::World;
    }
}

// ---------- the code pane ----------

// Split the file into code[] lines (CRLF stripped, post-final-newline tail dropped,
// unreadable -> one empty line); clear undo/chords/search; cursor and scroll to 0.
pub fn code_load(app: &mut App, path: &str) {
    let src = world::read_file(path);
    app.cundo.clear();
    app.code_pending = 0;
    app.code_g = 0;
    app.code_d = 0;
    app.searching = false;
    app.search.clear();
    match src {
        None => {
            // unreadable: an empty buffer, never an empty Vec
            app.code = vec![Vec::new()];
        }
        Some(src) => {
            app.code = split_lines(&src, true);
            if app.code.is_empty() {
                app.code.push(Vec::new());
            }
        }
    }
    app.ccy = 0;
    app.ccx = 0;
    app.code_top = 0;
    app.code_dirty = false;
    app.code_insert = false;
}

// Display column -> byte offset in a UTF-8 line (cw accumulation). kore.c.
pub fn line_byte_at(line: &[u8], col: i32) -> usize {
    let mut w = 0i32;
    let mut i = 0usize;
    while i < line.len() && w < col {
        let mut q = i;
        w += text::cw(text::u8next(line, &mut q));
        i = q;
    }
    i
}

// Byte offset -> display column. kore.c.
pub fn line_col_of(line: &[u8], byte: usize) -> i32 {
    let mut w = 0i32;
    let mut i = 0usize;
    while i < line.len() && i < byte {
        w += text::cw(text::u8next(line, &mut i));
    }
    w
}

fn rune_is_word(c: u32) -> bool {
    c == b'_' as u32 || text::is_letter(c) || text::is_digit(c)
}

// w: leave the current run (word runes `_`/letter/digit vs punct), skip whitespace, land
// on the next head; wraps to the next line's col 0 (or $ of the last line).
pub fn code_word_fwd(app: &mut App) {
    let ncode = app.code.len() as i32;
    let ln = app.code[app.ccy as usize].clone();
    let at = line_byte_at(&ln, app.ccx);
    if at >= ln.len() {
        if app.ccy < ncode - 1 {
            app.ccy += 1;
            app.ccx = 0;
        }
        return;
    }
    let mut p = at;
    let mut q = p;
    let c = text::u8next(&ln, &mut q);
    if !text::is_whitespace(c) {
        let cls = rune_is_word(c);
        p = q; // past the head rune
        while p < ln.len() {
            q = p;
            let c = text::u8next(&ln, &mut q);
            if text::is_whitespace(c) || rune_is_word(c) != cls {
                break;
            }
            p = q;
        }
    }
    while p < ln.len() {
        q = p;
        let c = text::u8next(&ln, &mut q);
        if !text::is_whitespace(c) {
            break;
        }
        p = q;
    }
    if p >= ln.len() {
        if app.ccy < ncode - 1 {
            app.ccy += 1;
            app.ccx = 0;
        } else {
            app.ccx = text::swidth(&ln);
        }
        return;
    }
    app.ccx = line_col_of(&ln, p);
}

// b: the mirror via text::rune_prev, landing on the previous run's head.
pub fn code_word_back(app: &mut App) {
    let ln = app.code[app.ccy as usize].clone();
    let at = line_byte_at(&ln, app.ccx);
    if at == 0 {
        if app.ccy > 0 {
            app.ccy -= 1;
            app.ccx = text::swidth(&app.code[app.ccy as usize]);
        }
        return;
    }
    let mut i = at;
    let mut c = text::rune_prev(&ln, &mut i);
    while i > 0 && text::is_whitespace(c) {
        c = text::rune_prev(&ln, &mut i);
    }
    let cls = rune_is_word(c);
    while i > 0 {
        let mut j = i;
        let d = text::rune_prev(&ln, &mut j);
        if text::is_whitespace(d) || rune_is_word(d) != cls {
            break;
        }
        i = j;
    }
    app.ccx = line_col_of(&ln, i);
}

// Push a whole-buffer snapshot (cap 64, overflow drops the oldest) before every mutation.
pub fn code_undo_push(app: &mut App) {
    if app.cundo.len() == KCUNDO {
        app.cundo.remove(0);
    }
    app.cundo.push(CodeSnap { code: app.code.clone(), ccx: app.ccx, ccy: app.ccy });
}

// Pop: restore buffer and cursor (clamped), set dirty, `code undo (%d left)` /
// `code: nothing to undo`.
pub fn code_undo_pop(app: &mut App) {
    let Some(snap) = app.cundo.pop() else {
        app.say("code: nothing to undo");
        return;
    };
    app.code = snap.code;
    if app.code.is_empty() {
        app.code.push(Vec::new());
    }
    app.ccy = if snap.ccy < app.code.len() as i32 { snap.ccy } else { app.code.len() as i32 - 1 };
    app.ccx = snap.ccx;
    let lw = text::swidth(&app.code[app.ccy as usize]);
    if app.ccx > lw {
        app.ccx = lw;
    }
    app.code_dirty = true;
    let msg = format!("code undo ({} left)", app.cundo.len());
    app.say(&msg);
}

// / typing: ESC clears and closes; Enter closes and jumps forward when non-empty; BS
// deletes one rune; chars append (cap 127 bytes). kore.c search_key.
pub fn search_key(app: &mut App, e: &Ev) {
    if e.etype == EV_KEY && e.key == K_ESC {
        app.searching = false;
        app.search.clear();
        return;
    }
    if e.etype == EV_KEY && e.key == K_ENTER {
        app.searching = false;
        if !app.search.is_empty() {
            code_search_jump(app, 1);
        }
        return;
    }
    if e.etype == EV_KEY && e.key == K_BS {
        if !app.search.is_empty() {
            let mut l = app.search.len() - 1;
            while l > 0 && (app.search[l] & 0xC0) == 0x80 {
                l -= 1;
            }
            app.search.truncate(l);
        }
        return;
    }
    if e.etype == EV_CHAR {
        let b = ev_bytes(e);
        if app.search.len() + b.len() < 127 {
            app.search.extend_from_slice(b);
        }
    }
}

// Cyclic scan from the cursor via text::find_base (base-letter matching); forward starts
// one byte past the cursor on the home line, backward keeps the last match strictly before
// it. Verdicts `/%s` / `?%s` / `no match: %s` / `no search — / sets one`.
pub fn code_search_jump(app: &mut App, dir: i32) {
    if app.search.is_empty() {
        app.say("no search — / sets one");
        return;
    }
    let needle = app.search.clone();
    let total = app.code.len() as i32;
    let mut found: Option<(i32, usize)> = None;
    for step in 0..=total {
        let li = ((app.ccy + dir * step) % total + total) % total;
        let ln = &app.code[li as usize];
        let ll = ln.len();
        if dir > 0 {
            let mut from = 0usize;
            if step == 0 {
                from = line_byte_at(ln, app.ccx) + 1;
                if from > ll {
                    continue;
                }
            }
            let at = text::find_base(ln, &needle, from);
            if at != text::NPOS {
                found = Some((li, at));
                break;
            }
        } else {
            let limit = if step == 0 { line_byte_at(ln, app.ccx) } else { ll + 1 };
            let mut best = text::NPOS;
            let mut at = 0usize;
            loop {
                let f = text::find_base(ln, &needle, at);
                if f == text::NPOS || f >= limit {
                    break;
                }
                best = f;
                at = f + 1;
            }
            if best != text::NPOS {
                found = Some((li, best));
                break;
            }
        }
    }
    match found {
        Some((li, at)) => {
            let col = line_col_of(&app.code[li as usize], at);
            app.ccy = li;
            app.ccx = col;
            let msg = format!("{}{}", if dir > 0 { "/" } else { "?" }, String::from_utf8_lossy(&needle));
            app.say(&msg);
        }
        None => {
            let msg = format!("no match: {}", String::from_utf8_lossy(&needle));
            app.sayerr(&msg);
        }
    }
}

// Insert u8 at the cursor's byte position; the cursor advances by the head rune's width.
fn code_insert_str(app: &mut App, u8b: &[u8]) {
    let at = line_byte_at(&app.code[app.ccy as usize], app.ccx);
    app.code[app.ccy as usize].splice(at..at, u8b.iter().copied());
    let mut q = 0usize;
    app.ccx += text::cw(text::u8next(u8b, &mut q));
    app.code_dirty = true;
}

// The vim table (kore.c code_key): insert mode (ESC leaves, Tab = two spaces,
// Enter splits, BS deletes/joins, chars insert; unmatched EV_KEY falls through to the
// browse arrows); browse counts 1-9 (0 while pending), hjkl/0/^/$/w/b motions, gg/G with
// counts, i/a/o/x/dd with undo pushes, u pop, n/N when a pattern is set, / search, s save;
// EV_KEY arrows/home/end/pgup/pgdn/Enter-into-insert. Matched vim chars return without the
// trailing ccx clamp, exactly as the C's early returns do.
pub fn code_key(app: &mut App, e: &Ev) {
    let lw = text::swidth(&app.code[app.ccy as usize]);
    if app.code_insert {
        if e.etype == EV_KEY && e.key == K_ESC {
            app.code_insert = false;
            return;
        }
        // tab is text here — two spaces, the corpus indents with spaces, never \t
        if e.etype == EV_KEY && e.key == K_TAB {
            code_insert_str(app, b" ");
            code_insert_str(app, b" ");
            return;
        }
        if e.etype == EV_KEY && e.key == K_ENTER {
            let at = line_byte_at(&app.code[app.ccy as usize], app.ccx);
            let rest = app.code[app.ccy as usize][at..].to_vec();
            app.code[app.ccy as usize].truncate(at);
            app.code.insert(app.ccy as usize + 1, rest);
            app.ccy += 1;
            app.ccx = 0;
            app.code_dirty = true;
            return;
        }
        if e.etype == EV_KEY && e.key == K_BS {
            let at = line_byte_at(&app.code[app.ccy as usize], app.ccx);
            if at > 0 {
                let ln = &app.code[app.ccy as usize];
                let mut prev = at - 1;
                while prev > 0 && (ln[prev] & 0xC0) == 0x80 {
                    prev -= 1;
                }
                let mut q = 0usize;
                let width = text::cw(text::u8next(&ln[prev..at], &mut q));
                app.ccx -= width;
                app.code[app.ccy as usize].drain(prev..at);
                app.code_dirty = true;
            } else if app.ccy > 0 {
                let cur = app.code.remove(app.ccy as usize);
                app.ccy -= 1;
                app.ccx = text::swidth(&app.code[app.ccy as usize]);
                app.code[app.ccy as usize].extend_from_slice(&cur);
                app.code_dirty = true;
            }
            return;
        }
        if e.etype == EV_CHAR {
            let b: Vec<u8> = ev_bytes(e).to_vec();
            code_insert_str(app, &b);
            return;
        }
    }
    // browse: vim vocabulary — counts, word motions, gg/G, /-search, code-local undo
    if e.etype == EV_CHAR {
        let ch = e.ch;
        if (ch >= b'1' as u32 && ch <= b'9' as u32) || (app.code_pending != 0 && ch == b'0' as u32) {
            app.code_pending = app.code_pending * 10 + (ch - b'0' as u32) as i32;
            if app.code_pending > 999999 {
                app.code_pending = 999999;
            }
            return;
        }
        let had_count = app.code_pending != 0;
        let mut rep = if had_count { app.code_pending } else { 1 };
        app.code_pending = 0;
        if ch != b'g' as u32 {
            app.code_g = 0;
        }
        if ch != b'd' as u32 {
            app.code_d = 0;
        }
        let ncode = app.code.len() as i32;
        if ch == b'j' as u32 {
            while rep > 0 && app.ccy < ncode - 1 {
                rep -= 1;
                app.ccy += 1;
            }
            return;
        } else if ch == b'k' as u32 {
            while rep > 0 && app.ccy > 0 {
                rep -= 1;
                app.ccy -= 1;
            }
            return;
        } else if ch == b'h' as u32 {
            app.ccx = if app.ccx > rep { app.ccx - rep } else { 0 };
            return;
        } else if ch == b'l' as u32 {
            app.ccx = if app.ccx + rep < lw { app.ccx + rep } else { lw };
            return;
        } else if ch == b'0' as u32 {
            app.ccx = 0;
            return;
        } else if ch == b'^' as u32 {
            let ln = &app.code[app.ccy as usize];
            let mut col = 0;
            for &b in ln.iter() {
                if b == b' ' || b == b'\t' {
                    col += 1;
                } else {
                    break;
                }
            }
            app.ccx = col;
            return;
        } else if ch == b'$' as u32 {
            app.ccx = lw;
            return;
        } else if ch == b'w' as u32 {
            while rep > 0 {
                rep -= 1;
                code_word_fwd(app);
            }
            return;
        } else if ch == b'b' as u32 {
            while rep > 0 {
                rep -= 1;
                code_word_back(app);
            }
            return;
        } else if ch == b'g' as u32 {
            // gg — [count]gg goes to that line, count stored on the FIRST g
            if app.code_g != 0 {
                let tgt = if app.code_g > 1 { app.code_g } else { 1 };
                app.ccy = if tgt <= ncode { tgt - 1 } else { ncode - 1 };
                app.ccx = 0;
                app.code_g = 0;
            } else {
                app.code_g = rep;
            }
            return;
        } else if ch == b'G' as u32 {
            // [count]G goes to that line, bare G to the last
            app.ccy = if had_count { if rep <= ncode { rep - 1 } else { ncode - 1 } } else { ncode - 1 };
            app.ccx = 0;
            return;
        } else if ch == b'i' as u32 {
            code_undo_push(app);
            app.code_insert = true;
            return;
        } else if ch == b'a' as u32 {
            code_undo_push(app);
            app.code_insert = true;
            app.ccx = if app.ccx < lw { app.ccx + 1 } else { lw };
            return;
        } else if ch == b'o' as u32 {
            code_undo_push(app);
            app.code.insert(app.ccy as usize + 1, Vec::new());
            app.ccy += 1;
            app.ccx = 0;
            app.code_insert = true;
            app.code_dirty = true;
            return;
        } else if ch == b'x' as u32 {
            code_undo_push(app);
            while rep > 0 {
                rep -= 1;
                let at = line_byte_at(&app.code[app.ccy as usize], app.ccx);
                let ln = &mut app.code[app.ccy as usize];
                if at >= ln.len() {
                    break;
                }
                let mut next = at + 1;
                while next < ln.len() && (ln[next] & 0xC0) == 0x80 {
                    next += 1;
                }
                ln.drain(at..next);
                app.code_dirty = true;
            }
            return;
        } else if ch == b'd' as u32 {
            // dd — [count]dd deletes that many lines, count stored on the FIRST d
            if app.code_d != 0 {
                let mut cnt = app.code_d;
                app.code_d = 0;
                code_undo_push(app);
                while cnt > 0 {
                    cnt -= 1;
                    if app.code.len() > 1 {
                        app.code.remove(app.ccy as usize);
                        if app.ccy >= app.code.len() as i32 {
                            app.ccy = app.code.len() as i32 - 1;
                        }
                    } else {
                        app.code[0].clear();
                        break;
                    }
                }
                app.code_dirty = true;
            } else {
                app.code_d = rep;
            }
            return;
        } else if ch == b'u' as u32 {
            code_undo_pop(app);
            return;
        } else if ch == b'n' as u32 {
            if !app.search.is_empty() {
                code_search_jump(app, 1);
            }
            return;
        } else if ch == b'N' as u32 {
            if !app.search.is_empty() {
                code_search_jump(app, -1);
            }
            return;
        } else if ch == b'/' as u32 {
            app.searching = true;
            app.search.clear();
            return;
        } else if ch == b's' as u32 {
            code_save(app);
            return;
        }
    }
    if e.etype == EV_KEY {
        let page = if app.code_r.h > 4 { app.code_r.h - 3 } else { 10 };
        let ncode = app.code.len() as i32;
        match e.key {
            K_UP => app.ccy = if app.ccy > 0 { app.ccy - 1 } else { 0 },
            K_DOWN => {
                if app.ccy < ncode - 1 {
                    app.ccy += 1;
                }
            }
            K_LEFT => app.ccx = if app.ccx > 0 { app.ccx - 1 } else { 0 },
            K_RIGHT => app.ccx = if app.ccx < lw { app.ccx + 1 } else { lw },
            K_HOME => app.ccx = 0,
            K_END => app.ccx = lw,
            K_PGUP => app.ccy = if app.ccy > page { app.ccy - page } else { 0 },
            K_PGDN => app.ccy = if app.ccy + page < ncode { app.ccy + page } else { ncode - 1 },
            K_ENTER => {
                code_undo_push(app);
                app.code_insert = true;
            }
            _ => {}
        }
    }
    let nlw = text::swidth(&app.code[app.ccy as usize]);
    if app.ccx > nlw {
        app.ccx = nlw;
    }
}

// s: no demo no-ops; world::code_guard first (play copy), then all lines \n-joined via
// world::write_commit; `saved %s` / `cannot write %s`. kore.c code_save.
pub fn code_save(app: &mut App) {
    if app.demo_path.is_empty() {
        return;
    }
    if !world::code_guard(app) {
        return;
    }
    let mut b: Vec<u8> = Vec::new();
    for l in &app.code {
        b.extend_from_slice(l);
        b.push(b'\n');
    }
    if !world::write_commit(&app.demo_live, &b) {
        let msg = format!("cannot write {}", app.demo_live);
        app.sayerr(&msg);
    } else {
        app.code_dirty = false;
        let msg = format!("saved {}", app.demo_live);
        app.say(&msg);
    }
}

// Title `code · %s%s%s · %d/%d` (+` · play copy`, +` +` dirty), INSERT/BROWSE chip;
// per-line syntax colors directives, comments, hinges, defs, operators, and digits. Search
// highlights use rev_cell; insert mode uses DECSCUSR 5. kore.c draw_code.
pub fn draw_code(app: &mut App, t: &mut Term) {
    let r = app.code_r;
    if r.h <= 1 {
        return;
    }
    let title = format!(
        "code · {}{}{} · {}/{}",
        if app.demo_path.is_empty() { "—" } else { app.demo_path.as_str() },
        if !app.demo_path.is_empty() && app.demo_live != app.demo_path { " · play copy" } else { "" },
        if app.code_dirty { " +" } else { "" },
        app.ccy + 1,
        app.code.len()
    );
    t.draw_box(r, &title, C_CODEC, app.focus == Focus::Code);
    // the mode chip: loud in the pane's own title rule, not the status-line corner
    if app.focus == Focus::Code {
        let chip: &[u8] = if app.code_insert { b" INSERT " } else { b" BROWSE " };
        let chw = text::swidth(chip);
        if r.w > chw + 4 {
            t.put(
                r.x + r.w - 2 - chw,
                r.y,
                if app.code_insert { crate::term::A_REV | A_BOLD } else { A_DIM },
                if app.code_insert { C_GLOW } else { 0 },
                chip,
                chw,
            );
        }
    }
    let vis = r.h - 2;
    if app.ccy < app.code_top {
        app.code_top = app.ccy;
    }
    if app.ccy >= app.code_top + vis {
        app.code_top = app.ccy - vis + 1;
    }
    let mut i = 0;
    while i < vis && app.code_top + i < app.code.len() as i32 {
        let li = app.code_top + i;
        let ln = &app.code[li as usize];
        let lt = ln.iter().position(|&b| b != b' ' && b != b'\t').unwrap_or(ln.len());
        let dirline = ln[lt..].starts_with(b"--!"); // directives in their own hue
        let dim = !dirline && ln[lt..].starts_with(b"--"); // comments dim
        let mut def_off = -1i32;
        let mut def_end = -1i32;
        if !dim && !dirline {
            if ln[lt..].starts_with(b"def ") {
                def_off = lt as i32;
                def_end = def_off + 3;
            } else if ln[lt..].starts_with("定義 ".as_bytes()) {
                def_off = lt as i32;
                def_end = def_off + 6;
            }
        }
        let x = r.x + 1;
        let y = r.y + 1 + i;
        let mut p = 0usize;
        let mut col = 0i32;
        let maxw = r.w - 2;
        while p < ln.len() && col < maxw {
            let at = p;
            let c = text::u8next(ln, &mut p);
            let mut attr = 0u8;
            let mut fg = 0u8;
            if dirline {
                fg = C_DIRECTIVE;
            } else if dim {
                attr = A_DIM;
            } else if c == b',' as u32
                || (c == b'=' as u32 && p < ln.len() && ln[p] == b'>')
                || (c == b'>' as u32 && at > 0 && ln[at - 1] == b'=')
            {
                attr = A_BOLD;
                fg = C_GLOW;
            } else if (at as i32) >= def_off && (at as i32) < def_end {
                attr = A_BOLD;
                fg = C_DEF;
            } else if c == b'&' as u32 || c == b'|' as u32 || c == b'<' as u32 || c == b'>' as u32
                || c == b'=' as u32 || c == b'~' as u32 || c == b'!' as u32 || c == b'+' as u32
                || c == b'*' as u32 || c == b'/' as u32 || c == b'%' as u32
            {
                fg = C_OP;
            } else if c >= b'0' as u32 && c <= b'9' as u32 {
                fg = C_NUMLIT;
            }
            let gl = (p - at).min(7);
            col += t.put(x + col, y, attr, fg, &ln[at..at + gl], maxw - col);
        }
        // every /-match on a visible line lights up (span approximated by the needle)
        if !app.search.is_empty() {
            let ndw = text::swidth(&app.search);
            let mut from = 0usize;
            loop {
                let f = text::find_base(ln, &app.search, from);
                if f == text::NPOS {
                    break;
                }
                let c0 = line_col_of(ln, f);
                let mut cc = c0;
                while cc < c0 + ndw && x + cc < r.x + r.w - 1 {
                    t.rev_cell(x + cc, y);
                    cc += 1;
                }
                from = f + 1;
            }
        }
        if app.focus == Focus::Code && li == app.ccy {
            let cx = x + app.ccx;
            if cx < r.x + r.w - 1 {
                // insert gets the terminal's own bar cursor (DECSCUSR 5); browse the block reverse
                if app.code_insert {
                    t.cur_x = cx;
                    t.cur_y = y;
                    t.cur_shape = 5;
                } else {
                    t.rev_cell(cx, y);
                }
            }
        }
        i += 1;
    }
    if app.searching || !app.search.is_empty() {
        let mut sb: Vec<u8> = Vec::with_capacity(app.search.len() + 4);
        sb.push(b'/');
        sb.extend_from_slice(&app.search);
        if app.searching {
            sb.extend_from_slice("▏".as_bytes());
        }
        t.put(r.x + 2, r.y + r.h - 1, A_BOLD, C_SEARCH, &sb, r.w - 4);
    }
    t.scrollbar(r, app.code_top, vis, app.code.len() as i32, C_CODEC);
}

// ---------- the world table ----------

// Flatten the world surface into app.vrows (cap 32768, guard 32760): n table rows, then
// per E_FIELD a blank row, a title row, latH?latH:1 field rows. kore.c world_vrows.
pub fn world_vrows(app: &mut App) {
    app.vrows.clear();
    let mut r = 0;
    while r < app.world.n && (app.vrows.len() as i32) < 32760 {
        app.vrows.push(VRow { kind: 0, seg: 0, row: r });
        r += 1;
    }
    let gh = if app.world.lat_h != 0 { app.world.lat_h } else { 1 };
    let mut seg = 1;
    for i in 0..app.world.ents.len() {
        if app.world.ents[i].kind != EKind::Field {
            continue;
        }
        if app.vrows.len() as i32 + gh + 2 >= 32760 {
            break;
        }
        app.vrows.push(VRow { kind: 3, seg, row: 0 });
        app.vrows.push(VRow { kind: 1, seg, row: 0 });
        for gy in 0..gh {
            app.vrows.push(VRow { kind: 2, seg, row: gy });
        }
        seg += 1;
    }
}

// The vrow index holding the cursor. kore.c cursor_vrow.
pub fn cursor_vrow(app: &App) -> i32 {
    for (i, vr) in app.vrows.iter().enumerate() {
        if vr.seg == app.w_seg && vr.row == app.w_row && (vr.kind == 0 || vr.kind == 2) {
            return i as i32;
        }
    }
    0
}

// Field cell geometry: char fields cellW 1 pad 0; num/bool cellW = max fmt_num width,
// pad 1. kore.c field_cellw.
pub fn field_cellw(app: &App, ent: usize) -> (i32, i32) {
    let e = &app.world.ents[ent];
    let mut cell_w = 1i32;
    let pad = if e.vtype == VType::Char { 0 } else { 1 };
    if e.vtype != VType::Char {
        for &v in &e.nums {
            let l = world::fmt_num(v).len() as i32;
            if l > cell_w {
                cell_w = l;
            }
        }
    }
    (cell_w, pad)
}

// The world panel: title `%s · %s%s · n %d` (bitmap/space/world · path · (play)/(pristine)
// · step · lattice), the pinned header row, presence-dimmed cells, per-kind value hues,
// cursor/drag reverse, the editing cell as `%s▏` A_REV|A_BOLD fg 93, field title/data rows,
// scrollbar over nvrows. Routes to draw_space/draw_bitmap by app.space_view.
pub fn draw_world(app: &mut App, t: &mut Term) {
    let r = app.world_r;
    if r.h <= 1 {
        return;
    }
    let mut title = format!(
        "{} · {}{} · n {}",
        if app.space_view == 2 { "bitmap" } else if app.space_view != 0 { "space" } else { "world" },
        if app.world.loaded { app.world.path.as_str() } else { "—" },
        if app.world_is_copy {
            " (play)"
        } else if app.world.loaded && app.mode != Mode::Reg {
            " (pristine)"
        } else {
            ""
        },
        app.world.n
    );
    if app.world_is_copy && app.undo_seq > 0 {
        title.push_str(&format!(" · step {}", app.undo_seq));
    }
    if app.world.lat_w != 0 {
        title.push_str(&format!(" · {}×{}", app.world.lat_w, app.world.lat_h));
    }
    t.draw_box(r, &title, C_WORLDC, app.focus == Focus::World);
    if !app.world.loaded {
        t.put(r.x + 2, r.y + 1, A_DIM, 0, "no world — pick a demo or open a .reg".as_bytes(), r.w - 4);
        return;
    }
    if app.space_view == 2 {
        draw_bitmap(app, t);
        return;
    }
    if app.space_view != 0 {
        draw_space(app, t);
        return;
    }
    world::table_cols(app);
    world_vrows(app);
    let ndcols = app.dcols.len() as i32;
    if app.w_seg == 0 && app.w_col >= ndcols {
        app.w_col = if ndcols != 0 { ndcols - 1 } else { 0 };
    }
    let x = r.x + 1;
    let y = r.y + 1;
    let row_lbl_w = 4;
    // the table header pins above the scroll
    t.put(x, y, A_DIM, C_ROWLBL, b"row", row_lbl_w);
    let mut cx = x + row_lbl_w + 1;
    for c in 0..app.dcols.len() {
        if cx >= r.x + r.w - 1 {
            break;
        }
        let e = &app.world.ents[app.dcols[c].ent];
        t.put(cx, y, A_BOLD | if e.kind == EKind::Pres { A_DIM } else { 0 }, C_HDR, &e.name, app.dcols[c].width);
        cx += app.dcols[c].width + 1;
    }
    let mut vis = r.h - 3;
    if vis < 1 {
        vis = 1;
    }
    let cv = cursor_vrow(app);
    if cv < app.w_top {
        app.w_top = cv;
    }
    if cv >= app.w_top + vis {
        app.w_top = cv - vis + 1;
    }
    let nvrows = app.vrows.len() as i32;
    if app.w_top > nvrows - vis {
        app.w_top = nvrows - vis;
    }
    if app.w_top < 0 {
        app.w_top = 0;
    }
    let mut d = 0;
    while d < vis && app.w_top + d < nvrows {
        let vr = app.vrows[(app.w_top + d) as usize];
        let yy = y + 1 + d;
        d += 1;
        if vr.kind == 3 {
            continue;
        }
        if vr.kind == 1 {
            if let Some(fi) = world::seg_field(app, vr.seg) {
                t.put(x, yy, A_BOLD, C_HDR, &app.world.ents[fi].name, r.w - 2);
            }
            continue;
        }
        if vr.kind == 0 {
            let row = vr.row;
            let lbl = format!("{}", row);
            t.put(x, yy, A_DIM, C_ROWLBL, lbl.as_bytes(), row_lbl_w);
            cx = x + row_lbl_w + 1;
            for c in 0..app.dcols.len() {
                if cx >= r.x + r.w - 1 {
                    break;
                }
                let ei = app.dcols[c].ent;
                let width = app.dcols[c].width;
                let mut cell = world::table_cell(app, row, c as i32);
                let e = &app.world.ents[ei];
                let mut dim = false;
                // presence gaps visibly absent: a value under pres 0 dims to ·
                if e.kind == EKind::Col {
                    if let Some(pi) = app.world.ent(&e.name, EKind::Pres) {
                        let pe = &app.world.ents[pi];
                        if (row as usize) < pe.nums.len() && pe.nums[row as usize] == 0.0 {
                            cell = "·".as_bytes().to_vec();
                            dim = true;
                        }
                    }
                }
                if e.kind == EKind::Rel && cell.first() == Some(&b'/') {
                    dim = true;
                }
                let cur = app.w_seg == 0 && app.focus == Focus::World && row == app.w_row && c as i32 == app.w_col;
                let in_drag = app.dragging
                    && app.drag_seg == 0
                    && row >= app.drag_r0.min(app.drag_r1)
                    && row <= app.drag_r0.max(app.drag_r1);
                if cur && app.editing {
                    let mut eb = app.edit_buf.clone();
                    eb.extend_from_slice("▏".as_bytes());
                    t.put(cx, yy, crate::term::A_REV | A_BOLD, 93, &eb, width);
                } else {
                    // value hue by kind: sym lilac, rel salmon, bool teal, char warm, vec pale
                    let cfg = if e.vtype == VType::Sym {
                        C_SYM
                    } else if e.vtype == VType::Char {
                        C_CHAR
                    } else if e.kind == EKind::Rel {
                        C_REL
                    } else if e.kind == EKind::Alias {
                        C_DIRECTIVE
                    } else if e.kind == EKind::Pres {
                        0
                    } else if e.vtype == VType::Bool {
                        C_BOOL
                    } else if e.vtype == VType::Vec {
                        C_NUMLIT
                    } else {
                        0
                    };
                    t.put(
                        cx,
                        yy,
                        (if cur || in_drag { crate::term::A_REV } else { 0 }) | (if dim { A_DIM } else { 0 }),
                        if dim { 0 } else { cfg },
                        &cell,
                        width,
                    );
                }
                cx += width + 1;
            }
            continue;
        }
        // field row: the lattice field in its w×h shape
        let Some(fi) = world::seg_field(app, vr.seg) else { continue };
        let e = &app.world.ents[fi];
        let gw = if app.world.lat_w != 0 { app.world.lat_w } else { e.nums.len() as i32 };
        let gy = vr.row;
        let (cell_w, pad) = field_cellw(app, fi);
        let e = &app.world.ents[fi];
        for gx in 0..gw {
            let k = gy * gw + gx;
            let cell: Vec<u8> = if e.vtype == VType::Char {
                world::glyph_at(e, k)
            } else if (k as usize) < e.nums.len() {
                format!("{:>width$}", world::fmt_num(e.nums[k as usize]), width = cell_w as usize).into_bytes()
            } else {
                Vec::new()
            };
            let px = x + gx * (cell_w + pad);
            if px + cell_w >= r.x + r.w - 1 {
                break;
            }
            let cur = app.focus == Focus::World && app.w_seg == vr.seg && app.w_row == gy && app.w_col == gx;
            if cur && app.editing {
                let mut eb = app.edit_buf.clone();
                eb.extend_from_slice("▏".as_bytes());
                t.put(px, yy, crate::term::A_REV | A_BOLD, 93, &eb, cell_w + 1);
            } else {
                let zero = e.vtype == VType::Bool && (k as usize) < e.nums.len() && e.nums[k as usize] == 0.0;
                t.put(
                    px,
                    yy,
                    if cur { crate::term::A_REV } else if zero { A_DIM } else { 0 },
                    if e.vtype == VType::Char { C_CHAR } else if e.vtype == VType::Bool { C_BOOL } else { 0 },
                    &cell,
                    cell_w + 1,
                );
            }
        }
    }
    t.scrollbar(r, app.w_top, vis, nvrows, C_WORLDC);
}

// Browse keys: hjkl/arrows clamp within the segment; j at a table segment's last row steps
// into the next segment, k at row 0 steps back (table mode only); PGUP/PGDN ±10; Enter
// opens the edit (space views route through the entity/field pick). kore.c world_key.
pub fn world_key(app: &mut App, e: &Ev) {
    if app.editing {
        edit_key(app, e);
        return;
    }
    let mut sgw = 1;
    let mut sgh = 1;
    if app.space_view != 0 {
        let (a, b) = space_dims(app);
        sgw = if a < 1 { 1 } else { a };
        sgh = if b < 1 { 1 } else { b };
    }
    let rows = if app.space_view != 0 { sgh } else { world::seg_rows(app, app.w_seg) };
    let cols = if app.space_view != 0 { sgw } else { world::seg_cols(app, app.w_seg) };
    let ch = if e.etype == EV_CHAR { e.ch } else { 0 };
    let key = if e.etype == EV_KEY { e.key } else { 0 };
    if ch == b'j' as u32 || key == K_DOWN {
        if app.w_row < rows - 1 {
            app.w_row += 1;
        } else if app.space_view == 0 && app.w_seg < nsegs(app) - 1 {
            app.w_seg += 1;
            app.w_row = 0;
            app.w_col = 0;
        }
    } else if ch == b'k' as u32 || key == K_UP {
        if app.w_row > 0 {
            app.w_row -= 1;
        } else if app.space_view == 0 && app.w_seg > 0 {
            app.w_seg -= 1;
            app.w_row = world::seg_rows(app, app.w_seg) - 1;
            if app.w_row < 0 {
                app.w_row = 0;
            }
            app.w_col = 0;
        }
    } else if ch == b'h' as u32 || key == K_LEFT {
        if app.w_col > 0 {
            app.w_col -= 1;
        }
    } else if ch == b'l' as u32 || key == K_RIGHT {
        if app.w_col < cols - 1 {
            app.w_col += 1;
        }
    } else if key == K_PGUP {
        app.w_row -= 10;
        if app.w_row < 0 {
            app.w_row = 0;
        }
    } else if key == K_PGDN {
        app.w_row += 10;
        if app.w_row >= rows {
            app.w_row = if rows != 0 { rows - 1 } else { 0 };
        }
    } else if key == K_ENTER {
        if !app.world.loaded {
            return;
        }
        if app.space_view != 0 {
            // map/bitmap edit: whatever painted this cell — the glyph (or pixel) you see
            // is the value you edit. The topmost positioned entity on the cell wins.
            let pos = app.world.pos();
            let mut ent_row: i32 = -1;
            if let Some(pi) = pos {
                let pe = &app.world.ents[pi];
                let mut i = 0usize;
                while i + 1 < pe.nums.len() {
                    if pe.nums[i] == app.w_col as f64 && pe.nums[i + 1] == app.w_row as f64 {
                        ent_row = (i / 2) as i32;
                    }
                    i += 2;
                }
            }
            if ent_row >= 0 {
                world::table_cols(app);
                let mut src = app.world.glyph_col_ent();
                let sym_set = |si: Option<usize>| -> bool {
                    match si {
                        Some(si) => {
                            let se = &app.world.ents[si];
                            (ent_row as usize) < se.syms.len() && !se.syms[ent_row as usize].is_empty()
                        }
                        None => false,
                    }
                };
                if !sym_set(src) {
                    let pr = app.world.proto_col_ent();
                    src = if sym_set(pr) { pr } else { pos };
                }
                let mut c = 0i32;
                if let Some(si) = src {
                    for (j, dc) in app.dcols.iter().enumerate() {
                        if dc.ent == si {
                            c = j as i32;
                            break;
                        }
                    }
                }
                app.spc_ret = 1;
                app.spc_row = app.w_row;
                app.spc_col = app.w_col;
                app.w_seg = 0;
                app.w_row = ent_row;
                app.w_col = c;
            } else {
                let k = app.w_row * (if app.world.lat_w != 0 { app.world.lat_w } else { 1 }) + app.w_col;
                let mut s = 1i32;
                let mut pick = 0i32;
                let mut first = 0i32;
                for fe in &app.world.ents {
                    if fe.kind != EKind::Field {
                        continue;
                    }
                    if !world::names_eq(&fe.name, b"x") && !world::names_eq(&fe.name, b"y") {
                        if first == 0 {
                            first = s;
                        }
                        let lit = if fe.vtype == VType::Char {
                            (k as usize) < fe.chars.len() && fe.chars[k as usize] != b'.'
                        } else {
                            (k as usize) < fe.nums.len() && fe.nums[k as usize] != 0.0
                        };
                        if lit {
                            pick = s;
                        }
                    }
                    s += 1;
                }
                app.w_seg = if pick != 0 { pick } else { first };
                if app.w_seg == 0 {
                    app.say("no editable field on the map");
                    return;
                }
            }
        }
        let cur: Vec<u8> = if app.w_seg == 0 {
            world::table_cols(app);
            world::table_cell(app, app.w_row, app.w_col)
        } else {
            let k = app.w_row * (if app.world.lat_w != 0 { app.world.lat_w } else { 1 }) + app.w_col;
            match world::seg_field(app, app.w_seg) {
                Some(fi) => {
                    let fe = &app.world.ents[fi];
                    if fe.vtype == VType::Char {
                        if (k as usize) < fe.chars.len() { world::glyph_at(fe, k) } else { b".".to_vec() }
                    } else if (k as usize) < fe.nums.len() {
                        world::fmt_num(fe.nums[k as usize]).into_bytes()
                    } else {
                        Vec::new()
                    }
                }
                None => Vec::new(),
            }
        };
        app.edit_buf = cur;
        app.edit_buf.truncate(511);
        app.editing = true;
        app.say("editing — enter commits, esc cancels");
    }
}

// The inline edit: ESC cancels (`edit cancelled`), Enter commits via cell_edit_commit,
// BS deletes one rune, chars append (cap 511 bytes). kore.c edit_key.
pub fn edit_key(app: &mut App, e: &Ev) {
    if e.etype == EV_KEY && e.key == K_ESC {
        app.editing = false;
        spc_return(app);
        app.say("edit cancelled");
        return;
    }
    if e.etype == EV_KEY && e.key == K_ENTER {
        cell_edit_commit(app);
        return;
    }
    if e.etype == EV_KEY && e.key == K_BS {
        if !app.edit_buf.is_empty() {
            let mut l = app.edit_buf.len() - 1;
            while l > 0 && (app.edit_buf[l] & 0xC0) == 0x80 {
                l -= 1;
            }
            app.edit_buf.truncate(l);
        }
        return;
    }
    if e.etype == EV_CHAR {
        let b = ev_bytes(e);
        if app.edit_buf.len() + b.len() < 511 {
            app.edit_buf.extend_from_slice(b);
        }
    }
}

// The interactive commit wrapper (bypassed by --edit): world_guard -> undo_push ->
// cell_commit; failure undo_drops and says `edit: %s`; success `cell written · step %d
// staged`; either way spc_return. kore.c cell_edit_commit.
pub fn cell_edit_commit(app: &mut App) {
    app.editing = false;
    if !world::world_guard(app) {
        spc_return(app);
        return;
    }
    let seq = world::undo_push(app);
    if seq < 0 {
        spc_return(app);
        app.sayerr("cannot stage undo copy");
        return;
    }
    let buf = app.edit_buf.clone();
    match world::cell_commit(app, &buf) {
        Err(err) => {
            world::undo_drop(app);
            let msg = format!("edit: {}", err);
            app.sayerr(&msg);
        }
        Ok(warn) => {
            // Ok(Some(_)) is the typed repair: the write landed, clamped, and says so
            let msg = match warn {
                Some(w) => format!("{} · cell written · step {} staged", w, seq),
                None => format!("cell written · step {} staged", seq),
            };
            app.say(&msg);
        }
    }
    spc_return(app);
}

// Restore the space cursor a map/bitmap edit parked. kore.c spc_return.
pub fn spc_return(app: &mut App) {
    if app.spc_ret == 0 {
        return;
    }
    app.spc_ret = 0;
    app.w_seg = 0;
    app.w_row = app.spc_row;
    app.w_col = app.spc_col;
}

// ---------- the space views ----------

// The declared lattice, else the positioned entities' bounding box (max coord + 1, coords
// in [0, 4096)). kore.c space_dims.
pub fn space_dims(app: &App) -> (i32, i32) {
    let w = &app.world;
    let mut gw = w.lat_w;
    let mut gh = w.lat_h;
    if gw == 0 {
        if let Some(pi) = w.pos() {
            let pe = &w.ents[pi];
            let mut i = 0usize;
            while i + 1 < pe.nums.len() {
                let dx = pe.nums[i];
                let dy = pe.nums[i + 1];
                if dx >= 0.0 && dx < 4096.0 && dx as i32 + 1 > gw {
                    gw = dx as i32 + 1;
                }
                if dy >= 0.0 && dy < 4096.0 && dy as i32 + 1 > gh {
                    gh = dy as i32 + 1;
                }
                i += 2;
            }
        }
    }
    (gw, gh)
}

// The num-field ramp: t<=0 '·', <0.25 '░', <0.5 '▒', <0.75 '▓', else '█'.
pub fn shade(v: f64, max: f64) -> &'static str {
    let max = if max <= 0.0 { 1.0 } else { max };
    let t = v / max;
    if t <= 0.0 { "·" } else if t < 0.25 { "░" } else if t < 0.5 { "▒" } else if t < 0.75 { "▓" } else { "█" }
}

// The glyph map: square cells two columns wide, ground '·' dim, fields paint in
// declaration order (char exact glyphs, bool '█' doubled, num shade doubled, x/y skip but
// count the palette ordinal), positioned entities on top under ent_glyph/ent_color (bold @
// fallback), cursor/drag reverse both columns. Draws only inside the border.
pub fn draw_space(app: &mut App, t: &mut Term) {
    let r = app.world_r;
    let (gw, gh) = space_dims(app);
    let pos = app.world.pos();
    if gw == 0 || gh == 0 {
        t.put(r.x + 2, r.y + 1, A_DIM, 0, "no lattice, no positions — table only (m cycles views)".as_bytes(), r.w - 4);
        return;
    }
    let ox = r.x + 2;
    let oy = r.y + 1;
    // square cells: two columns per cell; a cell draws only when both columns fit.
    // fields paint in declaration order, x/y coordinate fields skip but count pi.
    let mut cy = 0;
    while cy < gh && oy + cy < r.y + r.h - 1 {
        let mut cx = 0;
        while cx < gw && ox + 2 * cx + 2 <= r.x + r.w - 1 {
            let k = (cy * gw + cx) as usize;
            let mut g: Vec<u8> = "·".as_bytes().to_vec();
            let mut fg = 0u8;
            let mut attr = A_DIM;
            let mut pi = 0usize;
            let mut dbl = false; // block/shade glyphs double up
            for e in &app.world.ents {
                if e.kind != EKind::Field {
                    continue;
                }
                if world::names_eq(&e.name, b"x") || world::names_eq(&e.name, b"y") {
                    pi += 1;
                    continue;
                }
                if e.vtype == VType::Char {
                    if k < e.chars.len() && e.chars[k] != b'.' {
                        g = world::glyph_at(e, k as i32);
                        fg = FIELD_PAL[pi % 10];
                        attr = 0;
                        dbl = false;
                    }
                } else if k < e.nums.len() && e.nums[k] != 0.0 {
                    if e.vtype == VType::Bool {
                        g = "█".as_bytes().to_vec();
                        fg = FIELD_PAL[pi % 10];
                        attr = 0;
                        dbl = true;
                    } else {
                        let mut max = 0.0f64;
                        for &v in &e.nums {
                            if v > max {
                                max = v;
                            }
                        }
                        g = shade(e.nums[k], max).as_bytes().to_vec();
                        fg = FIELD_PAL[pi % 10];
                        attr = 0;
                        dbl = true;
                    }
                }
                pi += 1;
            }
            t.put(ox + 2 * cx, oy + cy, attr, fg, &g, 1);
            t.put(ox + 2 * cx + 1, oy + cy, attr, fg, if dbl { &g[..] } else { b" " }, 1);
            cx += 1;
        }
        cy += 1;
    }
    // positioned entities stand on their cells: glyph when one resolves, bold @ otherwise
    if let Some(pi) = pos {
        let mut i = 0usize;
        loop {
            let pe = &app.world.ents[pi];
            if i + 1 >= pe.nums.len() {
                break;
            }
            let dx = pe.nums[i];
            let dy = pe.nums[i + 1];
            i += 2;
            if dx < 0.0 || dx >= gw as f64 || dy < 0.0 || dy >= gh as f64 {
                continue; // guard before the cast
            }
            let px = dx as i32;
            let py = dy as i32;
            if ox + 2 * px + 2 > r.x + r.w - 1 || oy + py >= r.y + r.h - 1 {
                continue;
            }
            let row = ((i - 2) / 2) as i32;
            let eg = app.world.ent_glyph(row);
            let fg = app.world.ent_color(row, eg.as_deref().unwrap_or(b""));
            let (gstr, gattr): (&[u8], u8) = match &eg {
                Some(v) => (v.as_slice(), 0),
                None => (b"@", A_BOLD),
            };
            let gwd = t.putp(ox + 2 * px, oy + py, gattr, fg, 0, gstr, 2);
            if gwd < 2 {
                t.put(ox + 2 * px + 1, oy + py, 0, fg, b" ", 1);
            }
        }
    }
    // the cell cursor works on the map exactly as on the table
    if app.focus == Focus::World {
        let cx = ox + 2 * app.w_col;
        let cy = oy + app.w_row;
        if app.w_col < gw && app.w_row < gh && cx + 2 <= r.x + r.w - 1 && cy < r.y + r.h - 1 {
            t.rev_cell(cx, cy);
            t.rev_cell(cx + 1, cy);
        }
    }
    if app.dragging {
        let rr0 = app.drag_r0.min(app.drag_r1);
        let rr1 = app.drag_r0.max(app.drag_r1);
        let cc0 = app.drag_c0.min(app.drag_c1);
        let cc1 = app.drag_c0.max(app.drag_c1);
        let mut yy = rr0;
        while yy <= rr1 && yy < gh {
            let mut xx = cc0;
            while xx <= cc1 && xx < gw {
                if xx >= 0 && yy >= 0 && ox + 2 * xx + 2 <= r.x + r.w - 1 && oy + yy < r.y + r.h - 1 {
                    t.rev_cell(ox + 2 * xx, oy + yy);
                    t.rev_cell(ox + 2 * xx + 1, oy + yy);
                }
                xx += 1;
            }
            yy += 1;
        }
    }
}

// The same picture at pixel scale: a color plane, fields then entities, rendered as ▀
// half-blocks (fg upper pixel, bg lower, C_BG past the edge), num fields the gray ramp
// 236 + (t*12.0) as i32; cursor reverses the pixel's terminal cell. kore.c draw_bitmap.
pub fn draw_bitmap(app: &mut App, t: &mut Term) {
    let r = app.world_r;
    let (gw, gh) = space_dims(app);
    let pos = app.world.pos();
    if gw == 0 || gh == 0 {
        t.put(r.x + 2, r.y + 1, A_DIM, 0, "no lattice, no positions — table only (m cycles views)".as_bytes(), r.w - 4);
        return;
    }
    let ox = r.x + 2;
    let oy = r.y + 1;
    let mut pix = vec![C_BG; (gw * gh) as usize];
    let mut pi = 0usize;
    for e in &app.world.ents {
        if e.kind != EKind::Field {
            continue;
        }
        if world::names_eq(&e.name, b"x") || world::names_eq(&e.name, b"y") {
            pi += 1;
            continue;
        }
        if e.vtype == VType::Char {
            let len = e.chars.len();
            let mut k = 0usize;
            while k < (gw * gh) as usize && k < len {
                if e.chars[k] != b'.' {
                    pix[k] = FIELD_PAL[pi % 10];
                }
                k += 1;
            }
        } else {
            let mut max = 0.0f64;
            for &v in &e.nums {
                if v > max {
                    max = v;
                }
            }
            if max <= 0.0 {
                max = 1.0;
            }
            let mut k = 0usize;
            while k < (gw * gh) as usize && k < e.nums.len() {
                if e.nums[k] != 0.0 {
                    // nums ramp the xterm grayscale 236..248 — visible on the 234 canvas
                    let mut tt = e.nums[k] / max;
                    if tt < 0.0 {
                        tt = 0.0;
                    }
                    if tt > 1.0 {
                        tt = 1.0;
                    }
                    pix[k] = if e.vtype == VType::Bool { FIELD_PAL[pi % 10] } else { (236 + (tt * 12.0) as i32) as u8 };
                }
                k += 1;
            }
        }
        pi += 1;
    }
    if let Some(pidx) = pos {
        let mut i = 0usize;
        loop {
            let pe = &app.world.ents[pidx];
            if i + 1 >= pe.nums.len() {
                break;
            }
            let dx = pe.nums[i];
            let dy = pe.nums[i + 1];
            i += 2;
            if dx < 0.0 || dx >= gw as f64 || dy < 0.0 || dy >= gh as f64 {
                continue;
            }
            let row = ((i - 2) / 2) as i32;
            let eg = app.world.ent_glyph(row);
            pix[(dy as i32 * gw + dx as i32) as usize] = app.world.ent_color(row, eg.as_deref().unwrap_or(b""));
        }
    }
    let mut ty = 0;
    while 2 * ty < gh && oy + ty < r.y + r.h - 1 {
        let mut cx = 0;
        while cx < gw && ox + cx < r.x + r.w - 1 {
            let up = pix[(2 * ty * gw + cx) as usize];
            let lo = if 2 * ty + 1 < gh { pix[((2 * ty + 1) * gw + cx) as usize] } else { C_BG };
            t.putp(ox + cx, oy + ty, 0, up, lo, "▀".as_bytes(), 1);
            cx += 1;
        }
        ty += 1;
    }
    // the cell cursor addresses one pixel; the reverse marks its ▀ pair
    if app.focus == Focus::World && app.w_col < gw && app.w_row < gh {
        let cx = ox + app.w_col;
        let cy = oy + app.w_row / 2;
        if cx < r.x + r.w - 1 && cy < r.y + r.h - 1 {
            t.rev_cell(cx, cy);
        }
    }
    if app.dragging {
        let rr0 = app.drag_r0.min(app.drag_r1);
        let rr1 = app.drag_r0.max(app.drag_r1);
        let cc0 = app.drag_c0.min(app.drag_c1);
        let cc1 = app.drag_c0.max(app.drag_c1);
        let mut yy = rr0;
        while yy <= rr1 && yy < gh {
            let mut xx = cc0;
            while xx <= cc1 && xx < gw {
                if xx >= 0 && yy >= 0 && ox + xx < r.x + r.w - 1 && oy + yy / 2 < r.y + r.h - 1 {
                    t.rev_cell(ox + xx, oy + yy / 2);
                }
                xx += 1;
            }
            yy += 1;
        }
    }
}

// ---------- outputs, history, prompt, status ----------

// Newest group first: seam `— step %d` / `— run` dim, records `q%d · label → value`,
// multi-line values indented at x+6 (500-byte caps, text::u8_tail_fix), top-anchored
// scroll clamped to outputs_total_lines() - vis. kore.c draw_outputs.
pub fn draw_outputs(app: &mut App, t: &mut Term) {
    let r = app.outputs_r;
    if r.h <= 1 {
        return;
    }
    t.draw_box(r, "outputs", C_OUTPUTSC, app.focus == Focus::Outputs);
    let vis = r.h - 2;
    if vis < 1 {
        return;
    }
    let total = app.outputs_total_lines();
    let mut max = total - vis;
    if max < 0 {
        max = 0;
    }
    if app.outputs_scroll > max {
        app.outputs_scroll = max;
    }
    if app.outputs_scroll < 0 {
        app.outputs_scroll = 0;
    }
    if app.qgroups.is_empty() {
        t.put(r.x + 2, r.y + 1, A_DIM, 0, "query results land here — n ticks the demo, the prompt asks".as_bytes(), r.w - 4);
        return;
    }
    let mut li = 0i32;
    let mut y = r.y + 1;
    let yend = r.y + r.h - 1;
    let mut gi = app.qgroups.len();
    while gi > 0 && y < yend {
        gi -= 1;
        let step = app.qgroups[gi].step;
        if li >= app.outputs_scroll {
            let seam = if step > 0 { format!("— step {}", step) } else { "— run".to_string() };
            t.put(r.x + 2, y, A_DIM, C_FRAME, seam.as_bytes(), r.w - 4);
            y += 1;
        }
        li += 1;
        let nrecs = app.qgroups[gi].recs.len();
        let mut ri = 0usize;
        while ri < nrecs && y < yend {
            let q = &app.qgroups[gi].recs[ri];
            let multi = q.value.contains(&b'\n');
            if li >= app.outputs_scroll {
                let mut x = r.x + 2;
                let xe = r.x + r.w - 2;
                let ord = format!("q{}", ri + 1);
                x += t.put(x, y, A_BOLD, C_OUTPUTSC, ord.as_bytes(), xe - x);
                x += t.put(x, y, A_DIM, C_FRAME, " · ".as_bytes(), xe - x);
                x += t.put(x, y, 0, 0, &q.label, xe - x);
                if !multi {
                    x += t.put(x, y, A_DIM, C_FRAME, " → ".as_bytes(), xe - x);
                    t.put(x, y, A_BOLD, C_OUTPUTSC, &q.value, xe - x);
                }
                y += 1;
            }
            li += 1;
            if multi {
                let mut p = 0usize;
                while p < q.value.len() && y < yend {
                    let rel = q.value[p..].iter().position(|&b| b == b'\n');
                    let end = rel.map(|k| p + k).unwrap_or(q.value.len());
                    if li >= app.outputs_scroll {
                        let cap = (end - p).min(500);
                        let mut vb = q.value[p..p + cap].to_vec();
                        text::u8_tail_fix(&mut vb);
                        t.put(r.x + 6, y, 0, C_OUTPUTSC, &vb, r.w - 8);
                        y += 1;
                    }
                    li += 1;
                    match rel {
                        None => break,
                        Some(_) => p = end + 1,
                    }
                }
            }
            ri += 1;
        }
    }
    t.scrollbar(r, app.outputs_scroll, vis, total, C_OUTPUTSC);
}

// The history strip: log tail with per-line tints (`> ` prompt hue, `$ ` gold dim,
// error/FAIL/cannot and ` IS DEAD !`/` IS EMPTY !` error hue, ` rows -> ` world dim),
// the verdict line bold at r.y+r.h-2 (`—` when empty). kore.c draw_out.
pub fn draw_out(app: &mut App, t: &mut Term) {
    let r = app.out_r;
    if r.h <= 1 {
        return;
    }
    t.draw_box(r, "history", C_OUTC, app.focus == Focus::Out);
    // last lines of the log, minus the scrollback
    let vis = r.h - 3;
    let nls = app.out_log.iter().filter(|&&b| b == b'\n').count() as i32;
    let mut first = nls - vis - app.out_scroll;
    if first < 0 {
        first = 0;
    }
    let mut li = 0i32;
    let mut yy = r.y + 1;
    let mut p = 0usize;
    while p < app.out_log.len() && yy < r.y + r.h - 2 {
        let rel = app.out_log[p..].iter().position(|&b| b == b'\n');
        let end = rel.map(|k| p + k).unwrap_or(app.out_log.len());
        if li >= first {
            let cap = (end - p).min(500);
            let line = &app.out_log[p..p + cap];
            // echoes tint by origin, failures by content; trace diagnostics by their formats
            let mut fg = 0u8;
            let mut attr = 0u8;
            if line.len() >= 2 && line[0] == b'>' && line[1] == b' ' {
                fg = C_PROMPTC;
            } else if line.len() >= 2 && line[0] == b'$' && line[1] == b' ' {
                fg = C_CODEC;
                attr = A_DIM;
            } else if contains(line, b"error") || contains(line, b"FAIL") || contains(line, b"cannot") {
                fg = C_ERR;
            } else if contains(line, b" IS DEAD !") || contains(line, b" IS EMPTY !") {
                fg = C_ERR;
            } else if contains(line, b" rows -> ") {
                fg = C_WORLDC;
                attr = A_DIM;
            }
            t.put(r.x + 2, yy, attr, fg, line, r.w - 4);
            yy += 1;
        }
        li += 1;
        match rel {
            None => break,
            Some(_) => p = end + 1,
        }
    }
    // the verdict line: pins held, the failure, or the save/undo status — unambiguous
    let (vfg, vtext): (u8, &[u8]) = if !app.verdict.is_empty() {
        (if app.verdict_bad { C_ERR } else { C_OK }, &app.verdict)
    } else {
        (0, "—".as_bytes())
    };
    t.put(r.x + 2, r.y + r.h - 2, A_BOLD, vfg, vtext, r.w - 4);
    t.scrollbar(r, first, vis, nls, C_OUTC);
}

// The boxed prompt: `>` / `·` line heads, the cursor line's horizontal rune scroll with
// the `…` clipped-head mark, ptop follows the cursor line, rev_cell cursor.
pub fn draw_prompt(app: &mut App, t: &mut Term) {
    let r = app.prompt_r;
    let on = app.focus == Focus::Prompt;
    t.draw_box(r, if app.mode == Mode::Reg { "prompt · the program" } else { "prompt" }, C_PROMPTC, on);
    let mut vis = r.h - 2;
    if vis < 1 {
        vis = 1;
    }
    // the cursor's line, and the line count
    let mut cl = 0i32;
    let mut nlines = 1i32;
    for (i, &b) in app.prompt.iter().enumerate() {
        if b != b'\n' {
            continue;
        }
        nlines += 1;
        if i < app.pcur {
            cl += 1;
        }
    }
    if app.ptop > cl {
        app.ptop = cl;
    }
    if cl >= app.ptop + vis {
        app.ptop = cl - vis + 1;
    }
    if app.ptop > nlines - vis {
        app.ptop = nlines - vis;
    }
    if app.ptop < 0 {
        app.ptop = 0;
    }
    let mut avail = r.w - 7;
    if avail < 8 {
        avail = 8;
    }
    let mut lstart = 0usize;
    let mut li = 0i32;
    loop {
        let rel = app.prompt[lstart..].iter().position(|&b| b == b'\n');
        let lend = rel.map(|k| lstart + k).unwrap_or(app.prompt.len());
        if li >= app.ptop && li < app.ptop + vis {
            let y = r.y + 1 + li - app.ptop;
            t.put(r.x + 2, y, A_BOLD, if on { C_PROMPTC } else { C_FRAME }, if li == 0 { b">" } else { "·".as_bytes() }, 1);
            let mut off = 0usize;
            if li == cl {
                // horizontal window on the cursor's line only
                if app.pscroll < lstart || app.pscroll > app.pcur {
                    app.pscroll = lstart;
                }
                loop {
                    let wseg = text::swidth(&app.prompt[app.pscroll..app.pcur]);
                    if wseg < avail {
                        break;
                    }
                    let mut q = app.pscroll;
                    text::u8next(&app.prompt, &mut q);
                    app.pscroll = q;
                }
                off = app.pscroll - lstart;
            }
            t.put(r.x + 4, y, if on { 0 } else { A_DIM }, 0, &app.prompt[lstart + off..lend], avail);
            if li == cl && off > 0 {
                t.put(r.x + 3, y, A_DIM, C_PROMPTC, "…".as_bytes(), 1);
            }
            if on && li == cl {
                let cx = r.x + 4 + text::swidth(&app.prompt[app.pscroll..app.pcur]);
                if cx < r.x + r.w - 1 {
                    t.rev_cell(cx, y);
                }
            }
        }
        match rel {
            None => break,
            Some(_) => {
                lstart = lend + 1;
                li += 1;
            }
        }
    }
    t.scrollbar(r, app.ptop, vis, nlines, C_PROMPTC);
}

// The byte bounds of the prompt line containing byte position at. kore.c prompt_line_at.
pub fn prompt_line_at(prompt: &[u8], at: usize) -> (usize, usize) {
    let mut s = at;
    while s > 0 && prompt[s - 1] != b'\n' {
        s -= 1;
    }
    let mut e = at;
    while e < prompt.len() && prompt[e] != b'\n' {
        e += 1;
    }
    (s, e)
}

// Insert '\n' at the cursor (shift-enter, alt-enter). kore.c prompt_newline.
fn prompt_newline(app: &mut App) {
    if app.prompt.len() + 1 >= 1023 {
        return;
    }
    app.prompt.insert(app.pcur, b'\n');
    app.pcur += 1;
}

// ESC leaves (world/rail/code by mode); Enter: trailing `\` becomes a newline, else
// world::repl_submit; K_NEWLINE inserts '\n'; BS/arrows/HOME/END rune- and line-wise;
// Up/Down walk lines then history (past the newest restores an empty prompt); chars
// insert at the cursor (cap 1023). kore.c prompt_key.
pub fn prompt_key(app: &mut App, e: &Ev) {
    if e.etype == EV_KEY {
        match e.key {
            K_ESC => {
                app.focus = if app.world.loaded {
                    Focus::World
                } else if app.mode == Mode::Rail {
                    Focus::Rail
                } else {
                    Focus::Code
                };
            }
            K_ENTER => {
                // \⏎ asks for a line, not a run — the backslash becomes the newline
                if app.pcur > 0 && app.prompt[app.pcur - 1] == b'\\' {
                    app.prompt[app.pcur - 1] = b'\n';
                    return;
                }
                world::repl_submit(app);
            }
            K_NEWLINE => prompt_newline(app),
            K_BS => {
                if app.pcur > 0 {
                    let mut prev = app.pcur - 1;
                    while prev > 0 && (app.prompt[prev] & 0xC0) == 0x80 {
                        prev -= 1;
                    }
                    app.prompt.drain(prev..app.pcur);
                    app.pcur = prev;
                }
            }
            K_LEFT => {
                if app.pcur > 0 {
                    app.pcur -= 1;
                    while app.pcur > 0 && (app.prompt[app.pcur] & 0xC0) == 0x80 {
                        app.pcur -= 1;
                    }
                }
            }
            K_RIGHT => {
                if app.pcur < app.prompt.len() {
                    app.pcur += 1;
                    while app.pcur < app.prompt.len() && (app.prompt[app.pcur] & 0xC0) == 0x80 {
                        app.pcur += 1;
                    }
                }
            }
            K_HOME => {
                let (ls, _) = prompt_line_at(&app.prompt, app.pcur);
                app.pcur = ls;
            }
            K_END => {
                let (_, le) = prompt_line_at(&app.prompt, app.pcur);
                app.pcur = le;
            }
            K_UP => {
                // within a multi-line statement the arrows walk lines; history past the top
                let (ls, _le) = prompt_line_at(&app.prompt, app.pcur);
                if ls > 0 {
                    let col = line_col_of(&app.prompt[ls..], app.pcur - ls);
                    let (pls, ple) = prompt_line_at(&app.prompt, ls - 1);
                    let mut at = line_byte_at(&app.prompt[pls..], col);
                    if at > ple - pls {
                        at = ple - pls;
                    }
                    app.pcur = pls + at;
                    return;
                }
                if app.hist_at > 0 {
                    app.hist_at -= 1;
                    app.prompt = app.hist[app.hist_at as usize].clone();
                    app.prompt.truncate(1023);
                    app.pcur = app.prompt.len();
                }
            }
            K_DOWN => {
                let (ls, le) = prompt_line_at(&app.prompt, app.pcur);
                if le < app.prompt.len() && app.prompt[le] == b'\n' {
                    let col = line_col_of(&app.prompt[ls..], app.pcur - ls);
                    let (nls, nle) = prompt_line_at(&app.prompt, le + 1);
                    let mut at = line_byte_at(&app.prompt[nls..], col);
                    if at > nle - nls {
                        at = nle - nls;
                    }
                    app.pcur = nls + at;
                    return;
                }
                if app.hist_at < app.hist.len() as i32 - 1 {
                    app.hist_at += 1;
                    app.prompt = app.hist[app.hist_at as usize].clone();
                    app.prompt.truncate(1023);
                } else {
                    app.hist_at = app.hist.len() as i32;
                    app.prompt.clear();
                }
                app.pcur = app.prompt.len();
            }
            _ => {}
        }
        return;
    }
    if e.etype == EV_CHAR {
        let b = ev_bytes(e);
        if app.prompt.len() + b.len() < 1023 {
            let at = app.pcur;
            app.prompt.splice(at..at, b.iter().copied());
            app.pcur += b.len();
        }
    }
}

// The bottom row: one dim hint by context (confirm-reset, prompt, editing, searching,
// insert, code, outputs, default) and the bold mode chip at the right edge.
pub fn draw_status(app: &mut App, t: &mut Term) {
    let y = t.rows - 1;
    let cols = t.cols;
    t.fill(0, y, cols, 1, b" ", 0, 0);
    let hint: &str = if app.confirm_reset {
        "y wipes every play copy — demos/ becomes the only state · any other key cancels"
    } else if app.focus == Focus::Prompt {
        "enter runs · \\⏎ or shift-enter breaks a line · ↑↓ lines, history · esc leaves"
    } else if app.focus == Focus::World && app.editing {
        "enter commits · esc cancels"
    } else if app.focus == Focus::Code && app.searching {
        "type the pattern · enter jumps · esc cancels"
    } else if app.focus == Focus::Code && app.code_insert {
        "insert — esc returns to browse"
    } else if app.focus == Focus::Code {
        "hjkl w b gg G 0 ^ $ move · / search, n N · i a o insert · x dd delete · u undo · s save"
    } else if app.focus == Focus::Outputs {
        "outputs — j k scroll · pgup pgdn page · newest tick first · u drops a tick"
    } else {
        "tab focus · > prompt · r reset · n next · m view (table/map/bitmap) · u undo · w snap · t trace · E editor · q quit"
    };
    t.put(1, y, A_DIM, 0, hint.as_bytes(), cols - 10);
    let mode = match app.mode {
        Mode::Rail => "rail",
        Mode::Reg => "world",
        Mode::Demo => "demo",
    };
    let mfg = match app.mode {
        Mode::Rail => C_RAILC,
        Mode::Reg => C_WORLDC,
        Mode::Demo => C_CODEC,
    };
    let mw = text::swidth(mode.as_bytes());
    t.put(cols - mw - 2, y, A_BOLD, mfg, mode.as_bytes(), mw);
}

// ---------- dispatch ----------

// The order is contract (kore.c handle): EV_NONE; the armed >reset modal (y/Y runs
// world::reset_all, anything else `reset cancelled`); K_RESETALL -> kore_command("reset");
// mouse; text-entry contexts swallow (prompt / editing / searching / insert); global chars
// q : > m r n u w t E with the code-focus yields (n with a search set, u always, w always;
// t never); ESC (search clear first, else the mode's home surface); Tab cycle skipping
// F_RAIL outside MODE_RAIL and F_CODE in MODE_REG; per-focus fallthrough.
pub fn handle(app: &mut App, t: &mut Term, e: &Ev) {
    if e.etype == EV_NONE {
        return;
    }
    // an armed >reset: y wipes, anything else cancels — the one modal in kore
    if app.confirm_reset {
        app.confirm_reset = false;
        if e.etype == EV_CHAR && (e.ch == b'y' as u32 || e.ch == b'Y' as u32) {
            world::reset_all(app);
        } else {
            app.say("reset cancelled");
        }
        return;
    }
    if e.etype == EV_KEY && e.key == K_RESETALL {
        world::kore_command(app, "reset");
        return;
    }
    if e.etype == EV_MOUSE {
        mouse_ev(app, e);
        return;
    }
    // text-entry contexts swallow everything
    if app.focus == Focus::Prompt {
        prompt_key(app, e);
        return;
    }
    if app.focus == Focus::World && app.editing {
        edit_key(app, e);
        return;
    }
    if app.focus == Focus::Code && app.searching {
        search_key(app, e);
        return;
    }
    if app.focus == Focus::Code && app.code_insert {
        code_key(app, e);
        return;
    }
    // global keys — u, w, n yield to the code surface (vim undo, word motion, match)
    if e.etype == EV_CHAR {
        let ch = e.ch;
        if ch == b'q' as u32 {
            app.quit = true;
            return;
        } else if ch == b':' as u32 {
            app.focus = Focus::Prompt;
            return;
        } else if ch == b'>' as u32 {
            // the command form: an empty prompt opens pre-filled with >
            app.focus = Focus::Prompt;
            if app.prompt.is_empty() {
                app.prompt.push(b'>');
                app.pcur = 1;
            }
            return;
        } else if ch == b'm' as u32 {
            app.space_view = (app.space_view + 1) % 3;
            app.w_row = 0;
            app.w_col = 0;
            return;
        } else if ch == b'r' as u32 {
            world::world_reset(app);
            return;
        } else if ch == b'n' as u32 && !(app.focus == Focus::Code && !app.search.is_empty()) {
            world::world_next(app);
            return;
        } else if ch == b'u' as u32 && app.focus != Focus::Code {
            world::undo_pop(app);
            return;
        } else if ch == b'w' as u32 && app.focus != Focus::Code {
            world::snapshot(app);
            return;
        } else if ch == b't' as u32 {
            // trace toggle: observability only — post-state identical either way;
            // t is no code-pane vim key, so it never yields the way u/w/n do
            app.trace = !app.trace;
            if app.trace {
                app.say("trace on — dead links and tick deltas land in history");
            } else {
                app.say("trace off");
            }
            return;
        } else if ch == b'E' as u32 {
            editor_hop(app, t);
            return;
        }
    }
    // esc steps out: an active search clears first, then any panel returns to the
    // mode's home surface — the rail, the code, or the prompt
    if e.etype == EV_KEY && e.key == K_ESC {
        if app.focus == Focus::Code && !app.search.is_empty() {
            app.search.clear();
            app.say("search cleared");
            return;
        }
        app.focus = match app.mode {
            Mode::Rail => Focus::Rail,
            Mode::Reg => Focus::Prompt,
            Mode::Demo => Focus::Code,
        };
        return;
    }
    if e.etype == EV_KEY && e.key == K_TAB {
        let order = [Focus::Rail, Focus::Code, Focus::World, Focus::Outputs, Focus::Out, Focus::Prompt];
        let mut at = 0usize;
        for (i, f) in order.iter().enumerate() {
            if *f == app.focus {
                at = i;
            }
        }
        for i in 1..=6usize {
            let f = order[(at + i) % 6];
            if f == Focus::Rail && app.mode != Mode::Rail {
                continue;
            }
            if f == Focus::Code && app.mode == Mode::Reg {
                continue;
            }
            app.focus = f;
            break;
        }
        return;
    }
    match app.focus {
        Focus::Rail => rail_key(app, e),
        Focus::Code => code_key(app, e),
        Focus::World => world_key(app, e),
        Focus::Outputs => {
            let mut vis = app.outputs_r.h - 2;
            if vis < 1 {
                vis = 1;
            }
            let mut max = app.outputs_total_lines() - vis;
            if max < 0 {
                max = 0;
            }
            let ch = if e.etype == EV_CHAR { e.ch } else { 0 };
            let key = if e.etype == EV_KEY { e.key } else { 0 };
            if ch == b'j' as u32 || key == K_DOWN {
                app.outputs_scroll += 1;
            } else if ch == b'k' as u32 || key == K_UP {
                app.outputs_scroll -= 1;
            } else if key == K_PGDN {
                app.outputs_scroll += vis;
            } else if key == K_PGUP {
                app.outputs_scroll -= vis;
            }
            if app.outputs_scroll < 0 {
                app.outputs_scroll = 0;
            }
            if app.outputs_scroll > max {
                app.outputs_scroll = max;
            }
        }
        Focus::Out => {
            if e.etype == EV_CHAR && e.ch == b'j' as u32 && app.out_scroll > 0 {
                app.out_scroll -= 1;
            }
            if e.etype == EV_CHAR && e.ch == b'k' as u32 {
                app.out_scroll += 1;
            }
            if e.etype == EV_KEY && e.key == K_DOWN && app.out_scroll > 0 {
                app.out_scroll -= 1;
            }
            if e.etype == EV_KEY && e.key == K_UP {
                app.out_scroll += 1;
            }
            if e.etype == EV_KEY && e.key == K_PGUP {
                app.out_scroll += if app.out_r.h > 4 { app.out_r.h - 3 } else { 5 };
            }
            if e.etype == EV_KEY && e.key == K_PGDN {
                app.out_scroll -= if app.out_r.h > 4 { app.out_r.h - 3 } else { 5 };
                if app.out_scroll < 0 {
                    app.out_scroll = 0;
                }
            }
        }
        _ => {}
    }
}

// Wheel scrolls the panel under the pointer (prompt walks history); press focuses and
// cursors (rail double-select opens, world table/space cell math per the map); drag
// updates the selection; release fires drag_skeleton only when the drag moved.
pub fn mouse_ev(app: &mut App, e: &Ev) {
    let x = e.mx;
    let y = e.my;
    if e.mkind == M_WHEELUP || e.mkind == M_WHEELDN {
        // one item per notch — the view follows one line at a time, never a leap
        let d = if e.mkind == M_WHEELUP { -1 } else { 1 };
        let n_demos = app.demo_list.len() as i32;
        if hit(app.rail_r, x, y) {
            app.rail_sel += d;
            if app.rail_sel < 0 {
                app.rail_sel = 0;
            }
            if app.rail_sel >= n_demos {
                app.rail_sel = if n_demos != 0 { n_demos - 1 } else { 0 };
            }
        } else if hit(app.code_r, x, y) {
            app.ccy += d;
            if app.ccy < 0 {
                app.ccy = 0;
            }
            if app.ccy >= app.code.len() as i32 {
                app.ccy = if !app.code.is_empty() { app.code.len() as i32 - 1 } else { 0 };
            }
        } else if hit(app.world_r, x, y) {
            let rows = if app.space_view != 0 { space_dims(app).1 } else { world::seg_rows(app, app.w_seg) };
            app.w_row += d;
            if app.w_row < 0 {
                app.w_row = 0;
            }
            if app.w_row >= rows {
                app.w_row = if rows != 0 { rows - 1 } else { 0 };
            }
        } else if hit(app.outputs_r, x, y) {
            let mut vis = app.outputs_r.h - 2;
            if vis < 1 {
                vis = 1;
            }
            let mut max = app.outputs_total_lines() - vis;
            if max < 0 {
                max = 0;
            }
            app.outputs_scroll += d;
            if app.outputs_scroll < 0 {
                app.outputs_scroll = 0;
            }
            if app.outputs_scroll > max {
                app.outputs_scroll = max;
            }
        } else if hit(app.out_r, x, y) {
            app.out_scroll -= d;
            if app.out_scroll < 0 {
                app.out_scroll = 0;
            }
        } else if hit(app.prompt_r, x, y) {
            // the wheel walks the session history, exactly as ↑↓ do
            app.focus = Focus::Prompt;
            if e.mkind == M_WHEELUP && app.hist_at > 0 {
                app.hist_at -= 1;
                app.prompt = app.hist[app.hist_at as usize].clone();
                app.prompt.truncate(1023);
                app.pcur = app.prompt.len();
            } else if e.mkind == M_WHEELDN {
                if app.hist_at < app.hist.len() as i32 - 1 {
                    app.hist_at += 1;
                    app.prompt = app.hist[app.hist_at as usize].clone();
                    app.prompt.truncate(1023);
                } else {
                    app.hist_at = app.hist.len() as i32;
                    app.prompt.clear();
                }
                app.pcur = app.prompt.len();
            }
        }
        return;
    }
    if e.mkind == M_PRESS {
        if hit(app.prompt_r, x, y) {
            app.focus = Focus::Prompt;
            return;
        }
        if hit(app.rail_r, x, y) {
            app.focus = Focus::Rail;
            let i = app.rail_top + (y - app.rail_r.y - 1);
            if i >= 0 && i < app.demo_list.len() as i32 {
                if i == app.rail_sel {
                    let path = app.demo_list[i as usize].clone();
                    world::open_demo(app, &path);
                    app.focus = Focus::World;
                } else {
                    app.rail_sel = i;
                }
            }
            return;
        }
        if hit(app.code_r, x, y) {
            app.focus = Focus::Code;
            let li = app.code_top + (y - app.code_r.y - 1);
            if li >= 0 && li < app.code.len() as i32 {
                app.ccy = li;
                app.ccx = x - app.code_r.x - 1;
                let lw = text::swidth(&app.code[li as usize]);
                if app.ccx > lw {
                    app.ccx = lw;
                }
            }
            return;
        }
        if hit(app.outputs_r, x, y) {
            app.focus = Focus::Outputs;
            return;
        }
        if hit(app.out_r, x, y) {
            app.focus = Focus::Out;
            return;
        }
        if hit(app.world_r, x, y) {
            app.focus = Focus::World;
            if app.space_view != 0 {
                let (gw, gh) = space_dims(app);
                // map cells are two columns wide; a bitmap column is one cell, its row
                // the ▀ pair's upper pixel
                let rx = x - app.world_r.x - 2;
                let ry = y - app.world_r.y - 1;
                let cx = if app.space_view == 2 { rx } else { rx / 2 };
                let cy = if app.space_view == 2 { 2 * ry } else { ry };
                if rx >= 0 && ry >= 0 && cx < gw && cy < gh {
                    app.w_col = cx;
                    app.w_row = cy;
                    app.dragging = true;
                    app.drag_seg = 1;
                    app.drag_r0 = cy;
                    app.drag_r1 = cy;
                    app.drag_c0 = cx;
                    app.drag_c1 = cx;
                }
                return;
            }
            world::table_cols(app);
            world_vrows(app);
            let vi = app.w_top + (y - app.world_r.y - 2);
            if vi >= 0 && (vi as usize) < app.vrows.len() {
                let vr = app.vrows[vi as usize];
                if vr.kind == 0 {
                    app.w_seg = 0;
                    app.w_row = vr.row;
                    let mut cx = app.world_r.x + 1 + 5;
                    for c in 0..app.dcols.len() {
                        if x >= cx && x < cx + app.dcols[c].width {
                            app.w_col = c as i32;
                            break;
                        }
                        cx += app.dcols[c].width + 1;
                    }
                    app.dragging = true;
                    app.drag_seg = 0;
                    app.drag_r0 = vr.row;
                    app.drag_r1 = vr.row;
                    app.drag_c0 = app.w_col;
                    app.drag_c1 = app.w_col;
                } else if vr.kind == 2 {
                    if let Some(fi) = world::seg_field(app, vr.seg) {
                        let (cell_w, pad) = field_cellw(app, fi);
                        let gx = (x - app.world_r.x - 1) / (cell_w + pad);
                        let gw = if app.world.lat_w != 0 { app.world.lat_w } else { app.world.ents[fi].nums.len() as i32 };
                        if gx >= 0 && gx < gw {
                            app.w_seg = vr.seg;
                            app.w_row = vr.row;
                            app.w_col = gx;
                        }
                    }
                }
            }
            return;
        }
    }
    if e.mkind == M_DRAG && app.dragging {
        if app.space_view != 0 {
            let rx = x - app.world_r.x - 2;
            let ry = y - app.world_r.y - 1;
            let cx = if app.space_view == 2 { rx } else { rx / 2 };
            let cy = if app.space_view == 2 { 2 * ry } else { ry };
            if rx >= 0 && ry >= 0 {
                app.drag_c1 = cx;
                app.drag_r1 = cy;
            }
        } else {
            let row = app.w_top + (y - app.world_r.y - 2);
            if row >= 0 && row < app.world.n {
                app.drag_r1 = row;
            }
        }
        return;
    }
    if e.mkind == M_RELEASE && app.dragging {
        app.dragging = false;
        if app.drag_r0 != app.drag_r1 || app.drag_c0 != app.drag_c1 {
            drag_skeleton(app);
        }
    }
}

// The predicate skeleton pre-fill: rows `index == %d , ` / `index >= %d & index <= %d , `;
// lattice cells `%d %d & x >= %s & … , ` (registered x/y fields sampled at the drag
// corners, min/max normalized); positioned `%.100s.x >= %d & … , `; else `nothing to
// select here`. Focus to the prompt, verdict `selection painted — finish the effect`.
pub fn drag_skeleton(app: &mut App) {
    let r0 = app.drag_r0.min(app.drag_r1);
    let r1 = app.drag_r0.max(app.drag_r1);
    let body: Vec<u8>;
    if app.drag_seg == 0 && app.space_view == 0 {
        body = if r0 == r1 {
            format!("index == {} , ", r0).into_bytes()
        } else {
            format!("index >= {} & index <= {} , ", r0, r1).into_bytes()
        };
    } else if app.world.lat_w != 0 {
        let c0 = app.drag_c0.min(app.drag_c1);
        let c1 = app.drag_c0.max(app.drag_c1);
        // registered x/y fields win, else the emitter's lattice frame: x the column,
        // y the row — sample the fields so either convention lands right
        let fx = app.world.ent(b"x", EKind::Field);
        let fy = app.world.ent(b"y", EKind::Field);
        let w = app.world.lat_w;
        let mut xlo = c0 as f64;
        let mut xhi = c1 as f64;
        let mut ylo = r0 as f64;
        let mut yhi = r1 as f64;
        if let (Some(fx), Some(fy)) = (fx, fy) {
            let ex = &app.world.ents[fx];
            let ey = &app.world.ents[fy];
            let need = (r1 * w + c1) as usize;
            if ex.nums.len() > need && ey.nums.len() > need {
                xlo = ex.nums[(r0 * w + c0) as usize];
                xhi = ex.nums[(r1 * w + c1) as usize];
                ylo = ey.nums[(r0 * w + c0) as usize];
                yhi = ey.nums[(r1 * w + c1) as usize];
                if xlo > xhi {
                    std::mem::swap(&mut xlo, &mut xhi);
                }
                if ylo > yhi {
                    std::mem::swap(&mut ylo, &mut yhi);
                }
            }
        }
        body = format!(
            "{} {} & x >= {} & x <= {} & y >= {} & y <= {} , ",
            app.world.lat_w,
            app.world.lat_h,
            world::fmt_num(xlo),
            world::fmt_num(xhi),
            world::fmt_num(ylo),
            world::fmt_num(yhi)
        )
        .into_bytes();
    } else {
        // no lattice: positioned entities address through their pos pair fields
        let Some(pi) = app.world.pos() else {
            app.say("nothing to select here");
            return;
        };
        let c0 = app.drag_c0.min(app.drag_c1);
        let c1 = app.drag_c0.max(app.drag_c1);
        let name = &app.world.ents[pi].name;
        let nm = &name[..name.len().min(100)]; // the C's %.100s
        let mut b: Vec<u8> = Vec::new();
        b.extend_from_slice(nm);
        b.extend_from_slice(format!(".x >= {} & ", c0).as_bytes());
        b.extend_from_slice(nm);
        b.extend_from_slice(format!(".x <= {} & ", c1).as_bytes());
        b.extend_from_slice(nm);
        b.extend_from_slice(format!(".y >= {} & ", r0).as_bytes());
        b.extend_from_slice(nm);
        b.extend_from_slice(format!(".y <= {} , ", r1).as_bytes());
        body = b;
    }
    app.prompt = body;
    app.prompt.truncate(1023);
    app.pcur = app.prompt.len();
    app.focus = Focus::Prompt;
    app.say("selection painted — finish the effect");
}

// Executable of that name on $PATH (a name with a slash checks directly). kore.c.
fn path_has(cmd: &str) -> bool {
    if cmd.contains('/') {
        return sys::access_x(cmd);
    }
    let Ok(p) = std::env::var("PATH") else { return false };
    for dir in p.split(':') {
        if dir.is_empty() {
            continue;
        }
        if sys::access_x(&format!("{}/{}", dir, cmd)) {
            return true;
        }
    }
    false
}

// $VISUAL, else $EDITOR, else the first of nvim vim micro nano vi on PATH (else vi).
pub fn editor_pick() -> String {
    if let Ok(ed) = std::env::var("VISUAL") {
        if !ed.is_empty() {
            return ed;
        }
    }
    if let Ok(ed) = std::env::var("EDITOR") {
        if !ed.is_empty() {
            return ed;
        }
    }
    for cand in ["nvim", "vim", "micro", "nano", "vi"] {
        if path_has(cand) {
            return cand.to_string();
        }
    }
    "vi".to_string()
}

// E: guard (world_guard under world focus / no demo, else code_guard), term leave,
// sys::spawn_wait, term re-enter + size, reload whichever file was edited (`reloaded %s`).
// kore.c editor_hop.
pub fn editor_hop(app: &mut App, t: &mut Term) {
    let ed = editor_pick();
    // the hop is a mutating act: corpus files guard into their play copies first
    let mut file = String::new();
    let mut world_file = false;
    if app.world.loaded && (app.focus == Focus::World || app.demo_path.is_empty()) {
        if !world::world_guard(app) {
            return;
        }
        file = app.world.path.clone();
        world_file = true;
    } else if !app.demo_path.is_empty() {
        if !world::code_guard(app) {
            return;
        }
        file = app.demo_live.clone();
    }
    if file.is_empty() {
        app.say("nothing to edit");
        return;
    }
    sys::term_leave();
    // SIGINT/SIGTERM park inside spawn_wait: the editor owns ^C, kore survives underneath
    sys::spawn_wait(&ed, &file);
    let _ = t.enter();
    if world_file && app.world.loaded {
        let path = app.world.path.clone();
        match world::world_load(&path) {
            Ok(w) => {
                app.world = w;
                let msg = format!("reloaded {}", path);
                app.say(&msg);
            }
            Err(err) => app.sayerr(&err),
        }
    } else if !app.demo_path.is_empty() {
        let live = app.demo_live.clone();
        code_load(app, &live);
        let msg = format!("reloaded {}", live);
        app.say(&msg);
    }
}
