// World model and mutation: .reg loading, lookups, number spelling, splices, .kore scratch,
// undo, snapshots, session logs, REPL/demo ticks, --check, and --edit. Guard diagnostics are
// pinned by native Rust tests. Child spawning and the 0x1D/0x1F demux live in main.rs
// (crate::find_steel, crate::cap_split); sys:: provides strtod_prefix/fmt_g for wnum/fmt_num.

use crate::app::{App, DCol, KMAXHIST, KMAXSDEF, Mode, SDef};
use crate::sys;
use crate::term::{C_AT, C_CASELO, C_CASEUP, Rect, Term};
use crate::text;
use std::io::Write;
use steel::alias::{AliasEnvironment, sidecar_path};

pub const KMAXENT: usize = 512; // data lines past the cap parse to no Ent but stay in lines[]

// Entity archetype ink: arch_color's FNV-1a (case-folded) % 12.
pub const ENT_PAL: [u8; 12] = [114, 183, 210, 117, 221, 80, 213, 147, 84, 173, 152, 229];
// Field ground paint: declaration ordinal % 10, x/y counted.
pub const FIELD_PAL: [u8; 10] = [39, 208, 170, 114, 221, 80, 213, 147, 210, 84];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EKind {
    Col,
    Field,
    Pres,
    Rel,
    Srel,
    Alias, // a static alias mask: a stored mask VALUE, not a spelling alias and not the dynamic ^name overlay
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VType {
    Num,
    Bool,
    Nat,
    Int,
    Sym,
    Char,
    Vec,
}

// 2^53, the nat/int ceiling — steel's ANO_NATMAX, the contiguous-integer bound.
pub const NATMAX: f64 = 9_007_199_254_740_992.0;

// One parsed entry. keyw is the data word-index delta: +1 keyed
// rel/srel (`rel <key> <name> …`), -1 `unique` (no type word) — splice targets shift by it.
pub struct Ent {
    pub kind: EKind,
    pub vtype: VType,
    pub name: Vec<u8>,
    pub line: usize,    // index into World.lines — the splice target
    pub nums: Vec<f64>, // num/bool/rel/pres/alias values; vec flattened pairs
    pub syms: Vec<Vec<u8>>,
    pub chars: Vec<u8>,      // char payload copy
    pub fib_off: Vec<usize>, // srel fibers into fib_vals
    pub fib_len: Vec<usize>,
    pub fib_vals: Vec<f64>,
    pub is_inv: bool, // srel spelled `inv` — fibers derived, never edited
    pub inv: Vec<u8>, // the rel an inv derives from
    pub keyw: i32,
    pub is_uniq: bool, // a `unique` line — pairwise-distinct at load, whatever its kind word
    pub rng: Option<(f64, f64)>, // declared `range <col> <lo> <hi>` bounds — edits clamp into it
}

// The parsed world. lines[] verbatim is the one source of truth;
// the tables are a view. One load replaces the last whole (Rust ownership is the arena).
#[derive(Default)]
pub struct World {
    pub path: String,
    pub lines: Vec<Vec<u8>>,
    pub n: i32,
    pub lat_w: i32,
    pub lat_h: i32,
    pub ents: Vec<Ent>,
    pub pos_col: Vec<u8>, // role pos target, else empty (literal `pos` fallback)
    pub glyph_col: Vec<u8>, // role glyph target, else empty (literal `glyph`)
    pub proto_col: Vec<u8>, // role proto target, else empty (literal `proto`)
    pub loaded: bool,
}

// ---------- small shared helpers ----------

// A zeroed C Ent: kind/type discriminant 0, everything else empty.
fn ent_new(kind: EKind, vtype: VType, line: usize) -> Ent {
    Ent {
        kind,
        vtype,
        name: Vec::new(),
        line,
        nums: Vec::new(),
        syms: Vec::new(),
        chars: Vec::new(),
        fib_off: Vec::new(),
        fib_len: Vec::new(),
        fib_vals: Vec::new(),
        is_inv: false,
        inv: Vec::new(),
        keyw: 0,
        is_uniq: false,
        rng: None,
    }
}

// C-string view: everything up to the first NUL (the C reader parses NUL-terminated).
fn cstr(b: &[u8]) -> &[u8] {
    &b[..b.iter().position(|&x| x == 0).unwrap_or(b.len())]
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle)
}

// C snprintf %.<n>s truncation, then lossy for message text.
fn trunc_lossy(b: &[u8], n: usize) -> String {
    String::from_utf8_lossy(&b[..b.len().min(n)]).into_owned()
}

fn trunc_str(s: &str, n: usize) -> String {
    trunc_lossy(s.as_bytes(), n)
}

// C atoi: optional whitespace, optional sign, digit prefix, 0 on garbage.
fn atoi(s: &[u8]) -> i32 {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'-' || s[i] == b'+') {
        neg = s[i] == b'-';
        i += 1;
    }
    let mut v: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        v = v.saturating_mul(10).saturating_add((s[i] - b'0') as i64);
        i += 1;
    }
    if neg {
        v = -v;
    }
    v.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

fn fold(c: u8) -> u8 {
    if c.is_ascii_uppercase() { c + 32 } else { c }
}

// ---------- helpers: tokenizing, numbers, names ----------

// The loader's case contract: ASCII A-Z folds, every other byte exact.
pub fn names_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(&x, &y)| fold(x) == fold(y))
}

// Byte span (off, len) of the 0-based idx-th word; None when fewer words. Words split on
// space/tab; a word-boundary '#' ends the data.
pub fn word_span(line: &[u8], idx: i32) -> Option<(usize, usize)> {
    let mut p = 0usize;
    let mut bow = true;
    let mut i: i32 = 0;
    while p < line.len() {
        while p < line.len() && (line[p] == b' ' || line[p] == b'\t') {
            p += 1;
            bow = true;
        }
        if p >= line.len() || (bow && line[p] == b'#') {
            return None;
        }
        let w = p;
        while p < line.len() && line[p] != b' ' && line[p] != b'\t' {
            p += 1;
        }
        if i == idx {
            return Some((w, p - w));
        }
        i += 1;
        bow = false;
    }
    None
}

// Tokenize a comment-stripped copy: the C split_words. Owned words, same boundaries.
pub fn split_words(line: &[u8]) -> Vec<Vec<u8>> {
    let mut end = line.len();
    let mut bow = true;
    for (i, &b) in line.iter().enumerate() {
        if b == b'#' && bow {
            end = i;
            break;
        }
        bow = b == b' ' || b == b'\t';
    }
    let mut words = Vec::new();
    let mut p = 0usize;
    while p < end {
        while p < end && (line[p] == b' ' || line[p] == b'\t') {
            p += 1;
        }
        if p >= end {
            break;
        }
        let w = p;
        while p < end && line[p] != b' ' && line[p] != b'\t' {
            p += 1;
        }
        words.push(line[w..p].to_vec());
    }
    words
}

// The glyph run's byte span on a col/field char line: from word index 3 to the
// word-boundary '#' or line end, trailing space/tab/\r trimmed; None when word 3 absent.
pub fn char_span(line: &[u8]) -> Option<(usize, usize)> {
    let (o, _l) = word_span(line, 3)?;
    let mut end = o;
    let mut bow = true;
    for i in o..line.len() {
        if line[i] == b'#' && bow {
            break;
        }
        bow = line[i] == b' ' || line[i] == b'\t';
        end = i + 1;
    }
    while end > o && (line[end - 1] == b' ' || line[end - 1] == b'\t' || line[end - 1] == b'\r') {
        end -= 1;
    }
    Some((o, end - o))
}

// The loader's representability rules for a glyph run: length exactly rows, rows > 0,
// first byte not space/tab/'#', last byte not space/tab, no '#' preceded by space/tab.
pub fn run_ok(run: &[u8], rows: i32) -> bool {
    if rows <= 0 || run.len() as i32 != rows {
        return false;
    }
    let n = run.len();
    if run[0] == b' ' || run[0] == b'\t' || run[0] == b'#' {
        return false;
    }
    if run[n - 1] == b' ' || run[n - 1] == b'\t' {
        return false;
    }
    for j in 1..n {
        if run[j] == b'#' && (run[j - 1] == b' ' || run[j - 1] == b'\t') {
            return false;
        }
    }
    true
}

// Parse one fully consumed extended-real word: finite or signed infinity, never NaN.
pub fn wnum(w: &[u8]) -> Option<f64> {
    match w {
        b"\xe2\x88\x9e" | b"+\xe2\x88\x9e" => return Some(f64::INFINITY),
        b"-\xe2\x88\x9e" | b"\xc2\xaf\xe2\x88\x9e" => return Some(f64::NEG_INFINITY),
        _ => {}
    }
    let (v, used) = sys::strtod_prefix(w);
    if used > 0 && used == w.len() && !v.is_nan() {
        Some(v)
    } else {
        None
    }
}

// integer in [-9e15, 9e15] -> %lld; else the shortest %.{1..17}g
// that round-trips through strtod (sys::fmt_g / sys::strtod_prefix). Owned String replaces
// the C's 8-slot static ring; the OUTPUT bytes must match.
pub fn fmt_num(v: f64) -> String {
    if v == (v as i64) as f64 && v >= -9e15 && v <= 9e15 {
        return sys::fmt_lld(v as i64);
    }
    let mut b = String::new();
    for p in 1..=17 {
        b = sys::fmt_g(v, p);
        let (rv, _) = sys::strtod_prefix(b.as_bytes());
        if rv == v {
            break;
        }
    }
    b
}

// ---------- the .reg reader ----------

// KNAMESZ-1 truncation: names clip at 255 bytes.
fn name_trunc(w: &[u8]) -> Vec<u8> {
    w[..w.len().min(255)].to_vec()
}

// The pairwise-distinct check steel's loader runs on `unique` lines (keyw -1 marks them).
fn world_check_unique(w: &World) -> Result<(), String> {
    for e in &w.ents {
        if e.kind != EKind::Col || !e.is_uniq {
            continue;
        }
        for x in 0..e.nums.len() {
            for y in x + 1..e.nums.len() {
                if e.nums[x] == e.nums[y] {
                    return Err(format!(
                        "line {}: unique {}: value {} repeats (rows {}, {})",
                        e.line + 1,
                        String::from_utf8_lossy(&e.name),
                        sys::fmt_g(e.nums[x], 6),
                        x,
                        y
                    ));
                }
            }
        }
    }
    Ok(())
}

