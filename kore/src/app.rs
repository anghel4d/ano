// app.rs — the shared application state: kore.c's `static struct App A` (l.1048) as an owned
// struct, plus the OUTPUTS store ops and the say/log helpers. COMPLETE — no porter owns this
// file; every module takes &mut App. Text policy: file content and world-model text (lines,
// names, syms, code, prompt, labels, values, the history log) are Vec<u8> — .reg/.ano bytes are
// byte-transparent; paths and verdicts are String, byte fields entering a message go through
// String::from_utf8_lossy (the corpus is valid UTF-8; the deviation exists only on malformed input).
// Defaults mirror the C's zeroed static: everything zero/empty/false, sess_ja included.

use crate::term::Rect;
use crate::world::World;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    #[default]
    Rail,
    Code,
    World,
    Outputs,
    Out,
    Prompt,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Rail,
    Demo,
    Reg,
}

pub const KMAXDEMO: usize = 1024; // rail entries past the cap are dropped
pub const KMAXHIST: usize = 128; // prompt history; overflow silently dropped
pub const KMAXQREC: usize = 256; // OUTPUTS records; whole oldest groups drop past it
pub const KCUNDO: usize = 64; // code undo snapshots; overflow drops the oldest
pub const KMAXSDEF: usize = 64; // session defs; extras silently dropped
pub const KMAXVROW: usize = 32768; // flattened world surface rows (guard at 32760)

// One labeled query result; a run's records group under the step it staged.
pub struct QRec {
    pub label: Vec<u8>,
    pub value: Vec<u8>,
}

pub struct QGroup {
    pub step: i32,
    pub recs: Vec<QRec>,
}

// One session def: its full text, surface, and def-head name (exact bytes, no fold).
pub struct SDef {
    pub text: Vec<u8>,
    pub ja: bool,
    pub name: Vec<u8>,
}

// One code-undo snapshot: the whole buffer plus the cursor (kore.c cundo, l.2113).
pub struct CodeSnap {
    pub code: Vec<Vec<u8>>,
    pub ccx: i32,
    pub ccy: i32,
}

// A display column of the entity table: index into world.ents + rendered width [3,24].
#[derive(Clone, Copy)]
pub struct DCol {
    pub ent: usize,
    pub width: i32,
}

// One flattened world-surface row. kind: 0 table row, 1 field title, 2 field row, 3 blank.
#[derive(Clone, Copy)]
pub struct VRow {
    pub kind: i32,
    pub seg: i32,
    pub row: i32,
}

#[derive(Default)]
pub struct App {
    pub mode: Mode,
    pub focus: Focus,
    pub world: World,
    pub world_is_copy: bool, // the world is the demo's .kore/play scratch
    pub pristine: String,    // the demo's own registry (never mutated); "" = registry-less
    pub demo_path: String,   // the demo's identity: tags, sessions, anchors key off it
    pub demo_live: String,   // the file backing the code buffer — corpus demo until first save, play copy after
    pub world_orig: String,  // MODE_REG: the corpus .reg the play copy shadows
    pub confirm_reset: bool, // >reset armed: y wipes the play tree, any other key cancels
    pub code: Vec<Vec<u8>>,
    pub code_dirty: bool,
    pub code_insert: bool,
    pub ccy: i32,
    pub ccx: i32, // display column, not a byte offset
    pub code_top: i32,
    pub code_pending: i32, // vim count accumulator
    pub code_g: i32,       // gg chord: count stored on the first g, 0 = unarmed
    pub code_d: i32,       // dd chord likewise
    pub search: Vec<u8>,   // cap 127 bytes
    pub searching: bool,
    pub cundo: Vec<CodeSnap>,
    pub rail_sel: i32,
    pub rail_top: i32,
    pub demo_list: Vec<String>,
    pub space_view: i32, // 0 table, 1 map, 2 bitmap
    pub w_seg: i32,      // world cursor: segment 0 = entity table, 1.. fields
    pub w_row: i32,
    pub w_col: i32,
    pub w_top: i32,
    pub editing: bool,
    pub edit_buf: Vec<u8>, // cap 511 bytes
    pub spc_ret: i32,      // a space-view entity edit parked the cell cursor here
    pub spc_row: i32,
    pub spc_col: i32,
    pub out_log: Vec<u8>, // the history feed, raw bytes
    pub out_scroll: i32,  // lines scrolled back from the tail
    pub qgroups: Vec<QGroup>,
    pub outputs_scroll: i32, // lines scrolled down from the newest tick
    pub verdict: Vec<u8>,    // cap 511 bytes (vsnprintf into char[512])
    pub verdict_bad: bool,
    pub prompt: Vec<u8>, // cap 1023 bytes
    pub pcur: usize,     // byte cursor into prompt
    pub pscroll: usize,  // byte offset: the cursor line's horizontal scroll
    pub ptop: i32,       // line scroll of a tall prompt body
    pub hist: Vec<Vec<u8>>,
    pub hist_at: i32,
    pub dragging: bool,
    pub drag_seg: i32, // 0 table rows, 1 space cells
    pub drag_r0: i32,
    pub drag_c0: i32,
    pub drag_r1: i32,
    pub drag_c1: i32,
    pub undo_seq: i32,
    pub trace: bool, // t: pass --trace to anoc; 0x1F lines land in history, never OUTPUTS
    pub sess_ja: i32, // the session log's surface; -1 unknown
    pub sdefs: Vec<SDef>,
    pub quit: bool,
    pub rail_r: Rect,
    pub code_r: Rect,
    pub world_r: Rect,
    pub outputs_r: Rect,
    pub out_r: Rect,
    pub prompt_r: Rect,
    // relocated C statics:
    pub dcols: Vec<DCol>,          // table_cols' dcols/ndcols (l.2396)
    pub vrows: Vec<VRow>,          // world_vrows' vrows/nvrows (l.2979)
    pub run_lines: Vec<Vec<u8>>,   // the composed program for 0x1D tag resolution (l.1107)
    pub snap_seq: i32,             // per-process snapshot counter, deliberately not disk-scanned
    pub anoc_path: Option<String>, // find_anoc's cache
}