// Parse path into a fresh World. Err(message) on unreadable file
// (`cannot read <path>: <strerror>`) or a failed unique check (`line <n>: unique <name>:
// value <g> repeats (rows <i>, <j>)`); the caller replaces its world only on Ok. Line split:
// \n, one trailing \r stripped, post-final-newline empty tail is not a line EXCEPT an empty
// file yields one empty line. Dispatch covers n, lattice, col/field, pres, unique, rel/alias
// (keyed when the third word is non-numeric and not `|`),
// srel/inv, and role pos/glyph/proto; everything else remains in lines[].
pub fn world_load(path: &str) -> Result<World, String> {
    crate::registry_tx::validate(path)?;
    let raw = match std::fs::File::open(path) {
        Ok(mut f) => {
            use std::io::Read;
            let mut b = Vec::new();
            // fread failure reads as far as it got, like the C's got-byte NUL cap
            let _ = f.read_to_end(&mut b);
            b
        }
        Err(_) => return Err(format!("cannot read {}: {}", path, sys::errno_str())),
    };
    let buf = cstr(&raw);
    let mut w = World {
        path: path.to_string(),
        ..World::default()
    };
    let mut p = 0usize;
    loop {
        let nl = buf[p..].iter().position(|&b| b == b'\n').map(|k| p + k);
        let end = nl.unwrap_or(buf.len());
        let mut ln = &buf[p..end];
        // the empty tail after a final newline is not a line
        if nl.is_none() && ln.is_empty() && !w.lines.is_empty() {
            break;
        }
        if ln.last() == Some(&b'\r') {
            ln = &ln[..ln.len() - 1];
        }
        w.lines.push(ln.to_vec());
        match nl {
            Some(k) => p = k + 1,
            None => break,
        }
    }
    for li in 0..w.lines.len() {
        let words = split_words(&w.lines[li]);
        let nw = words.len();
        if nw == 0 {
            continue;
        }
        let k: &[u8] = &words[0];
        let can = w.ents.len() < KMAXENT;
        if k == b"n" && nw == 2 {
            if let Some(d) = wnum(&words[1]) {
                w.n = if d.is_finite()
                    && d >= 0.0
                    && d <= 1e6
                    && d.fract() == 0.0
                {
                    d as i32
                } else {
                    0
                };
            }
        } else if k == b"lattice" && nw == 3 {
            if let (Some(d), Some(h)) = (wnum(&words[1]), wnum(&words[2])) {
                if d.is_finite()
                    && h.is_finite()
                    && d >= 0.0
                    && d <= 4096.0
                    && h >= 0.0
                    && h <= 4096.0
                    && d.fract() == 0.0
                    && h.fract() == 0.0
                {
                    w.lat_w = d as i32;
                    w.lat_h = h as i32;
                }
            }
        } else if (k == b"col" || k == b"field") && nw >= 3 && can {
            let kind = if k[0] == b'f' {
                EKind::Field
            } else {
                EKind::Col
            };
            let ty: &[u8] = &words[2];
            if ty == b"num" || ty == b"bool" || ty == b"nat" || ty == b"int" || ty == b"vec" {
                let vt = if ty == b"bool" {
                    VType::Bool
                } else if ty == b"nat" {
                    VType::Nat
                } else if ty == b"int" {
                    VType::Int
                } else if ty == b"vec" {
                    VType::Vec
                } else {
                    VType::Num
                };
                let mut e = ent_new(kind, vt, li);
                e.name = name_trunc(&words[1]);
                for j in 3..nw {
                    if words[j] == b"|" {
                        continue;
                    }
                    if let Some(v) = wnum(&words[j]) {
                        e.nums.push(v);
                    }
                }
                w.ents.push(e);
            } else if ty == b"sym" {
                let mut e = ent_new(kind, VType::Sym, li);
                e.name = name_trunc(&words[1]);
                for j in 3..nw {
                    e.syms.push(words[j].clone());
                }
                w.ents.push(e);
            } else if ty == b"char" {
                let mut e = ent_new(kind, VType::Char, li);
                e.name = name_trunc(&words[1]);
                // the run is the raw tail from the 4th word — the loader's own read
                if let Some((off, len)) = char_span(&w.lines[li]) {
                    e.chars = w.lines[li][off..off + len].to_vec();
                }
                w.ents.push(e);
            }
            // any other type word: not an entry, verbatim pass-through
        } else if k == b"pres" && nw >= 2 && can {
            let mut e = ent_new(EKind::Pres, VType::Bool, li);
            e.name = name_trunc(&words[1]);
            for j in 2..nw {
                if let Some(v) = wnum(&words[j]) {
                    e.nums.push(v);
                }
            }
            w.ents.push(e);
        } else if k == b"unique" && nw >= 2 && can {
            // declared injectivity; bare = a num column with no type word (data starts one
            // word early, keyw -1). Constraints stack: an optional kind word refines the
            // carrier (`unique id nat`) and shifts the data to the col layout (keyw 0).
            let mut e = ent_new(EKind::Col, VType::Num, li);
            e.is_uniq = true;
            e.keyw = -1;
            e.name = name_trunc(&words[1]);
            let mut di = 2;
            if nw >= 3 && wnum(&words[2]).is_none() {
                e.vtype = if words[2] == b"nat" {
                    VType::Nat
                } else if words[2] == b"int" {
                    VType::Int
                } else {
                    VType::Num
                };
                e.keyw = 0;
                di = 3;
            }
            for j in di..nw {
                if let Some(v) = wnum(&words[j]) {
                    e.nums.push(v);
                }
            }
            w.ents.push(e);
        } else if (k == b"rel" || k == b"alias") && nw >= 2 && can {
            // a static alias mask is a stored mask VALUE — data, so it displays and edits like a rel;
            // a keyed rel (`rel id mentor …`) carries its key column as one extra name
            let isr = k[0] == b'r';
            let keyed = isr && nw >= 3 && wnum(&words[2]).is_none() && words[2] != b"|";
            let keyw: i32 = if keyed { 1 } else { 0 };
            let mut e = ent_new(
                if isr { EKind::Rel } else { EKind::Alias },
                if isr { VType::Num } else { VType::Bool },
                li,
            );
            e.keyw = keyw;
            e.name = name_trunc(&words[(1 + keyw) as usize]);
            for j in (2 + keyw) as usize..nw {
                if let Some(v) = wnum(&words[j]) {
                    e.nums.push(v);
                }
            }
            w.ents.push(e);
        } else if (k == b"srel" || k == b"inv") && nw >= 2 && can {
            let iss = k[0] == b's';
            let keyed = iss && nw >= 3 && wnum(&words[2]).is_none() && words[2] != b"|";
            let keyw: i32 = if keyed { 1 } else { 0 };
            let mut e = ent_new(EKind::Srel, VType::Num, li);
            e.keyw = keyw;
            e.is_inv = k[0] == b'i';
            if e.is_inv && nw >= 3 {
                e.inv = name_trunc(&words[2]);
            }
            e.name = name_trunc(&words[(1 + keyw) as usize]);
            if !e.is_inv {
                e.fib_off.push(0);
                for j in (2 + keyw) as usize..nw {
                    if words[j] == b"|" {
                        let last = *e.fib_off.last().unwrap();
                        e.fib_len.push(e.fib_vals.len() - last);
                        e.fib_off.push(e.fib_vals.len());
                    } else if let Some(v) = wnum(&words[j]) {
                        e.fib_vals.push(v);
                    }
                }
                let last = *e.fib_off.last().unwrap();
                e.fib_len.push(e.fib_vals.len() - last);
            }
            w.ents.push(e);
        } else if k == b"range" && nw == 4 {
            // declared bounds rider: attach to its column (which precedes it, forward-only);
            // kore stays lenient — steel is the sealer, the editor only clamps toward it
            if let (Some(lo), Some(hi)) = (wnum(&words[2]), wnum(&words[3])) {
                if lo <= hi {
                    let name = name_trunc(&words[1]);
                    for e in w.ents.iter_mut().rev() {
                        if (e.kind == EKind::Col || e.kind == EKind::Field)
                            && names_eq(&e.name, &name)
                        {
                            e.rng = Some((lo, hi));
                            break;
                        }
                    }
                }
            }
        } else if k == b"role" && nw == 3 && words[1] == b"pos" {
            w.pos_col = name_trunc(&words[2]);
        } else if k == b"role" && nw == 3 && words[1] == b"glyph" {
            w.glyph_col = name_trunc(&words[2]);
        } else if k == b"role" && nw == 3 && words[1] == b"proto" {
            w.proto_col = name_trunc(&words[2]);
        }
        // everything else: schema, preserved verbatim in lines[]
    }
    world_check_unique(&w)?;
    w.loaded = true;
    Ok(w)
}

impl World {
    // First entry of kind whose name matches under names_eq.
    pub fn ent(&self, name: &[u8], kind: EKind) -> Option<usize> {
        self.ents
            .iter()
            .position(|e| e.kind == kind && names_eq(&e.name, name))
    }

    // The pos-role column when it resolves to a V_VEC E_COL, else literal `pos` when V_VEC.
    pub fn pos(&self) -> Option<usize> {
        if !self.pos_col.is_empty() {
            if let Some(i) = self.ent(&self.pos_col, EKind::Col) {
                if self.ents[i].vtype == VType::Vec {
                    return Some(i);
                }
            }
        }
        let i = self.ent(b"pos", EKind::Col)?;
        if self.ents[i].vtype == VType::Vec {
            Some(i)
        } else {
            None
        }
    }

    // Role name first (must be V_SYM E_COL), else the literal name.
    pub fn sym_col(&self, role: &[u8], lit: &[u8]) -> Option<usize> {
        if !role.is_empty() {
            if let Some(i) = self.ent(role, EKind::Col) {
                if self.ents[i].vtype == VType::Sym {
                    return Some(i);
                }
            }
        }
        let i = self.ent(lit, EKind::Col)?;
        if self.ents[i].vtype == VType::Sym {
            Some(i)
        } else {
            None
        }
    }

    pub fn glyph_col_ent(&self) -> Option<usize> {
        self.sym_col(&self.glyph_col, b"glyph")
    }

    pub fn proto_col_ent(&self) -> Option<usize> {
        self.sym_col(&self.proto_col, b"proto")
    }

    // The entity's glyph, at most 7 bytes: the glyph column's sym clamped to its FIRST rune;
    // empty/absent -> the proto sym only when the noun IS a single rune; else None (the
    // bold-@ fallback belongs to the call sites).
    pub fn ent_glyph(&self, row: i32) -> Option<Vec<u8>> {
        let sym_at = |i: Option<usize>| -> Option<&[u8]> {
            let i = i?;
            let e = &self.ents[i];
            if row >= 0 && (row as usize) < e.syms.len() {
                Some(&e.syms[row as usize])
            } else {
                None
            }
        };
        let mut s = sym_at(self.glyph_col_ent()).filter(|x| !x.is_empty());
        if s.is_none() {
            let p = sym_at(self.proto_col_ent()).filter(|x| !x.is_empty())?;
            let mut q = 0usize;
            text::u8next(p, &mut q);
            if q < p.len() {
                return None; // a multi-rune noun is a name, not a glyph
            }
            s = Some(p);
        }
        let s = s.unwrap();
        let mut i = 0usize;
        text::u8next(s, &mut i);
        Some(s[..i.min(7)].to_vec())
    }

    // Ink: single-byte ASCII-cased glyph takes C_CASEUP/C_CASELO (case carries side); else
    // the first E_COL V_BOOL with nums[row] != 0 hashes via arch_color; else C_AT.
    pub fn ent_color(&self, row: i32, glyph: &[u8]) -> u8 {
        if glyph.len() == 1 {
            if glyph[0].is_ascii_uppercase() {
                return C_CASEUP;
            }
            if glyph[0].is_ascii_lowercase() {
                return C_CASELO;
            }
        }
        for e in &self.ents {
            if e.kind == EKind::Col
                && e.vtype == VType::Bool
                && row >= 0
                && (row as usize) < e.nums.len()
                && e.nums[row as usize] != 0.0
            {
                return arch_color(&e.name);
            }
        }
        C_AT
    }
}

// FNV-1a 32-bit (offset 2166136261, prime 16777619) over the ASCII-case-folded name,
// into ENT_PAL[h % 12].
pub fn arch_color(name: &[u8]) -> u8 {
    let mut h: u32 = 2166136261;
    for &b in name {
        h = (h ^ fold(b) as u32).wrapping_mul(16777619);
    }
    ENT_PAL[(h % 12) as usize]
}

// A char cell's display: printable ASCII 0x20..0x7E verbatim, anything else "·",
// out-of-range " ".
pub fn glyph_at(e: &Ent, k: i32) -> Vec<u8> {
    if k < 0 || k as usize >= e.chars.len() {
        return b" ".to_vec();
    }
    let c = e.chars[k as usize];
    if c < 0x20 || c >= 0x7F {
        return "\u{b7}".as_bytes().to_vec();
    }
    vec![c]
}

// ---------- segments, display columns, cell text ----------

// Rebuild app.dcols from non-E_FIELD ents in declaration order; width = max(name width,
// every cell width) clamped [3, 24]. Width-pass spellings: srel space-joined / `(inv)`,
// vec `x,y` (comma — display uses space, same width).
pub fn table_cols(app: &mut App) {
    let mut dc: Vec<DCol> = Vec::new();
    for (i, e) in app.world.ents.iter().enumerate() {
        if e.kind == EKind::Field {
            continue;
        }
        let mut w = text::swidth(&e.name);
        let rows = if e.kind == EKind::Srel {
            e.fib_off.len()
        } else if e.vtype == VType::Vec {
            e.nums.len() / 2
        } else {
            e.nums.len()
        };
        for r in 0..rows {
            let mut cell: Vec<u8> = Vec::new();
            if e.kind == EKind::Srel && !e.is_inv {
                for j in 0..e.fib_len[r] {
                    if cell.len() >= 100 {
                        break;
                    }
                    if j > 0 {
                        cell.push(b' ');
                    }
                    cell.extend(fmt_num(e.fib_vals[e.fib_off[r] + j]).into_bytes());
                }
            } else if e.kind == EKind::Srel {
                cell = b"(inv)".to_vec();
            } else if e.vtype == VType::Vec {
                cell = format!("{},{}", fmt_num(e.nums[2 * r]), fmt_num(e.nums[2 * r + 1]))
                    .into_bytes();
            } else if e.vtype == VType::Sym {
                if r < e.syms.len() {
                    cell = e.syms[r].clone();
                }
            } else if e.vtype == VType::Char {
                cell = glyph_at(e, r as i32);
            } else if r < e.nums.len() {
                cell = fmt_num(e.nums[r]).into_bytes();
            }
            let cwd = text::swidth(&cell);
            if cwd > w {
                w = cwd;
            }
        }
        if w > 24 {
            w = 24;
        }
        if w < 3 {
            w = 3;
        }
        dc.push(DCol { ent: i, width: w });
    }
    app.dcols = dc;
}

// The display/edit spelling of segment-0 cell (row, col): srel fibers space-joined, inv
// `(inv <name>)`, vec `x y` (space), sym verbatim, char glyph_at, rel < 0 -> `/`, else
// fmt_num.
pub fn table_cell(app: &App, row: i32, col: i32) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    if col < 0 || col as usize >= app.dcols.len() {
        return out;
    }
    let e = &app.world.ents[app.dcols[col as usize].ent];
    let ru = row as usize;
    if e.kind == EKind::Srel && !e.is_inv {
        if row >= 0 && ru < e.fib_off.len() {
            for j in 0..e.fib_len[ru] {
                if j > 0 {
                    out.push(b' ');
                }
                out.extend(fmt_num(e.fib_vals[e.fib_off[ru] + j]).into_bytes());
            }
        }
    } else if e.kind == EKind::Srel {
        out.extend_from_slice(b"(inv ");
        out.extend_from_slice(&e.name);
        out.push(b')');
    } else if e.vtype == VType::Vec {
        if row >= 0 && 2 * ru + 1 < e.nums.len() {
            out = format!(
                "{} {}",
                fmt_num(e.nums[2 * ru]),
                fmt_num(e.nums[2 * ru + 1])
            )
            .into_bytes();
        }
    } else if e.vtype == VType::Sym {
        if row >= 0 && ru < e.syms.len() {
            out = e.syms[ru].clone();
        }
    } else if e.vtype == VType::Char {
        if row >= 0 && ru < e.chars.len() {
            out = glyph_at(e, row);
        }
    } else if row >= 0 && ru < e.nums.len() {
        if e.kind == EKind::Rel && e.nums[ru] < 0.0 {
            out = b"/".to_vec();
        } else {
            out = fmt_num(e.nums[ru]).into_bytes();
        }
    }
    out
}

// The seg-th E_FIELD (segments 1..); None when out of range.
pub fn seg_field(app: &App, seg: i32) -> Option<usize> {
    let mut s = 0;
    for (i, e) in app.world.ents.iter().enumerate() {
        if e.kind == EKind::Field {
            s += 1;
            if s == seg {
                return Some(i);
            }
        }
    }
    None
}

// Segment 0 -> world.n; fields -> latH ? latH : 1.
pub fn seg_rows(app: &App, seg: i32) -> i32 {
    if seg == 0 {
        return app.world.n;
    }
    if app.world.lat_h != 0 {
        app.world.lat_h
    } else {
        1
    }
}

// Segment 0 -> dcols.len(); fields -> latW ? latW : the field's nn.
pub fn seg_cols(app: &App, seg: i32) -> i32 {
    if seg == 0 {
        return app.dcols.len() as i32;
    }
    if app.world.lat_w != 0 {
        app.world.lat_w
    } else {
        seg_field(app, seg)
            .map(|i| app.world.ents[i].nums.len() as i32)
            .unwrap_or(0)
    }
}

// ---------- files and atomicity ----------

// Whole file; None on failure.
pub fn read_file(path: &str) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = Vec::new();
    // a mid-read error keeps what fread got, exactly as the C's got-byte return
    let _ = f.read_to_end(&mut buf);
    Some(buf)
}

// Write and fsync <path>.staged, then rename it over path. Failures remove the staged file.
// The parent directory is not fsynced.
pub fn write_commit(path: &str, data: &[u8]) -> bool {
    let staged = format!("{}.staged", path);
    let wrote = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&staged)?;
        f.write_all(data)?;
        f.sync_all()?;
        Ok(())
    })();
    if wrote.is_err() {
        let _ = std::fs::remove_file(&staged);
        return false;
    }
    if std::fs::rename(&staged, path).is_err() {
        let _ = std::fs::remove_file(&staged);
        return false;
    }
    true
}

pub fn copy_file(from: &str, to: &str) -> bool {
    match read_file(from) {
        Some(d) => write_commit(to, &d),
        None => false,
    }
}

// mkdir every prefix then the whole path, 0755, errors ignored.
pub fn mkdirs(path: &str) {
    let b = path.as_bytes();
    for i in 1..b.len() {
        if b[i] == b'/' {
            let _ = std::fs::create_dir(&path[..i]);
        }
    }
    let _ = std::fs::create_dir(path);
}

// ---------- the splice machinery ----------

// Splice repl over (off, len) of line, rebuild the WHOLE file as every line + "\n"
// (normalizes a missing final newline; CRLF already collapsed at load), write_commit, and
// world_load the same path back into app.world. Errors verbatim: `splice: no line %d`,
// `splice: bad span`, `cannot write %.180s: %s`.
pub fn world_splice(
    app: &mut App,
    line: usize,
    off: usize,
    len: usize,
    repl: &[u8],
) -> Result<(), String> {
    if line >= app.world.lines.len() {
        return Err(format!("splice: no line {}", line));
    }
    let old_len = app.world.lines[line].len();
    if off
        .checked_add(len)
        .map(|end| end > old_len)
        .unwrap_or(true)
    {
        return Err("splice: bad span".to_string());
    }
    let mut lines = app.world.lines.clone();
    lines[line].splice(off..off + len, repl.iter().copied());
    let mut bytes = Vec::new();
    for source_line in &lines {
        bytes.extend_from_slice(source_line);
        bytes.push(b'\n');
    }
    let path = app.world.path.clone();
    crate::registry_tx::publish(&path, &bytes)
        .map_err(|error| format!("cannot write {}: {}", trunc_str(&path, 180), error))?;
    let next = world_load(&path)?;
    app.world = next;
    Ok(())
}

// Guards in order: repl exactly 1 byte in 0x20..0x7E (`char cell wants one printable ASCII
// byte`); char_span (`bad char line`); cell in range and len == rows (`glyph out of
// range`); the mutated run passes run_ok (`that run would have no .reg spelling (boundary
// space or word-boundary '#')`). Then a 1-byte splice.
pub fn char_splice(
    app: &mut App,
    ent: usize,
    cell: i32,
    rows: i32,
    repl: &[u8],
) -> Result<(), String> {
    if repl.len() != 1 || repl[0] < 0x20 || repl[0] >= 0x7F {
        return Err("char cell wants one printable ASCII byte".to_string());
    }
    let line_idx = app.world.ents[ent].line;
    let (off, len) =
        char_span(&app.world.lines[line_idx]).ok_or_else(|| "bad char line".to_string())?;
    if cell < 0 || cell >= len as i32 || len as i32 != rows {
        return Err("glyph out of range".to_string());
    }
    let mut cand = app.world.lines[line_idx][off..off + len].to_vec();
    cand[cell as usize] = repl[0];
    if !run_ok(&cand, rows) {
        return Err(
            "that run would have no .reg spelling (boundary space or word-boundary '#')"
                .to_string(),
        );
    }
    world_splice(app, line_idx, off + cell as usize, 1, repl)
}