impl App {
    pub fn new() -> App {
        App::default()
    }

    // The verdict line, good hue. Truncates at 511 bytes like the C's vsnprintf into char[512].
    pub fn say(&mut self, msg: &str) {
        self.verdict = msg.as_bytes().iter().copied().take(511).collect();
        self.verdict_bad = false;
    }

    // say, but the verdict draws in the error hue.
    pub fn sayerr(&mut self, msg: &str) {
        self.verdict = msg.as_bytes().iter().copied().take(511).collect();
        self.verdict_bad = true;
    }

    // Append to the history feed; snaps the view back to the tail (kore.c logOut, l.1099).
    pub fn log(&mut self, s: &[u8]) {
        self.out_log.extend_from_slice(s);
        self.out_scroll = 0;
    }

    pub fn outputs_clear(&mut self) {
        self.qgroups.clear();
        self.outputs_scroll = 0;
    }

    // u's inverse of a push: every group whose step lies past the restored one goes (l.1152).
    pub fn outputs_drop_after(&mut self, seq: i32) {
        while self.qgroups.last().is_some_and(|g| g.step > seq) {
            self.qgroups.pop();
        }
        self.outputs_scroll = 0;
    }

    // Whole oldest groups drop while total records exceed the cap; the newest group stays (l.1159).
    pub fn outputs_bound(&mut self) {
        let mut total: usize = self.qgroups.iter().map(|g| g.recs.len()).sum();
        let mut drop = 0;
        while drop < self.qgroups.len().saturating_sub(1) && total > KMAXQREC {
            total -= self.qgroups[drop].recs.len();
            drop += 1;
        }
        if drop > 0 {
            self.qgroups.drain(..drop);
        }
    }

    // Display lines the OUTPUTS panel holds: a seam per group, a line per record, plus a
    // multi-line value's own lines indented beneath its label (l.1175).
    pub fn outputs_total_lines(&self) -> i32 {
        let mut t: i32 = 0;
        for g in &self.qgroups {
            t += 1 + g.recs.len() as i32;
            for r in &g.recs {
                let nl = r.value.iter().filter(|&&b| b == b'\n').count();
                if nl > 0 {
                    t += nl as i32 + 1;
                }
            }
        }
        t
    }

    // The composed program (next.ano / repl.ano) as lines, kept for the run's duration (l.1110).
    pub fn run_lines_set(&mut self, text: &[u8]) {
        if text.is_empty() {
            self.run_lines = Vec::new();
            return;
        }
        self.run_lines = text.split(|&b| b == b'\n').map(|l| l.to_vec()).collect();
        // C's splitter drops the empty tail after a final newline.
        if text.last() == Some(&b'\n') {
            self.run_lines.pop();
        }
    }

    // The 1-based program line, leading spaces/tabs trimmed; None out of range (l.1130).
    pub fn run_line(&self, ln: i32) -> Option<&[u8]> {
        if ln < 1 || ln as usize > self.run_lines.len() {
            return None;
        }
        let l = &self.run_lines[ln as usize - 1];
        let start = l.iter().position(|&b| b != b' ' && b != b'\t').unwrap_or(l.len());
        Some(&l[start..])
    }
}