// The one splice router, addressed by app.w_seg/w_row/w_col (the same triple --edit takes).
// Field segments splice word 3 + cell (char fields via char_splice with rows = latW*latH);
// segment 0 dispatches by kind: sym one word, vec both pair words re-formatted via fmt_num,
// num word 3 + keyw + row, pres/alias word 2 + row, rel `/`->-1 word
// 2 + keyw + row, srel whole-fiber replace (inv refuses), default `cell not editable`.
// Every error string is part of the native --edit surface. Ok(Some(_)) is the clamp warning:
// the write landed, repaired onto the column's carrier set.
pub fn cell_commit(app: &mut App, text: &[u8]) -> Result<Option<String>, String> {
    let mut repl: Vec<u8> = text.iter().copied().take(511).collect();
    let mut warn: Option<String> = None;
    if app.w_seg > 0 {
        let ei = seg_field(app, app.w_seg).ok_or_else(|| "no field segment".to_string())?;
        let lat_w = app.world.lat_w;
        let cell = app.w_row * (if lat_w != 0 { lat_w } else { 1 }) + app.w_col;
        let line_idx = app.world.ents[ei].line;
        if app.world.ents[ei].vtype == VType::Char {
            return char_splice(app, ei, cell, lat_w * app.world.lat_h, &repl).map(|_| None);
        }
        let Some(v) = wnum(&repl) else {
            return Err(format!("not a number: {}", trunc_lossy(&repl, 100)));
        };
        let (c, w) = clamp_typed(app.world.ents[ei].vtype, app.world.ents[ei].rng, v);
        if w.is_some() {
            repl = fmt_num(c).into_bytes();
            warn = w;
        }
        let (off, len) = word_span(&app.world.lines[line_idx], 3 + cell)
            .ok_or_else(|| "value out of range".to_string())?;
        return world_splice(app, line_idx, off, len, &repl).map(|_| warn);
    }
    if app.w_col < 0 || app.w_col as usize >= app.dcols.len() {
        return Err("no column".to_string());
    }
    let ei = app.dcols[app.w_col as usize].ent;
    let row = app.w_row;
    let (kind, vtype, keyw, line_idx) = {
        let e = &app.world.ents[ei];
        (e.kind, e.vtype, e.keyw, e.line)
    };
    match kind {
        EKind::Col => match vtype {
            VType::Char => char_splice(app, ei, row, app.world.n, &repl).map(|_| None),
            VType::Sym => {
                if repl.is_empty() || repl.contains(&b' ') || repl[0] == b'#' {
                    return Err("sym wants one word".to_string());
                }
                let (off, len) = word_span(&app.world.lines[line_idx], 3 + row)
                    .ok_or_else(|| "row out of range".to_string())?;
                world_splice(app, line_idx, off, len, &repl).map(|_| None)
            }
            VType::Vec => {
                // sscanf "%lf %lf": trailing garbage tolerated, no isfinite guard — preserve
                let (x, y) = scan_two_lf(&repl).ok_or_else(|| "vec cell wants: x y".to_string())?;
                // one splice covering both pair words — the reload frees e, so never two
                let ln = &app.world.lines[line_idx];
                let (o1, _l1, o2, l2) =
                    match (word_span(ln, 3 + 3 * row), word_span(ln, 4 + 3 * row)) {
                        (Some((o1, l1)), Some((o2, l2))) => (o1, l1, o2, l2),
                        _ => return Err("row out of range".to_string()),
                    };
                let pair = format!("{} {}", fmt_num(x), fmt_num(y));
                world_splice(app, line_idx, o1, o2 + l2 - o1, pair.as_bytes()).map(|_| None)
            }
            _ => {
                let Some(v) = wnum(&repl) else {
                    return Err(format!("not a number: {}", trunc_lossy(&repl, 100)));
                };
                // Typed repair applies to every numeric carrier, including unique columns.
                let (c, w) = clamp_typed(vtype, app.world.ents[ei].rng, v);
                if w.is_some() {
                    repl = fmt_num(c).into_bytes();
                    warn = w;
                }
                let (off, len) = word_span(&app.world.lines[line_idx], 3 + keyw + row)
                    .ok_or_else(|| "row out of range".to_string())?;
                world_splice(app, line_idx, off, len, &repl).map(|_| warn)
            }
        },
        EKind::Pres | EKind::Alias => {
            let Some(v) = wnum(&repl) else {
                return Err(format!("not a bit: {}", trunc_lossy(&repl, 100)));
            };
            // a mask is boolean by nature: the same repair as a bool column
            let (c, w) = clamp_typed(VType::Bool, None, v);
            if w.is_some() {
                repl = fmt_num(c).into_bytes();
                warn = w;
            }
            let (off, len) = word_span(&app.world.lines[line_idx], 2 + row)
                .ok_or_else(|| "row out of range".to_string())?;
            world_splice(app, line_idx, off, len, &repl).map(|_| warn)
        }
        EKind::Rel => {
            if repl == b"/" {
                repl = b"-1".to_vec(); // the drawing's none
            }
            if wnum(&repl).is_none() {
                return Err(format!("not a row index: {}", trunc_lossy(&repl, 100)));
            }
            let (off, len) = word_span(&app.world.lines[line_idx], 2 + keyw + row)
                .ok_or_else(|| "row out of range".to_string())?;
            world_splice(app, line_idx, off, len, &repl).map(|_| None)
        }
        EKind::Srel => {
            let (is_inv, inv) = {
                let e = &app.world.ents[ei];
                (e.is_inv, e.inv.clone())
            };
            if is_inv {
                return Err(format!(
                    "inv fibers derive from '{}' — edit the rel",
                    trunc_lossy(&inv, 100)
                ));
            }
            // replace fiber `row` wholesale: every word must parse, or the splice would not reload
            for wv in split_words(&repl).iter().take(128) {
                if wnum(wv).is_none() {
                    return Err(format!("fiber wants numbers: '{}'", trunc_lossy(wv, 60)));
                }
            }
            let ln = app.world.lines[line_idx].clone();
            let mut wi = 2 + keyw;
            let mut fib: i32 = 0;
            let (mut first_w, mut last_w) = (-1i32, -1i32);
            loop {
                let Some((off, len)) = word_span(&ln, wi) else {
                    break;
                };
                let word = &ln[off..off + len];
                if word == b"|" {
                    if fib == row {
                        break;
                    }
                    fib += 1;
                    wi += 1;
                    continue;
                }
                if fib == row {
                    if first_w < 0 {
                        first_w = wi;
                    }
                    last_w = wi;
                }
                wi += 1;
            }
            if fib < row {
                return Err("fiber out of range".to_string());
            }
            if first_w >= 0 {
                let (o1, _l1) = word_span(&ln, first_w).unwrap();
                let (o2, l2) = word_span(&ln, last_w).unwrap();
                return world_splice(app, line_idx, o1, o2 + l2 - o1, &repl).map(|_| None);
            }
            // empty fiber: insert before its trailing '|', or at line end for the last
            if repl.is_empty() {
                return Ok(None);
            }
            let mut sep = 2 + keyw;
            let mut f2: i32 = 0;
            let mut ins_at: i64 = -1;
            loop {
                let Some((off, len)) = word_span(&ln, sep) else {
                    break;
                };
                if &ln[off..off + len] == b"|" {
                    if f2 == row {
                        ins_at = off as i64;
                        break;
                    }
                    f2 += 1;
                }
                sep += 1;
            }
            let (at, ins) = if ins_at < 0 {
                let mut v = b" ".to_vec();
                v.extend_from_slice(&repl);
                (ln.len(), v)
            } else {
                let mut v = repl.clone();
                v.push(b' ');
                (ins_at as usize, v)
            };
            world_splice(app, line_idx, at, 0, &ins).map(|_| None)
        }
        EKind::Field => Err("cell not editable".to_string()),
    }
}

// The TUI repair: clamp an entered value onto the column's carrier set — bool by 0<
// (positive is true), nat/int by floor then clamp to ±2^53, a declared range by clamp,
// outermost. The loader refuses rest data, the barrier retracts writes; the editor
// repairs interactively and says so. Returns the admitted value and the warning
// (`<refinement> clamps <from> → <to>`) when the value moved.
pub fn clamp_typed(vtype: VType, rng: Option<(f64, f64)>, v: f64) -> (f64, Option<String>) {
    let t = match vtype {
        VType::Bool => {
            if v > 0.0 {
                1.0
            } else {
                0.0
            }
        }
        VType::Nat => v.floor().clamp(0.0, NATMAX),
        VType::Int => v.floor().clamp(-NATMAX, NATMAX),
        _ => v,
    };
    let c = match rng {
        Some((lo, hi)) => t.clamp(lo, hi),
        None => t,
    };
    if c == v {
        return (c, None);
    }
    let why = if t != v {
        match vtype {
            VType::Bool => "bool".to_string(),
            VType::Nat => "nat".to_string(),
            _ => "int".to_string(),
        }
    } else {
        let (lo, hi) = rng.unwrap_or((0.0, 0.0));
        format!("range {}..{}", fmt_num(lo), fmt_num(hi))
    };
    (
        c,
        Some(format!("{} clamps {} → {}", why, fmt_num(v), fmt_num(c))),
    )
}

// C sscanf("%lf %lf") over bytes: skip isspace, strtod prefix, twice. None unless both parse.
fn scan_two_lf(s: &[u8]) -> Option<(f64, f64)> {
    let issp = |b: u8| matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r');
    let mut i = 0usize;
    while i < s.len() && issp(s[i]) {
        i += 1;
    }
    let (x, ux) = sys::strtod_prefix(&s[i..]);
    if ux == 0 {
        return None;
    }
    i += ux;
    while i < s.len() && issp(s[i]) {
        i += 1;
    }
    let (y, uy) = sys::strtod_prefix(&s[i..]);
    if uy == 0 {
        return None;
    }
    Some((x, y))
}

// ---------- the .kore scratch ----------

// `<stem>-<8-hex>`: FNV-1a 64 (offset 0xcbf29ce484222325, prime 0x100000001b3) over
// realpath-else-raw, low 32 bits %08x; stem = RAW basename, last extension cut. Keep the
// stem/hash asymmetry.
pub fn tag_of(path: &str) -> String {
    let rp = sys::real_path(path);
    let hashed: &[u8] = rp.as_deref().map(str::as_bytes).unwrap_or(path.as_bytes());
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in hashed {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    let base = match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    };
    let sb = base.as_bytes();
    let sb = &sb[..sb.len().min(255)]; // the C's 256-byte stem buffer
    let stem = match sb.iter().rposition(|&b| b == b'.') {
        Some(i) => &sb[..i],
        None => sb,
    };
    format!(
        "{}-{:08x}",
        String::from_utf8_lossy(stem),
        (h & 0xffffffff) as u32
    )
}

fn schema_generation(path: &str) -> Option<(u64, u64)> {
    let text =
        std::fs::read_to_string(steel::migration::manifest_path(path)).ok()?;
    let fields: Vec<&str> = text.lines().next()?.split('\t').collect();
    if fields.len() != 3 || fields[0] != "ano-schema-v1" {
        return None;
    }
    let version = fields[1].parse().ok()?;
    let fingerprint = u64::from_str_radix(fields[2], 16).ok()?;
    Some((version, fingerprint))
}

// Path identity plus schema generation. Ordinary world values and population do not move the
// suffix; a successful schema barrier does, so an old registry-only undo can never cross it.
fn world_tag(app: &App) -> String {
    let tag = tag_of(&app.world.path);
    match schema_generation(&app.world.path) {
        Some((version, fingerprint)) => {
            format!("{}-s{}-{:016x}", tag, version, fingerprint)
        }
        None => tag,
    }
}

fn basename(p: &str) -> &str {
    match p.rfind('/') {
        Some(i) => &p[i + 1..],
        None => p,
    }
}

// .kore/play/<tag(demoPath||worldPath)>/<basename(pristine||worldPath)>; None on overflow
// is the C's -1 — in Rust paths don't overflow, keep Option for the empty-inputs edge.
pub fn play_scratch(app: &App) -> Option<String> {
    let key_path = if !app.demo_path.is_empty() {
        &app.demo_path
    } else {
        &app.world.path
    };
    let src = if !app.pristine.is_empty() {
        &app.pristine
    } else {
        &app.world.path
    };
    Some(format!(".kore/play/{}/{}", tag_of(key_path), basename(src)))
}

// .kore/play/<tag(demoPath)>/<demo basename> — the editable .ano copy.
pub fn play_code(app: &App) -> Option<String> {
    Some(format!(
        ".kore/play/{}/{}",
        tag_of(&app.demo_path),
        basename(&app.demo_path)
    ))
}

// Ensure the scratch's directory exists; scratch is the file itself.
fn play_mkdir(scratch: &str) {
    let dir = match scratch.rfind('/') {
        Some(i) => &scratch[..i],
        None => scratch,
    };
    mkdirs(dir);
}

// realpath succeeds AND contains "/demos/" as a plain substring; fails closed.
pub fn in_demos(path: &str) -> bool {
    sys::real_path(path)
        .map(|rp| rp.contains("/demos/"))
        .unwrap_or(false)
}

// Copy-on-first-mutation for the world: already a copy, or MODE_REG on a non-corpus file,
// is a no-op. Else copy to the play scratch, log `world copied to %s — the corpus stays
// immutable\n`, record world_orig, load the scratch, set world_is_copy, sess_ja = -1,
// undo_scan, session_rehydrate. false only on hard failure.
pub fn world_guard(app: &mut App) -> bool {
    if app.world_is_copy || (app.mode == Mode::Reg && !in_demos(&app.world.path)) {
        return true;
    }
    let Some(dst) = play_scratch(app) else {
        app.sayerr("play path overlong");
        return false;
    };
    play_mkdir(&dst);
    if !copy_file(&app.world.path, &dst) {
        app.sayerr(&format!("cannot copy world to {}", dst));
        return false;
    }
    copy_sidecar(&app.world.path, &dst); // the overlay follows the file the world reads
    let msg = format!("world copied to {} — the corpus stays immutable\n", dst);
    app.log(msg.as_bytes());
    app.world_orig = app.world.path.clone(); // >reset restores this
    match world_load(&dst) {
        Err(e) => {
            app.sayerr(&e);
            false
        }
        Ok(w) => {
            app.world = w;
            app.world_is_copy = true;
            app.sess_ja = -1; // the session moves beside the copy
            undo_scan(app);
            session_rehydrate(app);
            true
        }
    }
}

// Copy-on-first-mutation for the code: demo under demos/ copies to play_code, retargets
// demo_live, logs `code copied to %s — the corpus stays immutable\n`.
pub fn code_guard(app: &mut App) -> bool {
    if app.demo_path.is_empty() || !in_demos(&app.demo_live) {
        return true;
    }
    let Some(dst) = play_code(app) else {
        app.sayerr("play path overlong");
        return false;
    };
    play_mkdir(&dst);
    if !copy_file(&app.demo_live, &dst) {
        app.sayerr(&format!("cannot copy code to {}", dst));
        return false;
    }
    app.demo_live = dst.clone();
    let msg = format!("code copied to {} — the corpus stays immutable\n", dst);
    app.log(msg.as_bytes());
    true
}

// Load the scratch as the world (optionally copying pristine over it first: `cannot copy
// %.100s to %.100s`); when not yet a copy: set world_is_copy, sess_ja = -1, undo_scan,
// session_rehydrate.
pub fn play_adopt(app: &mut App, dst: &str, copy_first: bool) -> Result<(), String> {
    play_mkdir(dst);
    if copy_first {
        if !copy_file(&app.pristine, dst) {
            return Err(format!(
                "cannot copy {} to {}",
                trunc_str(&app.pristine, 100),
                trunc_str(dst, 100)
            ));
        }
        copy_sidecar(&app.pristine, dst); // the overlay follows the file the world reads
    }
    app.world = world_load(dst)?;
    if !app.world_is_copy {
        app.world_is_copy = true;
        app.sess_ja = -1;
        undo_scan(app);
        session_rehydrate(app);
    }
    Ok(())
}

// ---------- the undo ring (files under .kore/undo/, surviving the process) ----------
//
// The ring versions the world file alone. The dynamic-alias overlay deliberately does NOT ride
// undo or the w snapshot: A_t is host session state orthogonal to world time (todo/02:27), and
// entangling it with world time would invent semantics nobody ruled. Consequence: after a `u`
// that changes n, materialized mask and resolver entries go stale and emission refuses until a
// host transition or `kore alias <reg> clear`.

// app.undo_seq = highest existing seq matching `<tag>-` (atoi suffix), 0 when none.
pub fn undo_scan(app: &mut App) {
    app.undo_seq = 0;
    let pre = format!("{}-", world_tag(app));
    let Ok(rd) = std::fs::read_dir(".kore/undo") else {
        return;
    };
    for de in rd.flatten() {
        let name = de.file_name();
        let name = name.to_string_lossy();
        if let Some(rest) = name.strip_prefix(&pre) {
            let s = atoi(rest.as_bytes());
            if s > app.undo_seq {
                app.undo_seq = s;
            }
        }
    }
}

// Copy the CURRENT world file to seq+1; returns the new seq or -1. Every advance stages first.
pub fn undo_push(app: &mut App) -> i32 {
    mkdirs(".kore/undo");
    let dst = format!(".kore/undo/{}-{}.reg", world_tag(app), app.undo_seq + 1);
    if !copy_file(&app.world.path, &dst) {
        return -1;
    }
    app.undo_seq += 1;
    app.undo_seq
}

// Unlink the top pre-state and decrement — the failed-advance rollback.
pub fn undo_drop(app: &mut App) {
    if app.undo_seq <= 0 {
        return;
    }
    let p = format!(".kore/undo/{}-{}.reg", world_tag(app), app.undo_seq);
    let _ = std::fs::remove_file(p);
    app.undo_seq -= 1;
}

// u: restore the top pre-state over the world file, unlink, decrement, drop the stepped-back
// OUTPUTS group, reload. Verdicts verbatim (`undo → pre-state #%d restored (%d left)` …).
pub fn undo_pop(app: &mut App) {
    if app.undo_seq <= 0 {
        app.say("nothing to undo");
        return;
    }
    let p = format!(".kore/undo/{}-{}.reg", world_tag(app), app.undo_seq);
    if !copy_file(&p, &app.world.path.clone()) {
        app.sayerr(&format!("undo: cannot restore {}", p));
        return;
    }
    let _ = std::fs::remove_file(&p);
    app.undo_seq -= 1;
    app.outputs_drop_after(app.undo_seq); // the stepped-back tick's results go with it
    let path = app.world.path.clone();
    match world_load(&path) {
        Err(e) => app.sayerr(&format!("undo: {}", e)),
        Ok(w) => {
            app.world = w;
            let (a, b) = (app.undo_seq + 1, app.undo_seq);
            app.say(&format!("undo → pre-state #{} restored ({} left)", a, b));
        }
    }
}

// Unlink every ring file for tag_of(path); the file must still exist when called.
pub fn undo_wipe(path: &str) {
    let pre = format!("{}-", tag_of(path));
    let Ok(rd) = std::fs::read_dir(".kore/undo") else {
        return;
    };
    for de in rd.flatten() {
        let name = de.file_name();
        let n = name.to_string_lossy();
        if n.starts_with(&pre) {
            let _ = std::fs::remove_file(format!(".kore/undo/{}", n));
        }
    }
}

// w: .kore/<tag>-snap<seq>.reg, seq = app.snap_seq (per-process, restart overwrites snap1).
pub fn snapshot(app: &mut App) {
    if !app.world.loaded {
        app.say("no world to snapshot");
        return;
    }
    app.snap_seq += 1;
    let dst = format!(".kore/{}-snap{}.reg", world_tag(app), app.snap_seq);
    mkdirs(".kore");
    if !copy_file(&app.world.path.clone(), &dst) {
        app.sayerr("snapshot failed");
    } else {
        app.say(&format!("snapshot → {}", dst));
    }
}

// Non-dot entries directly under .kore/play.
pub fn play_count() -> i32 {
    let Ok(rd) = std::fs::read_dir(".kore/play") else {
        return 0;
    };
    let mut n = 0;
    for de in rd.flatten() {
        if !de.file_name().to_string_lossy().starts_with('.') {
            n += 1;
        }
    }
    n
}

// >reset confirmed: wipe rings, unlink every play file, rmdir dirs, rmdir .kore/play,
// unlink .kore/next.ano, outputs_clear; reopen the demo / restore world_orig. Verdict
// `reset — %d play cop%s removed (%d file%s); every demo is the pristine corpus again`.
pub fn reset_all(app: &mut App) {
    let mut demos = 0i32;
    let mut files = 0i32;
    if let Ok(rd) = std::fs::read_dir(".kore/play") {
        for de in rd.flatten() {
            let name = de.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let dir = format!(".kore/play/{}", name);
            let Ok(pd) = std::fs::read_dir(&dir) else {
                continue;
            };
            for pe in pd.flatten() {
                let pn = pe.file_name().to_string_lossy().into_owned();
                if pn.starts_with('.') {
                    continue;
                }
                let f = format!("{}/{}", dir, pn);
                undo_wipe(&f); // wipe before the unlink that orphans the ring
                if std::fs::remove_file(&f).is_ok() {
                    files += 1;
                }
            }
            if std::fs::remove_dir(&dir).is_ok() {
                demos += 1;
            }
        }
    }
    let _ = std::fs::remove_dir(".kore/play");
    let _ = std::fs::remove_file(".kore/next.ano");
    app.outputs_clear();
    if !app.demo_path.is_empty() {
        // a demo is open — rail mode or demo mode alike
        let keep = app.demo_path.clone();
        open_demo(app, &keep);
    } else if app.mode == Mode::Reg && app.world_is_copy && !app.world_orig.is_empty() {
        app.world_is_copy = false;
        app.sess_ja = -1;
        app.sdefs.clear();
        let orig = app.world_orig.clone();
        match world_load(&orig) {
            Err(e) => {
                app.sayerr(&e);
                return;
            }
            Ok(w) => app.world = w,
        }
        undo_scan(app);
        app.w_seg = 0;
        app.w_row = 0;
        app.w_col = 0;
        app.w_top = 0;
    }
    app.say(&format!(
        "reset — {} play cop{} removed ({} file{}); every demo is the pristine corpus again",
        demos,
        if demos == 1 { "y" } else { "ies" },
        files,
        if files == 1 { "" } else { "s" }
    ));
}

// The prompt's `>` verb router: `reset` and `alias`. Arms confirm_reset with the verdict
// (error hue) or says `nothing to reset — no play copies exist` / `unknown command >%.60s
// — commands: >reset >alias`.
pub fn kore_command(app: &mut App, cmd: &str) {
    let cmd = cmd.trim_start_matches(' ');
    if cmd == "alias" {
        alias_show(app);
        return;
    }
    if cmd == "reset" {
        let n = play_count();
        if n == 0 {
            app.say("nothing to reset — no play copies exist");
            return;
        }
        app.confirm_reset = true;
        app.sayerr(&format!(
            "reset {} play cop{} to the pristine corpus? y confirms — any other key cancels",
            n,
            if n == 1 { "y" } else { "ies" }
        ));
        return;
    }
    app.sayerr(&format!(
        "unknown command >{} — commands: >reset >alias",
        trunc_str(cmd, 60)
    ));
}

// ---------- the session log ----------

// <dir of world.path>/session.ano ("." when no slash) — always beside the live world.
pub fn session_path(app: &App) -> String {
    let p = &app.world.path;
    let dir = match p.rfind('/') {
        Some(i) => &p[..i],
        None => ".",
    };
    format!("{}/session.ano", dir)
}

// Append one successful statement; a fresh file writes the header (`-- kore session — a
// valid .ano program: replay with steel --run`, `--! registry session-base.reg`, `--! ja`
// when the first statement is ja). Other-surface bodies comment out line by line as
// `-- (other surface, not replayable) %s`.
pub fn session_log(app: &mut App, stmt: &[u8], ja: bool) {
    let p = session_path(app);
    let fresh = !sys::access_f(&p);
    if !fresh && app.sess_ja < 0 {
        app.sess_ja = match read_file(&p) {
            Some(s) => contains(cstr(&s), b"\n--! ja") as i32,
            None => 0,
        };
    }
    let Ok(mut f) = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&p)
    else {
        return;
    };
    if fresh {
        app.sess_ja = ja as i32;
        let _ = f.write_all(
            "-- kore session — a valid .ano program: replay with steel --run\n".as_bytes(),
        );
        let _ = f.write_all(b"--! registry session-base.reg\n");
        if ja {
            let _ = f.write_all(b"--! ja\n");
        }
    }
    if (ja as i32) != app.sess_ja {
        // every line comments out — a half-commented multi-line body would replay
        let s: Vec<u8> = stmt.iter().copied().take(1023).collect();
        for l in s.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
            let _ = f.write_all(b"-- (other surface, not replayable) ");
            let _ = f.write_all(l);
            let _ = f.write_all(b"\n");
        }
    } else {
        let _ = f.write_all(stmt);
        let _ = f.write_all(b"\n");
    }
}

// Append a seam only when session.ano already exists: `-- n: %s ticked the world (not
// replayable)` / `-- r: world reset to pristine %s (not replayable)`.
pub fn session_seam(app: &App, line: &str) {
    let sess = session_path(app);
    if !sys::access_f(&sess) {
        return;
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&sess) {
        let _ = f.write_all(line.as_bytes());
    }
}

// The def-head name: `def ` (4 bytes) or `定義 ` (7 bytes), spaces skipped, name to
// space/'='/'\n'. Exact bytes — defs never case-fold.
pub fn def_head(body: &[u8], ja: bool) -> Option<Vec<u8>> {
    let kw: &[u8] = if ja { "定義 ".as_bytes() } else { b"def " };
    if !body.starts_with(kw) {
        return None;
    }
    let mut p = kw.len();
    while p < body.len() && body[p] == b' ' {
        p += 1;
    }
    let mut out = Vec::new();
    while p < body.len()
        && body[p] != b' '
        && body[p] != b'='
        && body[p] != b'\n'
        && out.len() < 127
    {
        out.push(body[p]);
        p += 1;
    }
    if out.is_empty() { None } else { Some(out) }
}

// Any line of the body def-heads to name.
pub fn body_redefines(body: &[u8], ja: bool, name: &[u8]) -> bool {
    let b: Vec<u8> = body.iter().copied().take(1023).collect();
    b.split(|&x| x == b'\n')
        .filter(|l| !l.is_empty())
        .any(|l| def_head(l, ja).is_some_and(|dh| dh == name))
}

// Rebuild the def store from session.ano: sess_ja from a `\n--! ja` hit; skip `--` lines;
// same name replaces in place, else append up to 64. The log is the session's memory.
pub fn session_rehydrate(app: &mut App) {
    app.sdefs.clear();
    let p = session_path(app);
    let Some(raw) = read_file(&p) else { return };
    let s = cstr(&raw);
    let ja = contains(s, b"\n--! ja");
    app.sess_ja = ja as i32;
    for l0 in s.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
        let st = l0
            .iter()
            .position(|&b| b != b' ' && b != b'\t')
            .unwrap_or(l0.len());
        let l = &l0[st..];
        if l.starts_with(b"--") {
            continue;
        }
        let Some(dh) = def_head(l, ja) else { continue };
        let mut found: Option<usize> = None;
        for (i, d) in app.sdefs.iter().enumerate() {
            if d.name == dh {
                found = Some(i);
            }
        }
        match found {
            Some(i) => app.sdefs[i].text = l.to_vec(),
            None => {
                if app.sdefs.len() < KMAXSDEF {
                    app.sdefs.push(SDef {
                        text: l.to_vec(),
                        ja,
                        name: dh,
                    });
                }
            }
        }
    }
}

// ---------- the dynamic-alias overlay: host session state ----------

// Inputs: a source and a destination world path. Output: none — the destination's sidecar mirrors
// the source's, copied when there is one and REMOVED when there is not. The overlay follows the
// live world file, so `kore alias` and every emission always edit the same environment.
fn copy_sidecar(src: &str, dst: &str) {
    let from = sidecar_path(src).to_string_lossy().into_owned();
    let to = sidecar_path(dst).to_string_lossy().into_owned();
    if sys::access_f(&from) {
        let _ = copy_file(&from, &to);
    } else {
        let _ = std::fs::remove_file(&to);
    }
}

// The stem of an observation line: everything before the tab, sigil included.
fn alias_stem(line: &str) -> &str {
    match line.find('\t') {
        Some(i) => &line[..i],
        None => line,
    }
}

// The description of an observation line: everything after the tab.
fn alias_told(line: &str) -> &str {
    match line.find('\t') {
        Some(i) => &line[i + 1..],
        None => "",
    }
}

// Inputs: the live world path. Output: the environment version and its `^name<TAB>describe` lines,
// or None when the registry or its sidecar cannot be read — an observation never refuses a tick.
pub fn alias_observe(world_path: &str) -> Option<(u64, Vec<String>)> {
    let reg = steel::registry::reg_load(world_path).ok()?;
    let env = AliasEnvironment::load(sidecar_path(world_path), &reg).ok()?;
    let lines = env
        .iter()
        .map(|(name, t)| format!("^{}\t{}", name, t.describe()))
        .collect();
    Some((env.version(), lines))
}

// Inputs: the previous and current observation lines, the version they moved to, the barrier index.
// Output: the session records — the environment line, one line per installed or rebound entry, one
// per deleted. Pure; the diff keys on the stem, and every record carries its own newline.
pub fn alias_records(prev: &[String], next: &[String], version: u64, barrier: u32) -> Vec<String> {
    let mut out = vec![format!("-- alias@{}: environment v{}\n", barrier, version)];
    for line in next {
        if !prev.iter().any(|old| old == line) {
            out.push(format!(
                "-- alias@{}: {} = {}\n",
                barrier,
                alias_stem(line),
                alias_told(line)
            ));
        }
    }
    for line in prev {
        if !next.iter().any(|new| alias_stem(new) == alias_stem(line)) {
            out.push(format!(
                "-- alias@{}: {} deleted\n",
                barrier,
                alias_stem(line)
            ));
        }
    }
    out
}

// The live sidecar copied to `<session dir>/session-alias-v<version>.aliases` — the artifact a
// replay of this segment loads. No sidecar is nothing to freeze; the base copy answers for it.
fn alias_freeze(app: &App, version: u64) {
    let src = sidecar_path(&app.world.path).to_string_lossy().into_owned();
    if !sys::access_f(&src) {
        return;
    }
    let sess = session_path(app);
    let dir = match sess.rfind('/') {
        Some(i) => &sess[..i],
        None => ".",
    };
    let _ = copy_file(&src, &format!("{}/session-alias-v{}.aliases", dir, version));
}

// The session's starting observation, taken once: a transition is a move AWAY from this, never
// this. A resumed log primes here too, so its opening version still gets a frozen artifact.
fn alias_prime(app: &mut App) {
    if app.alias_ver.is_some() {
        return;
    }
    let path = app.world.path.clone();
    let Some((version, listing)) = alias_observe(&path) else {
        return;
    };
    alias_freeze(app, version);
    app.alias_ver = Some(version);
    app.alias_list = listing;
}

// One process barrier crossed (a successful submission): count it, observe the overlay, and when
// the host moved it since the last barrier append the transition records to session.ano and freeze
// the sidecar under its version. In Kore the barrier IS the submission boundary — each submission
// spawns one steel process that re-reads the sidecar, so a host transition becomes visible exactly
// at the next spawn and never mid-statement.
fn alias_barrier(app: &mut App) {
    app.sess_barrier = app.sess_barrier.wrapping_add(1);
    let path = app.world.path.clone();
    let Some((version, listing)) = alias_observe(&path) else {
        return;
    };
    if app.alias_ver == Some(version) && app.alias_list == listing {
        return;
    }
    for line in alias_records(&app.alias_list, &listing, version, app.sess_barrier) {
        session_seam(app, &line);
    }
    alias_freeze(app, version);
    app.alias_ver = Some(version);
    app.alias_list = listing;
}

// >alias: the live sidecar path (the play-scratch path-identity trap is diagnosed by reading it),
// the environment version, and one line per entry marked `(shadows <entry>)` when the stem also
// resolves in the bare namespace. Host state only — nothing enters dcols/vrows, so the overlay
// never renders as a column declaration.
fn alias_show(app: &mut App) {
    let path = app.world.path.clone();
    let side = sidecar_path(&path).to_string_lossy().into_owned();
    let Some((version, listing)) = alias_observe(&path) else {
        app.sayerr(&format!(
            "alias sidecar stale or unreadable — kore alias {} clear recovers",
            trunc_str(&path, 60)
        ));
        return;
    };
    let reg = steel::registry::reg_load(&path).ok();
    let mut text = format!("alias {}\nenvironment v{}\n", side, version);
    if listing.is_empty() {
        text.push_str("no dynamic aliases\n");
    }
    for line in &listing {
        text.push_str(line);
        let stem = alias_stem(line).trim_start_matches('^');
        if let Some(r) = reg.as_ref() {
            if let Some(index) = steel::registry::reg_find(r, stem) {
                text.push_str(&format!("\t(shadows {})", r.ents[index].name));
            }
        }
        text.push('\n');
    }
    app.log(text.as_bytes());
    app.say(&format!(
        "alias environment v{} — {} entries",
        version,
        listing.len()
    ));
}

// ---------- the ticks ----------

// One prompt submission = one program against the current world:
// history push, `>` -> kore_command, guards, `ja ` prefix, the space-in-path
// refusal, session-base snapshot, compose .kore/repl.ano (registry line, same-surface defs
// not redefined by the body, the body), undo_push, spawn steel --run --save <absw> --label
// [--trace] via crate::run_steel, echo `> %s\n`, crate::cap_split, then session_log + def
// harvest + reload on exit 0 / undo_drop on failure. Verdicts verbatim.
pub fn repl_submit(app: &mut App) {
    let stmt: Vec<u8> = app.prompt.iter().copied().take(1023).collect();
    if stmt.is_empty() {
        return;
    }
    if app.hist.len() < KMAXHIST {
        app.hist.push(stmt.clone());
    }
    app.hist_at = app.hist.len() as i32;
    app.prompt.clear();
    app.pcur = 0;
    if stmt[0] == b'>' {
        let cmd = String::from_utf8_lossy(&stmt[1..]).into_owned();
        kore_command(app, &cmd);
        return;
    }
    if !app.world.loaded {
        app.sayerr("no world loaded");
        return;
    }
    if !world_guard(app) {
        return;
    }
    let (ja, body): (bool, &[u8]) = if stmt.starts_with(b"ja ") {
        (true, &stmt[3..])
    } else {
        (false, &stmt[..])
    };
    let absw = sys::real_path(&app.world.path).unwrap_or_else(|| app.world.path.clone());
    if absw.contains(' ') || absw.contains('\t') {
        app.sayerr("world path contains a space — the --! registry directive is one word");
        return;
    }
    // first statement of a session: snapshot the pre-state the log will replay against
    let sess = session_path(app);
    if !sys::access_f(&sess) {
        let base = format!("{}-base.reg", &sess[..sess.len() - 4]);
        let world = app.world.path.clone();
        let _ = copy_file(&world, &base);
        copy_sidecar(&world, &base); // the overlay the first statement runs under
    }
    alias_prime(app);
    mkdirs(".kore");
    // one submission, one program — with the session's defs prepended (same surface,
    // any resubmitted head excluded); the body may hold several lines: one program
    let mut prog: Vec<u8> = Vec::new();
    prog.extend(format!("--! registry {}\n", absw).into_bytes());
    if ja {
        prog.extend_from_slice(b"--! ja\n");
    }
    for d in &app.sdefs {
        if d.ja == ja && !body_redefines(body, ja, &d.name) {
            prog.extend_from_slice(&d.text);
            prog.push(b'\n');
        }
    }
    prog.extend_from_slice(body);
    prog.push(b'\n');
    app.run_lines_set(&prog);
    if !write_commit(".kore/repl.ano", &prog) {
        app.sayerr("cannot write .kore/repl.ano");
        return;
    }
    let seq = undo_push(app);
    if seq < 0 {
        app.sayerr("cannot stage undo copy");
        return;
    }
    // the trace slot repeats --label when tracing is off: a fixed argv, one flag flipped
    let steel = crate::find_steel(app);
    let trace = if app.trace { "--trace" } else { "--label" };
    let argv = [
        steel.as_str(),
        "--run",
        "--save",
        &absw,
        "--label",
        trace,
        ".kore/repl.ano",
    ];
    let (cap, code) = crate::run_steel(&argv);
    let mut echo = b"> ".to_vec();
    echo.extend_from_slice(&stmt);
    echo.push(b'\n');
    app.log(&echo);
    crate::cap_split(app, &cap, code, seq);
    if code == 0 {
        session_log(app, body, ja);
        alias_barrier(app); // one submission, one barrier: the host transition records land here
        // every def line of the submission joins the session, exactly as rehydrate reads it
        for l in body.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
            let Some(dh) = def_head(l, ja) else { continue };
            let mut found: Option<usize> = None;
            for (i, d) in app.sdefs.iter().enumerate() {
                if d.ja == ja && d.name == dh {
                    found = Some(i);
                }
            }
            match found {
                Some(i) => app.sdefs[i].text = l.to_vec(),
                None => {
                    if app.sdefs.len() < KMAXSDEF {
                        app.sdefs.push(SDef {
                            text: l.to_vec(),
                            ja,
                            name: dh,
                        });
                    }
                }
            }
        }
        let path = app.world.path.clone();
        match world_load(&path) {
            Err(e) => app.sayerr(&e),
            Ok(w) => {
                app.world = w;
                app.say(&format!(
                    "world advanced · step {} staged · session logged",
                    seq
                ));
            }
        }
    } else {
        undo_drop(app);
        app.sayerr(&format!(
            "statement failed (exit {}) — the world stands",
            code
        ));
    }
    app.out_scroll = 0;
}

// The FIRST `--! registry ` line (13 bytes, single space) of ano_path, resolved: non-path
// specs (no '/' and no .reg suffix) gain .reg; relative resolves against anchor's dir.
pub fn demo_registry(ano_path: &str, anchor: &str) -> Option<String> {
    let raw = read_file(ano_path)?;
    let src = cstr(&raw);
    for l0 in src.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
        let st = l0
            .iter()
            .position(|&b| b != b' ' && b != b'\t')
            .unwrap_or(l0.len());
        let ln = &l0[st..];
        if !ln.starts_with(b"--! registry ") {
            continue;
        }
        let mut spec = &ln[13..];
        while !spec.is_empty() && spec[0] == b' ' {
            spec = &spec[1..];
        }
        while spec.len() > 1 && (spec[spec.len() - 1] == b' ' || spec[spec.len() - 1] == b'\r') {
            spec = &spec[..spec.len() - 1];
        }
        let dir = match anchor.rfind('/') {
            Some(i) => anchor[..i].to_string(),
            None => ".".to_string(),
        };
        let is_path = spec.contains(&b'/') || (spec.len() > 4 && spec.ends_with(b".reg"));
        let suffix = if is_path { "" } else { ".reg" };
        let spec_s = String::from_utf8_lossy(spec);
        return Some(if spec.first() == Some(&b'/') {
            format!("{}{}", spec_s, suffix)
        } else {
            format!("{}/{}{}", dir, spec_s, suffix)
        });
    }
    None
}

// A --! expect / expect-n / out pin, matched as steel tokenizes (any space/tab run after --!).
pub fn pin_line(lt: &[u8]) -> bool {
    if !lt.starts_with(b"--!") {
        return false;
    }
    let mut p = 3;
    while p < lt.len() && (lt[p] == b' ' || lt[p] == b'\t') {
        p += 1;
    }
    let mut k = 0;
    while p + k < lt.len() && lt[p + k] != b' ' && lt[p + k] != b'\t' && lt[p + k] != b'\r' {
        k += 1;
    }
    let w = &lt[p..p + k];
    w == b"out" || w == b"expect" || w == b"expect-n"
}

// Compose .kore/next.ano from demo_live: with absw, the first registry line retargets and
// pins DROP; without (registry-less), everything verbatim and pins hold.
pub fn tick_program(app: &mut App, absw: Option<&str>) -> Result<(), String> {
    let raw = read_file(&app.demo_live).ok_or_else(|| format!("cannot read {}", app.demo_live))?;
    let src = cstr(&raw).to_vec();
    let mut prog: Vec<u8> = Vec::new();
    let mut retargeted = false;
    let mut p = 0usize;
    loop {
        let nl = src[p..].iter().position(|&b| b == b'\n').map(|k| p + k);
        let end = nl.unwrap_or(src.len());
        let ln = &src[p..end];
        if nl.is_none() && ln.is_empty() {
            break;
        }
        let st = ln
            .iter()
            .position(|&b| b != b' ' && b != b'\t')
            .unwrap_or(ln.len());
        let lt = &ln[st..];
        if absw.is_some() && !retargeted && lt.starts_with(b"--! registry ") {
            prog.extend(format!("--! registry {}\n", absw.unwrap()).into_bytes());
            retargeted = true;
        } else if absw.is_some() && pin_line(lt) {
            // dropped: the tick program is scratch, never written back to the demo
        } else {
            prog.extend_from_slice(ln);
            prog.push(b'\n');
        }
        match nl {
            Some(k) => p = k + 1,
            None => break,
        }
    }
    if absw.is_some() && !retargeted {
        return Err(format!("no --! registry line in {}", app.demo_live));
    }
    app.run_lines_set(&prog);
    if !write_commit(".kore/next.ano", &prog) {
        return Err("cannot write .kore/next.ano".to_string());
    }
    Ok(())
}

// n: the demo tick — refusals (`bare world: statements step it —
// n steps demos`, `no demo selected`, `unsaved code — s saves it, then n steps`), the
// registry-less run (no --save, step 0), or adopt + tick_program + undo_push + steel --run
// --save + cap_split; seam + reload + `tick — world advanced · step %d · u steps back` on 0,
// undo_drop + `tick failed (exit %d) — the world stands` otherwise.
pub fn world_next(app: &mut App) {
    if app.mode == Mode::Reg {
        app.sayerr("bare world: statements step it — n steps demos");
        return;
    }
    if app.demo_path.is_empty() {
        app.sayerr("no demo selected");
        return;
    }
    // never silently write the file under the author: saving is an explicit s
    if app.code_dirty {
        app.sayerr("unsaved code — s saves it, then n steps");
        return;
    }
    mkdirs(".kore");
    if app.pristine.is_empty() {
        // no registry: nothing to advance — the demo runs verbatim, pins kept
        if let Err(e) = tick_program(app, None) {
            app.sayerr(&e);
            return;
        }
        let steel = crate::find_steel(app);
        let trace = if app.trace { "--trace" } else { "--label" };
        let argv = [steel.as_str(), "--run", "--label", trace, ".kore/next.ano"];
        let (cap, code) = crate::run_steel(&argv);
        app.log(format!("$ steel --run {}\n", app.demo_live).as_bytes());
        crate::cap_split(app, &cap, code, 0);
        if code == 0 {
            app.say("pins held (no registry — no world to step)");
        } else {
            app.sayerr(&format!("run failed (exit {}) — see output", code));
        }
        app.out_scroll = 0;
        return;
    }
    if !app.world_is_copy {
        let Some(dst) = play_scratch(app) else {
            app.sayerr("play path overlong");
            return;
        };
        let copy_first = !sys::access_f(&dst);
        if let Err(e) = play_adopt(app, &dst, copy_first) {
            app.sayerr(&e);
            return;
        }
    }
    let absw = sys::real_path(&app.world.path).unwrap_or_else(|| app.world.path.clone());
    if absw.contains(' ') || absw.contains('\t') {
        app.sayerr("world path contains a space — the --! registry directive is one word");
        return;
    }
    if let Err(e) = tick_program(app, Some(&absw)) {
        app.sayerr(&e);
        return;
    }
    let seq = undo_push(app);
    if seq < 0 {
        app.sayerr("cannot stage undo copy");
        return;
    }
    let steel = crate::find_steel(app);
    let trace = if app.trace { "--trace" } else { "--label" };
    let argv = [
        steel.as_str(),
        "--run",
        "--save",
        &absw,
        "--label",
        trace,
        ".kore/next.ano",
    ];
    let (cap, code) = crate::run_steel(&argv);
    app.log(format!("$ n — {} against {}\n", app.demo_path, app.world.path).as_bytes());
    crate::cap_split(app, &cap, code, seq);
    if code == 0 {
        session_seam(
            app,
            &format!(
                "-- n: {} ticked the world (not replayable)\n",
                app.demo_path
            ),
        );
        let path = app.world.path.clone();
        match world_load(&path) {
            Err(e) => app.sayerr(&e),
            Ok(w) => {
                app.world = w;
                app.say(&format!(
                    "tick — world advanced · step {} · u steps back",
                    seq
                ));
            }
        }
    } else {
        undo_drop(app);
        app.sayerr(&format!("tick failed (exit {}) — the world stands", code));
    }
    app.out_scroll = 0;
}

// r: MODE_REG reloads in place; a demo copies pristine over the scratch (staging the ring
// first when they differ, with the -- r: seam).
pub fn world_reset(app: &mut App) {
    if app.mode == Mode::Reg {
        if !app.world.loaded {
            app.sayerr("no world loaded");
            return;
        }
        let path = app.world.path.clone();
        match world_load(&path) {
            Err(e) => app.sayerr(&e),
            Ok(w) => {
                app.world = w;
                app.outputs_clear();
                app.say(&format!("world reloaded from {}", path));
            }
        }
        return;
    }
    if app.demo_path.is_empty() {
        app.sayerr("no demo selected");
        return;
    }
    if app.pristine.is_empty() {
        app.sayerr("this demo declares no registry — n runs it for the output");
        return;
    }
    let Some(dst) = play_scratch(app) else {
        app.sayerr("play path overlong");
        return;
    };
    let had = sys::access_f(&dst);
    if had {
        if !app.world_is_copy {
            if let Err(e) = play_adopt(app, &dst, false) {
                app.sayerr(&e);
                return;
            }
        }
        // stage the pre-reset state, unless it already equals the pristine bytes
        let sa = read_file(&app.pristine);
        let sb = read_file(&dst);
        let same = matches!((&sa, &sb), (Some(a), Some(b)) if a == b);
        if !same {
            if undo_push(app) < 0 {
                app.sayerr("cannot stage undo copy");
                return;
            }
            session_seam(
                app,
                &format!(
                    "-- r: world reset to pristine {} (not replayable)\n",
                    app.pristine
                ),
            );
        }
    }
    if let Err(e) = play_adopt(app, &dst, true) {
        app.sayerr(&e);
        return;
    }
    app.outputs_clear();
    app.w_seg = 0;
    app.w_row = 0;
    app.w_col = 0;
    app.w_top = 0;
    if had {
        app.say("reset → pristine world (n steps it, u steps back)");
    } else {
        app.say(&format!("pristine world loaded → {} (n steps it)", dst));
    }
}

// Open a demo: play code copy shadows the corpus (`code: play copy %s resumed — >reset
// restores the corpus\n`), per-demo session state resets, registry resolves and loads
// (an existing scratch resumes: `%s — play world resumed at step %d (r resets to
// pristine)`).
pub fn open_demo(app: &mut App, path: &str) {
    app.demo_path = path.to_string();
    // an earlier session's play code copy shadows a corpus demo — the buffer rides it
    let live = play_code(app);
    let shadowed = in_demos(path) && live.as_deref().map(sys::access_f).unwrap_or(false);
    app.demo_live = if shadowed {
        live.unwrap()
    } else {
        path.to_string()
    };
    let dl = app.demo_live.clone();
    crate::ui::code_load(app, &dl);
    if shadowed {
        let msg = format!(
            "code: play copy {} resumed — >reset restores the corpus\n",
            app.demo_live
        );
        app.log(msg.as_bytes());
    }
    app.world_is_copy = false;
    app.undo_seq = 0;
    app.sess_ja = -1;
    app.sdefs.clear();
    app.outputs_clear();
    if let Some(reg) = demo_registry(&dl, &app.demo_path.clone()) {
        app.pristine = reg.clone();
        // an earlier session left a play world: resume it — r resets to pristine
        if let Some(dst) = play_scratch(app) {
            if sys::access_f(&dst) && play_adopt(app, &dst, false).is_ok() {
                app.w_seg = 0;
                app.w_row = 0;
                app.w_col = 0;
                app.w_top = 0;
                let step = app.undo_seq;
                app.say(&format!(
                    "{} — play world resumed at step {} (r resets to pristine)",
                    path, step
                ));
                return;
            }
        }
        match world_load(&reg) {
            Err(e) => app.sayerr(&e),
            Ok(w) => app.world = w,
        }
    } else {
        app.world = World::default();
        app.pristine.clear();
    }
    app.w_seg = 0;
    app.w_row = 0;
    app.w_col = 0;
    app.w_top = 0;
    app.say(path);
}

// ---------- headless surfaces ----------

// Load, table_cols, spell every table_cell, and when a space exists render map and bitmap
// into a Term::headless(200, 400) with world_r = {0,0,399,199} (crate::ui::draw_space /
// draw_bitmap — the same code paths as the TUI). stdout is
// `ok %s n=%d lattice=%dx%d cols=%d fields=%d space=%s` (map+bitmap | table-only);
// failures use `FAIL %s: %s` on stderr. Returns 0/1.
pub fn check_reg(app: &mut App, path: &str) -> i32 {
    match world_load(path) {
        Err(e) => {
            eprintln!("FAIL {}: {}", path, e);
            return 1;
        }
        Ok(w) => app.world = w,
    }
    table_cols(app);
    for r in 0..app.world.n {
        for c in 0..app.dcols.len() as i32 {
            let _ = table_cell(app, r, c); // exercises the cell paths
        }
    }
    let fields = app
        .world
        .ents
        .iter()
        .filter(|e| e.kind == EKind::Field)
        .count();
    let space = app.world.lat_w > 0 || app.world.pos().is_some();
    if space {
        // render the space views to memory: the glyph map, then the bitmap
        let mut t = Term::headless(200, 400);
        t.frame_clear();
        app.world_r = Rect {
            x: 0,
            y: 0,
            w: 399,
            h: 199,
        };
        crate::ui::draw_space(app, &mut t);
        t.frame_clear();
        crate::ui::draw_bitmap(app, &mut t);
    }
    println!(
        "ok {} n={} lattice={}x{} cols={} fields={} space={}",
        path,
        app.world.n,
        app.world.lat_w,
        app.world.lat_h,
        app.dcols.len(),
        fields,
        if space { "map+bitmap" } else { "table-only" }
    );
    0
}

// kore --edit <reg> <seg> <row> <col> <value>: guard and undo bypassed; demos/ refusal
// is `FAIL %s: corpus file — point --edit at a copy`; atoi parses the triple; cell_commit
// performs the write; success re-runs table_cols and prints `ok %s` with the touched
// post-splice line ("?" when the entry cannot be re-found). Returns 0/1.
pub fn edit_reg(app: &mut App, args: &[String]) -> i32 {
    // the corpus is immutable under kore, headless included
    if in_demos(&args[0]) {
        eprintln!("FAIL {}: corpus file — point --edit at a copy", args[0]);
        return 1;
    }
    match world_load(&args[0]) {
        Err(e) => {
            eprintln!("FAIL {}: {}", args[0], e);
            return 1;
        }
        Ok(w) => app.world = w,
    }
    app.w_seg = atoi(args[1].as_bytes());
    app.w_row = atoi(args[2].as_bytes());
    app.w_col = atoi(args[3].as_bytes());
    table_cols(app);
    match cell_commit(app, args[4].as_bytes()) {
        Err(e) => {
            eprintln!("FAIL {}: {}", args[0], e);
            return 1;
        }
        // the typed repair warns on stderr; the ok line below shows the clamped spelling
        Ok(Some(w)) => eprintln!("warn {}: {}", args[0], w),
        Ok(None) => {}
    }
    table_cols(app); // the reload rebuilt ents; re-point before printing
    let ei = if app.w_seg != 0 {
        seg_field(app, app.w_seg)
    } else if app.w_col >= 0 && (app.w_col as usize) < app.dcols.len() {
        Some(app.dcols[app.w_col as usize].ent)
    } else {
        None
    };
    let out = std::io::stdout();
    let mut o = out.lock();
    let _ = o.write_all(b"ok ");
    match ei {
        Some(i) => {
            let _ = o.write_all(&app.world.lines[app.world.ents[i].line]);
        }
        None => {
            let _ = o.write_all(b"?");
        }
    }
    let _ = o.write_all(b"\n");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_and_paths() {
        let tag = tag_of("/no/such/dir/file.reg");
        assert!(tag.starts_with("file-") && tag.len() == 5 + 8);

        let path = format!(
            "{}/../demos/registries/001-canonical-masked-update.reg",
            env!("CARGO_MANIFEST_DIR")
        );
        let dotted = path.replace("/demos/", "/./demos/");
        assert_eq!(tag_of(&path), tag_of(&dotted));
    }

    #[test]
    fn undo_tag_is_namespaced_by_schema_generation() {
        let root = std::env::temp_dir().join(format!(
            "ano-world-tag-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("world.reg");
        std::fs::write(&path, b"n 0\n").unwrap();

        let mut app = App::new();
        app.world.path = path.to_string_lossy().into_owned();
        assert_eq!(world_tag(&app), tag_of(&app.world.path));
        std::fs::write(
            steel::migration::manifest_path(&app.world.path),
            b"ano-schema-v1\t7\t0123456789abcdef\n",
        )
        .unwrap();
        assert_eq!(
            world_tag(&app),
            format!("{}-s7-0123456789abcdef", tag_of(&app.world.path))
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

// The dynamic-alias overlay as Kore's host boundary sees it: the sidecar following the world file,
// the transition records, and the barrier at which a host transition becomes visible. Everything
// runs in process — kore is bin-only, so these call kore's own functions, never a spawned binary.
#[cfg(test)]
mod alias_session {
    use super::*;
    use steel::Registry;

    // Two total number columns over three rows: the stem `gold` shadows the column `Gold`.
    const FIXTURE: &str = "n 3\ncol Gold num 1 2 3\ncol Silver num 4 5 6\n";
    // Mask position under the sigiled spelling: the one place the overlay can move a lookup.
    const SRC: &str = "^Gold , Silver = 0";

    fn scratch(tag: &str) -> String {
        let d = std::env::temp_dir().join(format!("ano-kore-sess-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d.to_string_lossy().into_owned()
    }

    fn world_at(dir: &str) -> String {
        let path = format!("{}/world.reg", dir);
        std::fs::write(&path, FIXTURE).unwrap();
        path
    }

    fn argv(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| word.to_string()).collect()
    }

    // One emission: a fresh interner and a snapshot frozen at the boundary, the shape
    // steel/tests/sigil_semantics.rs drives through the same public API.
    fn plan_with_env(reg: &Registry, env: &AliasEnvironment, src: &str) -> String {
        let mut it = steel::Interner::new();
        let toks = steel::lex::lex(src.as_bytes(), false, &mut it).unwrap();
        let prog = steel::parse::parse(&toks, &mut it).unwrap();
        let snap = env.snapshot(reg).unwrap();
        let dirs = steel::Directives::default();
        match steel::emit::emit_with_aliases(&prog, reg, &dirs, &it, snap) {
            Ok(text) => text,
            Err(d) => panic!("{}: {}", src, d.msg),
        }
    }

    // One emission against the LIVE world file, sidecar and all — the pair a spawned steel reads.
    fn plan_at(world: &str, src: &str) -> String {
        let reg = steel::registry::reg_load(world).unwrap();
        let env = AliasEnvironment::load(sidecar_path(world), &reg).unwrap();
        plan_with_env(&reg, &env, src)
    }

    // The environment a recorded segment ran under, from persisted artifacts alone: the version's
    // frozen sidecar when one exists, else the session base's.
    fn replay_env(dir: &str, base_reg: &Registry, version: u64) -> AliasEnvironment {
        let versioned = format!("{}/session-alias-v{}.aliases", dir, version);
        let path = if sys::access_f(&versioned) {
            versioned
        } else {
            format!("{}/session-base.reg.aliases", dir)
        };
        AliasEnvironment::load(&path, base_reg).unwrap()
    }

    #[test]
    fn alias_records_report_installs_rebinds_and_deletes() {
        let prev = vec![
            "^focus\tGold (number)".to_string(),
            "^hot\tmask [1 0 1]".to_string(),
        ];
        let next = vec![
            "^focus\tSilver (number)".to_string(),
            "^new\tGold (number)".to_string(),
        ];
        let recs = alias_records(&prev, &next, 7, 3);
        assert_eq!(recs[0], "-- alias@3: environment v7\n");
        assert!(
            recs.contains(&"-- alias@3: ^focus = Silver (number)\n".to_string()),
            "{:?}",
            recs
        );
        assert!(
            recs.contains(&"-- alias@3: ^new = Gold (number)\n".to_string()),
            "{:?}",
            recs
        );
        assert!(
            recs.contains(&"-- alias@3: ^hot deleted\n".to_string()),
            "{:?}",
            recs
        );
        assert_eq!(recs.len(), 4);
        // an unmoved listing records the environment line alone
        assert_eq!(
            alias_records(&prev, &prev, 7, 3),
            vec!["-- alias@3: environment v7\n"]
        );
        // an emptied overlay deletes every stem
        assert_eq!(alias_records(&prev, &[], 8, 1).len(), 3);
    }

    #[test]
    fn sidecar_follows_the_world_copy() {
        let dir = scratch("follow");
        let src = format!("{}/src.reg", dir);
        let dst = format!("{}/dst.reg", dir);
        std::fs::write(&src, FIXTURE).unwrap();
        std::fs::write(sidecar_path(&src), "overlay\n").unwrap();
        copy_sidecar(&src, &dst);
        assert_eq!(
            std::fs::read_to_string(sidecar_path(&dst)).unwrap(),
            "overlay\n"
        );
        // a source without one clears the destination's stale overlay rather than leaving it live
        std::fs::remove_file(sidecar_path(&src)).unwrap();
        copy_sidecar(&src, &dst);
        assert!(!sidecar_path(&dst).exists());
    }

    // The Kore half of the barrier ruling: a host transition becomes visible exactly at the next
    // process spawn. Each emission re-reads the sidecar, so the same source takes the bare fallback
    // before the transition and the overlay target after it — and never changes mid-emission.
    #[test]
    fn alias_visibility_is_the_process_barrier() {
        let dir = scratch("barrier");
        let world = world_at(&dir);
        let before = plan_at(&world, SRC);
        assert_eq!(before, plan_at(&world, "Gold , Silver = 0"));

        assert_eq!(
            crate::aliases::run(&argv(&["alias", &world, "set", "^gold", "Silver"])),
            0
        );

        let after = plan_at(&world, SRC);
        assert_eq!(after, plan_at(&world, "Silver , Silver = 0"));
        assert_ne!(before, after);
        // the bare half of the world is where it was
        assert_eq!(plan_at(&world, "Gold , Silver = 0"), before);
    }

    // Replay is segment-wise: each statement segment runs against session-base.reg plus the
    // recorded version's sidecar. Building the artifacts exactly as the protocol writes them, a
    // replay from those files alone reproduces both the environment versions and the plans.
    #[test]
    fn alias_segment_replay_reproduces_versions_and_plans() {
        let dir = scratch("replay");
        let world = world_at(&dir);
        let sess = format!("{}/session.ano", dir);
        let base = format!("{}/session-base.reg", dir);

        // barrier 0: the session's opening observation, with the base pre-state beside it
        let (v0, list0) = alias_observe(&world).unwrap();
        assert_eq!(v0, 0);
        assert!(copy_file(&world, &base));
        copy_sidecar(&world, &base);
        let segment0 = plan_at(&world, SRC);

        // the host transition between barriers, then barrier 1
        assert_eq!(
            crate::aliases::run(&argv(&["alias", &world, "set", "^gold", "Silver"])),
            0
        );
        let (v1, list1) = alias_observe(&world).unwrap();
        assert_eq!(v1, 1);
        let records = alias_records(&list0, &list1, v1, 1);
        std::fs::write(&sess, records.concat()).unwrap();
        let frozen = format!("{}/session-alias-v{}.aliases", dir, v1);
        assert!(copy_file(&sidecar_path(&world).to_string_lossy(), &frozen));
        let segment1 = plan_at(&world, SRC);
        assert_ne!(segment0, segment1);

        // the recorded version comes out of the session log, never out of memory
        let logged = std::fs::read_to_string(&sess).unwrap();
        assert!(
            logged.contains("-- alias@1: ^gold = Silver (number)\n"),
            "{}",
            logged
        );
        let marker = "-- alias@1: environment v";
        let line = logged.lines().find(|l| l.starts_with(marker)).unwrap();
        let recorded: u64 = line[marker.len()..].parse().unwrap();
        assert_eq!(recorded, v1);

        // replay from the persisted artifacts alone
        let base_reg = steel::registry::reg_load(&base).unwrap();
        for (version, expected) in [(v0, &segment0), (recorded, &segment1)] {
            let env = replay_env(&dir, &base_reg, version);
            assert_eq!(env.version(), version);
            assert_eq!(&plan_with_env(&base_reg, &env, SRC), expected);
        }
    }
}
