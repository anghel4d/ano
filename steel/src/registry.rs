// The .reg world loader and dumper. Every load
// diagnostic is prefixed "registry line %d: " (1-based physical line, comments counted);
// the two path-level ones ("cannot read '%s'", "cannot write '%s': %s") carry no prefix.
// Numbers parse as finite f64 and dump with round-trip spellings. Reserved names refuse via
// lex::lex_reserved_fold; files move through fs::fs_read and fs::fs_write_commit.

use crate::{
    AliasRow, BindKind, ColType, Diag, ProtoField, Reap, RegEntry, RegEntryKind, Registry,
    ANO_NAMESZ, ANO_NATMAX,
};
use crate::{fs, lex, num};
use std::fmt::Write;

const ANO_ERRSZ: usize = 512;
const ANO_NROLES: usize = 8;

// The twelve emitter-reserved BQN identifiers wuniq refuses under the fold — exactly
// these, never a blanket ano prefix.
const RESV: [&str; 12] = [
    "anoN", "anoSel", "anoIdx", "anoSaveSep", "AnoSaveNum", "AnoSaveRow", "AnoRank", "AnoScat",
    "AnoAvg", "AnoNbrClamp", "AnoImage", "AnoInvFib",
];

// The emitter's relationship-write staging variables are a generated family, anoRelStage<n>,
// so the whole family is reserved by its stem rather than by any one name.
const RESVPFX: [&str; 1] = ["anoRelStage"];

// Inputs: full message. Output: the message clipped to ANO_ERRSZ-1 bytes (the C snprintf
// bound), backed off to a char boundary.
fn clip(mut s: String) -> String {
    if s.len() > ANO_ERRSZ - 1 {
        let mut cut = ANO_ERRSZ - 1;
        while !s.is_char_boundary(cut) {
            cut -= 1;
        }
        s.truncate(cut);
    }
    s
}

// Inputs: 1-based line number (0: no line prefix), message body. Output: the refusal Diag
// "registry line N: msg" (or bare msg), clipped at the C err-buffer bound.
fn rerr(ln: i32, msg: String) -> Diag {
    let full = if ln != 0 { format!("registry line {}: {}", ln, msg) } else { msg };
    Diag::refuse(clip(full))
}

// Inputs: physical line (already NUL-truncated). Output: the line with a word-boundary '#'
// comment cut and trailing space/tab/CR trimmed. Invariant: mid-word '#' is preserved.
fn strip_line(line: &str) -> &str {
    let b = line.as_bytes();
    let mut bow = true; // at beginning of word
    let mut end = b.len();
    for (i, &c) in b.iter().enumerate() {
        if c == b'#' && bow {
            end = i;
            break;
        }
        bow = c == b' ' || c == b'\t';
    }
    let mut s = &line[..end];
    while let Some(&c) = s.as_bytes().last() {
        if c == b' ' || c == b'\t' || c == b'\r' {
            s = &s[..s.len() - 1];
        } else {
            break;
        }
    }
    s
}

// Inputs: stripped line. Output: (byte offset, word) per space/tab-separated word — the
// offsets serve the raw space-carrying tails (char/fn payloads).
fn split_words(line: &str) -> Vec<(usize, &str)> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        while i < b.len() && (b[i] == b' ' || b[i] == b'\t') {
            i += 1;
        }
        if i >= b.len() {
            break;
        }
        let start = i;
        while i < b.len() && b[i] != b' ' && b[i] != b'\t' {
            i += 1;
        }
        out.push((start, &line[start..i]));
    }
    out
}

// Inputs: word, slot size, line. Output: the word owned; "bad name '%s'" when empty or
// >= slot size (the C fixed-buffer contract kept as a validation limit).
fn wname(w: &str, dstsz: usize, ln: i32) -> Result<String, Diag> {
    if w.is_empty() || w.len() >= dstsz {
        return Err(rerr(ln, format!("bad name '{}'", w)));
    }
    Ok(w.to_string())
}

// Inputs: an entry name or spelling-alias source word. Output: refusal when the lexer owns the word
// on either surface under the fold — the name is the address.
fn wfree(w: &str, ln: i32) -> Result<(), Diag> {
    if lex::lex_reserved_fold(w) {
        return Err(rerr(ln, format!("'{}' is lexer-reserved and cannot name an entry", w)));
    }
    Ok(())
}

// Inputs: registry, candidate entry name (not yet pushed). Output: refusal when the name
// folds onto an emitter-reserved identifier or another entry's name.
fn wuniq(reg: &Registry, name: &str, ln: i32) -> Result<(), Diag> {
    for r in RESV {
        if names_eq(name, r) {
            return Err(rerr(
                ln,
                format!("'{}' collides with the emitter's reserved '{}' under the case fold", name, r),
            ));
        }
    }
    for p in RESVPFX {
        if name.get(..p.len()).is_some_and(|head| names_eq(head, p)) {
            return Err(rerr(
                ln,
                format!("'{}' collides with the emitter's reserved '{}<n>' names under the case fold", name, p),
            ));
        }
    }
    for e in &reg.ents {
        if names_eq(&e.name, name) {
            return Err(rerr(ln, format!("'{}' collides with entry '{}' under the case fold", name, e.name)));
        }
    }
    Ok(())
}

// Inputs: registry, spelling-alias source word. Output: refusal when an already declared
// spelling-alias source folds equal — same rule as wuniq, per name kind.
fn wuniq_alias(reg: &Registry, w: &str, ln: i32) -> Result<(), Diag> {
    for a in &reg.aliases {
        if names_eq(&a.from, w) {
            return Err(rerr(ln, format!("alias source '{}' collides with '{}' under the case fold", w, a.from)));
        }
    }
    Ok(())
}

// The three name gates in load order — wfree, wname, wuniq; order selects the diagnostic.
fn gate_name(reg: &Registry, w: &str, ln: i32) -> Result<String, Diag> {
    wfree(w, ln)?;
    let name = wname(w, ANO_NAMESZ, ln)?;
    wuniq(reg, &name, ln)?;
    Ok(name)
}

// Inputs: words, start index, expected count (-1: any). Output: the parsed values;
// "expected %d values, got %d" on count mismatch, "bad number '%s'" per element.
fn wnums(words: &[(usize, &str)], from: usize, expect: i32, ln: i32) -> Result<Vec<f64>, Diag> {
    let c = words.len() as i32 - from as i32;
    if expect >= 0 && c != expect {
        return Err(rerr(ln, format!("expected {} values, got {}", expect, c)));
    }
    let mut v = Vec::with_capacity(c.max(0) as usize);
    for &(_, w) in &words[from.min(words.len())..] {
        match num::wnum(w) {
            Some(x) => v.push(x),
            None => return Err(rerr(ln, format!("bad number '{}'", w))),
        }
    }
    Ok(v)
}

// Inputs: registry, name. Output: entry index under names_eq, entries only (no alias hop) —
// the same comparator every resolution uses; forward-only references by construction.
fn find_ent(reg: &Registry, name: &str) -> Option<usize> {
    reg.ents.iter().position(|e| names_eq(&e.name, name))
}

fn is_uniq_col(e: &RegEntry) -> bool {
    matches!(e.kind, RegEntryKind::Col { uniq: true, .. })
}

fn col_nums(e: &RegEntry) -> Option<&[f64]> {
    if let RegEntryKind::Col { nums, .. } = &e.kind { Some(nums) } else { None }
}

// Inputs: a refined type. Output: its .reg kind word (Num-family only; sym/char/vec spell
// themselves elsewhere).
fn ty_word(ty: ColType) -> &'static str {
    match ty {
        ColType::Bool => "bool",
        ColType::Nat => "nat",
        ColType::Int => "int",
        _ => "num",
    }
}

// Inputs: a refined type, a value. Output: whether the value sits in the type's carrier
// set — bool {0,1}, nat ℕ∩[0,2^53], int ℤ∩[-2^53,2^53]. Num admits whatever.
fn ty_admits(ty: ColType, v: f64) -> bool {
    match ty {
        ColType::Bool => v == 0.0 || v == 1.0,
        ColType::Nat => v >= 0.0 && v <= ANO_NATMAX && v.fract() == 0.0,
        ColType::Int => v >= -ANO_NATMAX && v <= ANO_NATMAX && v.fract() == 0.0,
        _ => true,
    }
}

// Reject loaded values outside the declared carrier. Effects normalize at commit; loaded data does not.
fn seal_ty(kw: &str, name: &str, ty: ColType, nums: &[f64], ln: i32) -> Result<(), Diag> {
    for &v in nums {
        if !ty_admits(ty, v) {
            return Err(rerr(ln, format!("{} {}: value {} outside {}", kw, name, num::fmt_g(6, v), ty_word(ty))));
        }
    }
    Ok(())
}

// Inputs: registry, spelling-alias-table words, line, ja flag. Output: one AliasRow pushed —
// the pure spelling alias, one hop, target unvalidated free text.
fn push_alias(reg: &mut Registry, words: &[(usize, &str)], ln: i32, ja: bool) -> Result<(), Diag> {
    wfree(words[1].1, ln)?;
    wuniq_alias(reg, words[1].1, ln)?;
    let from = wname(words[1].1, ANO_NAMESZ, ln)?;
    let to = wname(words[2].1, ANO_NAMESZ, ln)?;
    reg.aliases.push(AliasRow { from, to, ja });
    Ok(())
}

// Inputs: .reg path. Output: the loaded world (entries in declaration order) or Diag.
// Invariants: two passes (capacity count, then per-line dispatch on the EXACT-BYTE first
// word); name gate order wfree -> wname -> wuniq selects which diagnostic fires; wuniq also
// refuses the twelve reserved emitter identifiers and the anoRelStage<n> family under the fold;
// forward-only references (n before cols, lattice before fields, rel before inv); the value
// currently admitted numeric domain is the finite doubles at the single wnum choke point;
// char/fn payloads are the RAW
// stripped line tail from the word's byte offset (interior spaces kept); keyed rel/srel store
// the key column's CANONICAL spelling; inv fibers compute here (ascending sources, key-space
// values); the `default` line parses its number BEFORE the entry lookup; empty = zero registry.
pub fn reg_load(path: &str) -> Result<Registry, Diag> {
    let mut reg = Registry::default();
    let bytes = match fs::fs_read(path) {
        Ok(b) => b,
        Err(_) => return Err(rerr(0, format!("cannot read '{}'", path))),
    };
    let text = String::from_utf8_lossy(&bytes).into_owned();

    let mut saw_n = false;
    let mut saw_lat = false;
    for (i, phys) in text.split('\n').enumerate() {
        let ln = (i + 1) as i32;
        // a C line stops at its first NUL
        let phys = phys.split('\0').next().unwrap_or("");
        let line = strip_line(phys);
        let words = split_words(line);
        if words.is_empty() {
            continue;
        }
        let nw = words.len();
        // raw tails into the untouched stripped line, for space-carrying payloads
        let raw2 = if nw > 2 { Some(&line[words[2].0..]) } else { None };
        let raw3 = if nw > 3 { Some(&line[words[3].0..]) } else { None };
        let k = words[0].1;

        match k {
            "n" => {
                let v = if nw == 2 { num::wnum(words[1].1) } else { None };
                let Some(v) = v.filter(|&v| v >= 0.0) else {
                    return Err(rerr(ln, "usage: n <count>".into()));
                };
                // one header: a redeclared n would let lines validate against different
                // counts, and reg_dump would have no one-header spelling to write back
                if saw_n {
                    return Err(rerr(ln, "n redeclared".into()));
                }
                saw_n = true;
                reg.n = v as i32;
            }

            "lattice" => {
                let wh = if nw == 3 { num::wnum(words[1].1).zip(num::wnum(words[2].1)) } else { None };
                let Some((w, h)) = wh.filter(|&(w, h)| w >= 0.0 && h >= 0.0) else {
                    return Err(rerr(ln, "usage: lattice <w> <h>".into()));
                };
                if saw_lat {
                    return Err(rerr(ln, "lattice redeclared".into()));
                }
                saw_lat = true;
                reg.lat_w = w as i32;
                reg.lat_h = h as i32;
            }

            "col" | "field" => {
                let is_field = k == "field";
                let rows = if is_field { reg.lat_w.wrapping_mul(reg.lat_h) } else { reg.n };
                if nw < 3 {
                    return Err(rerr(ln, format!("usage: {} <name> <type> <values>", k)));
                }
                if is_field && rows <= 0 {
                    return Err(rerr(ln, "field before lattice".into()));
                }
                let name = gate_name(&reg, words[1].1, ln)?;
                let ty_w = words[2].1;
                let (ty, nums, syms) = match ty_w {
                    "num" | "bool" | "nat" | "int" => {
                        let ty = match ty_w {
                            "bool" => ColType::Bool,
                            "nat" => ColType::Nat,
                            "int" => ColType::Int,
                            _ => ColType::Num,
                        };
                        let nums = wnums(&words, 3, rows, ln)?;
                        seal_ty(k, &name, ty, &nums, ln)?;
                        (ty, nums, Vec::new())
                    }
                    "sym" => {
                        if nw as i32 - 3 != rows {
                            return Err(rerr(ln, format!("expected {} values, got {}", rows, nw as i32 - 3)));
                        }
                        let mut syms = Vec::with_capacity(rows as usize);
                        for j in 0..rows as usize {
                            syms.push(wname(words[3 + j].1, ANO_NAMESZ, ln)?);
                        }
                        (ColType::Sym, Vec::new(), syms)
                    }
                    "char" => {
                        let Some(raw3) = raw3 else {
                            return Err(rerr(ln, format!("char {} needs a glyph string", k)));
                        };
                        if raw3.len() as i32 != rows {
                            return Err(rerr(ln, format!("expected {} glyphs, got {}", rows, raw3.len())));
                        }
                        (ColType::Char, Vec::new(), vec![raw3.to_string()])
                    }
                    "vec" => {
                        // pairs '|'-separated; flattened, nums.len() = 2*rows (pair-column convention)
                        let mut v: Vec<f64> = Vec::with_capacity(2 * rows.max(0) as usize);
                        let (mut g, mut inn) = (0i32, 0i32);
                        for &(_, w) in &words[3..] {
                            if w == "|" {
                                if inn != 2 {
                                    return Err(rerr(ln, format!("vec row {} needs 2 values", g)));
                                }
                                g += 1;
                                inn = 0;
                            } else {
                                if inn >= 2 || g >= rows {
                                    return Err(rerr(ln, format!("vec row {} needs 2 values", g)));
                                }
                                match num::wnum(w) {
                                    Some(x) => v.push(x),
                                    None => return Err(rerr(ln, format!("bad number '{}'", w))),
                                }
                                inn += 1;
                            }
                        }
                        if inn != 2 {
                            return Err(rerr(ln, format!("vec row {} needs 2 values", g)));
                        }
                        g += 1;
                        if g != rows {
                            return Err(rerr(ln, format!("expected {} vec rows, got {}", rows, g)));
                        }
                        (ColType::Num, v, Vec::new())
                    }
                    _ => return Err(rerr(ln, format!("unknown col type '{}'", ty_w))),
                };
                let kind = if is_field {
                    RegEntryKind::Field { ty, nums, syms, rng: None }
                } else {
                    RegEntryKind::Col { ty, uniq: false, nums, syms, pres: None, rng: None }
                };
                reg.ents.push(RegEntry { name, defval: 0.0, kind });
            }

            "unique" => {
                // declared injectivity: one numeric column, every element pairwise-distinct.
                // The check IS the ruled content (2026-07-11); mint-on-spawn rides on it.
                // Constraints stack (set-intersection, ano-ecs §10): an optional kind word
                // refines the carrier — `unique id nat` is injectivity ∩ ℕ. Detected LL(1):
                // data is always numeric, so a non-number in the kind slot is the kind.
                // The mint respects any carrier here (1+max clears the maximum; the f64
                // ceiling case is not checked), which is why range alone refuses to stack
                // on unique — a ceiling contradicts it.
                if nw < 3 {
                    return Err(rerr(ln, "usage: unique <name> [num|nat|int] <values>".into()));
                }
                let name = gate_name(&reg, words[1].1, ln)?;
                let (ty, vi) = if num::wnum(words[2].1).is_none() {
                    let ty = match words[2].1 {
                        "num" => ColType::Num,
                        "nat" => ColType::Nat,
                        "int" => ColType::Int,
                        w => return Err(rerr(ln, format!("unique {}: unsupported kind '{}' (num nat int)", name, w))),
                    };
                    (ty, 3)
                } else {
                    (ColType::Num, 2)
                };
                let nums = wnums(&words, vi, reg.n, ln)?;
                seal_ty(k, &name, ty, &nums, ln)?;
                for x in 0..nums.len() {
                    for y in x + 1..nums.len() {
                        if nums[x] == nums[y] {
                            return Err(rerr(
                                ln,
                                format!("unique {}: value {} repeats (rows {}, {})", name, num::fmt_g(6, nums[x]), x, y),
                            ));
                        }
                    }
                }
                reg.ents.push(RegEntry {
                    name,
                    defval: 0.0,
                    kind: RegEntryKind::Col { ty, uniq: true, nums, syms: Vec::new(), pres: None, rng: None },
                });
            }

            "pres" => {
                if nw < 2 {
                    return Err(rerr(ln, "usage: pres <col> <mask>".into()));
                }
                let idx = find_ent(&reg, words[1].1)
                    .filter(|&i| matches!(reg.ents[i].kind, RegEntryKind::Col { .. }));
                let Some(idx) = idx else {
                    return Err(rerr(ln, format!("pres: no column '{}'", words[1].1)));
                };
                if is_uniq_col(&reg.ents[idx]) {
                    return Err(rerr(ln, format!("pres on unique column '{}': a key column is total", reg.ents[idx].name)));
                }
                let mask = wnums(&words, 2, reg.n, ln)?;
                if let RegEntryKind::Col { pres, .. } = &mut reg.ents[idx].kind {
                    *pres = Some(mask);
                }
            }

            "default" => {
                if nw != 3 {
                    return Err(rerr(ln, "usage: default <name> <v>".into()));
                }
                let Some(v) = num::wnum(words[2].1) else {
                    return Err(rerr(ln, format!("bad number '{}'", words[2].1)));
                };
                let Some(idx) = find_ent(&reg, words[1].1) else {
                    return Err(rerr(ln, format!("default: no entry '{}'", words[1].1)));
                };
                // a unique column's values are minted, never defaulted: a shared default is
                // a standing violation of the declared injectivity
                if is_uniq_col(&reg.ents[idx]) {
                    return Err(rerr(ln, format!("default on unique column '{}'", reg.ents[idx].name)));
                }
                // the default is the spawn fill: it must sit in the column's carrier set
                if let RegEntryKind::Col { ty, rng, .. } | RegEntryKind::Field { ty, rng, .. } = &reg.ents[idx].kind {
                    let name = &reg.ents[idx].name;
                    if !ty_admits(*ty, v) {
                        return Err(rerr(ln, format!("default {}: value {} outside {}", name, num::fmt_g(6, v), ty_word(*ty))));
                    }
                    if let Some((lo, hi)) = rng {
                        if v < *lo || v > *hi {
                            return Err(rerr(
                                ln,
                                format!("default {}: value {} outside {}..{}", name, num::fmt_g(6, v), num::fmt_g(6, *lo), num::fmt_g(6, *hi)),
                            ));
                        }
                    }
                }
                reg.ents[idx].defval = v;
            }

            "range" => {
                // Loaded and default values must fit declared bounds; effects clamp at commit.
                if nw != 4 {
                    return Err(rerr(ln, "usage: range <col> <lo> <hi>".into()));
                }
                let idx = find_ent(&reg, words[1].1).filter(|&i| {
                    matches!(reg.ents[i].kind, RegEntryKind::Col { .. } | RegEntryKind::Field { .. })
                });
                let Some(idx) = idx else {
                    return Err(rerr(ln, format!("range: no column '{}'", words[1].1)));
                };
                if is_uniq_col(&reg.ents[idx]) {
                    return Err(rerr(ln, format!("range on unique column '{}'", reg.ents[idx].name)));
                }
                let (lo, hi) = match (num::wnum(words[2].1), num::wnum(words[3].1)) {
                    (Some(lo), Some(hi)) => (lo, hi),
                    (None, _) => return Err(rerr(ln, format!("bad number '{}'", words[2].1))),
                    (_, None) => return Err(rerr(ln, format!("bad number '{}'", words[3].1))),
                };
                let name = reg.ents[idx].name.clone();
                if lo > hi {
                    return Err(rerr(ln, format!("range {}: lo {} above hi {}", name, num::fmt_g(6, lo), num::fmt_g(6, hi))));
                }
                let is_field = matches!(reg.ents[idx].kind, RegEntryKind::Field { .. });
                let crows = if is_field { reg.lat_w.wrapping_mul(reg.lat_h) } else { reg.n };
                let defval = reg.ents[idx].defval;
                let (RegEntryKind::Col { ty, nums, rng, .. } | RegEntryKind::Field { ty, nums, rng, .. }) =
                    &mut reg.ents[idx].kind
                else {
                    unreachable!()
                };
                match ty {
                    ColType::Bool => return Err(rerr(ln, format!("range on bool column '{}'", name))),
                    ColType::Sym => return Err(rerr(ln, format!("range on sym column '{}'", name))),
                    ColType::Char => return Err(rerr(ln, format!("range on char column '{}'", name))),
                    _ => {}
                }
                if crows > 0 && nums.len() as i64 == 2 * crows as i64 {
                    return Err(rerr(ln, format!("range on vec column '{}'", name)));
                }
                if rng.is_some() {
                    return Err(rerr(ln, format!("range redeclared for '{}'", name)));
                }
                for &v in nums.iter() {
                    if v < lo || v > hi {
                        return Err(rerr(
                            ln,
                            format!("range {}: value {} outside {}..{}", name, num::fmt_g(6, v), num::fmt_g(6, lo), num::fmt_g(6, hi)),
                        ));
                    }
                }
                // the spawn fill (declared default, else the type zero) must sit inside
                if defval < lo || defval > hi {
                    return Err(rerr(
                        ln,
                        format!("range {}: default {} outside {}..{}", name, num::fmt_g(6, defval), num::fmt_g(6, lo), num::fmt_g(6, hi)),
                    ));
                }
                *rng = Some((lo, hi));
            }

            "rel" | "alias" => {
                if nw < 2 {
                    return Err(rerr(ln, format!("usage: {} <name> <values>", k)));
                }
                // keyed form (rel only): data is always numeric, so two names before it mean
                // the first is the key column and the second the declared name — LL(1)
                let is_rel = k == "rel";
                let keyed = is_rel && nw >= 3 && num::wnum(words[2].1).is_none() && words[2].1 != "|";
                let ni = if keyed { 2 } else { 1 };
                let name = gate_name(&reg, words[ni].1, ln)?;
                let mut key_of = None;
                if keyed {
                    let kc = find_ent(&reg, words[1].1).filter(|&i| is_uniq_col(&reg.ents[i]));
                    let Some(kc) = kc else {
                        return Err(rerr(ln, format!("rel {}: key '{}' is not a unique column", name, words[1].1)));
                    };
                    let kname = reg.ents[kc].name.clone();
                    // the key column's declared carrier governs which values it admits — nat excludes
                    // negatives, int admits them; keying a rel through a column restricts nothing further
                    key_of = Some(kname);
                }
                let vals = wnums(&words, ni + 1, reg.n, ln)?;
                let kind = if is_rel {
                    RegEntryKind::Rel { targets: vals, key_of }
                } else {
                    RegEntryKind::AliasMask { mask: vals }
                };
                reg.ents.push(RegEntry { name, defval: 0.0, kind });
            }

            "srel" => {
                if nw < 2 {
                    return Err(rerr(ln, "usage: srel <name> <fibers>".into()));
                }
                let keyed = nw >= 3 && num::wnum(words[2].1).is_none() && words[2].1 != "|";
                let ni = if keyed { 2 } else { 1 };
                let name = gate_name(&reg, words[ni].1, ln)?;
                let mut key_of = None;
                if keyed {
                    let kc = find_ent(&reg, words[1].1).filter(|&i| is_uniq_col(&reg.ents[i]));
                    let Some(kc) = kc else {
                        return Err(rerr(ln, format!("srel {}: key '{}' is not a unique column", name, words[1].1)));
                    };
                    key_of = Some(reg.ents[kc].name.clone());
                }
                // fiber count stored as written, never checked against n
                let mut fib: Vec<Vec<f64>> = vec![Vec::new()];
                for &(_, w) in &words[ni + 1..] {
                    if w == "|" {
                        fib.push(Vec::new());
                    } else {
                        match num::wnum(w) {
                            Some(x) => fib.last_mut().unwrap().push(x),
                            None => return Err(rerr(ln, format!("bad number '{}'", w))),
                        }
                    }
                }
                reg.ents.push(RegEntry { name, defval: 0.0, kind: RegEntryKind::SRel { fib, inv_of: None, key_of } });
            }

            "inv" => {
                if nw != 3 {
                    return Err(rerr(ln, "usage: inv <name> <rel>".into()));
                }
                let rel_idx = find_ent(&reg, words[2].1)
                    .filter(|&i| matches!(reg.ents[i].kind, RegEntryKind::Rel { .. }));
                let Some(rel_idx) = rel_idx else {
                    return Err(rerr(ln, format!("inv: no functional rel '{}'", words[2].1)));
                };
                let name = gate_name(&reg, words[1].1, ln)?;
                let inv_of = wname(words[2].1, ANO_NAMESZ, ln)?;
                let (targets, rel_key) = match &reg.ents[rel_idx].kind {
                    RegEntryKind::Rel { targets, key_of } => (targets.clone(), key_of.clone()),
                    _ => unreachable!(),
                };
                // fiber for target t: ascending source ids with rel==key(t); nfib = world n.
                // The inverse of a keyed rel is keyed automatically: targets match through the
                // key column and fiber values are the sources' keys.
                let mut key_of = None;
                let mut kc_nums: Option<Vec<f64>> = None;
                if let Some(kname) = rel_key {
                    let Some(kc) = find_ent(&reg, &kname) else {
                        return Err(rerr(ln, format!("inv {}: key column '{}' missing", name, kname)));
                    };
                    kc_nums = col_nums(&reg.ents[kc]).map(<[f64]>::to_vec);
                    key_of = Some(kname);
                }
                let n = reg.n.max(0) as usize;
                let mut fib: Vec<Vec<f64>> = Vec::with_capacity(n);
                for t in 0..n {
                    let tk = kc_nums.as_ref().map_or(t as f64, |kn| kn[t]);
                    let mut f = Vec::new();
                    for (s, &tv) in targets.iter().enumerate() {
                        if tv == tk {
                            f.push(kc_nums.as_ref().map_or(s as f64, |kn| kn[s]));
                        }
                    }
                    fib.push(f);
                }
                reg.ents.push(RegEntry { name, defval: 0.0, kind: RegEntryKind::SRel { fib, inv_of: Some(inv_of), key_of } });
            }

            "bind" => {
                if nw < 3 {
                    return Err(rerr(ln, "usage: bind <name> <kind> <values>".into()));
                }
                let name = gate_name(&reg, words[1].1, ln)?;
                let (kind, expect) = match words[2].1 {
                    "entity" => (BindKind::Entity, 1),
                    "num" => (BindKind::Num, 1),
                    "point" => (BindKind::Point, 2),
                    "mask" => (BindKind::Mask, reg.n),
                    "vec" => (BindKind::Vec, -1),
                    bk => return Err(rerr(ln, format!("unknown bind kind '{}'", bk))),
                };
                let vals = wnums(&words, 3, expect, ln)?;
                reg.ents.push(RegEntry { name, defval: 0.0, kind: RegEntryKind::Bind { kind, vals } });
            }

            "fn" => {
                if nw < 2 {
                    return Err(rerr(ln, "usage: fn <name> [bqn]".into()));
                }
                let name = gate_name(&reg, words[1].1, ln)?;
                let body = raw2.map(str::to_string); // verbatim BQN body convention
                reg.ents.push(RegEntry { name, defval: 0.0, kind: RegEntryKind::Fn { body } });
            }

            // the spelling alias: one hop, no transitivity, outranked by real entries; `ja` and
            // 2-arity `as` fill one table — the ja spelling documents the JA surface
            "ja" => {
                if nw != 3 {
                    return Err(rerr(ln, "usage: ja <word> <name>".into()));
                }
                push_alias(&mut reg, &words, ln, true)?;
            }
            "as" if nw == 3 => push_alias(&mut reg, &words, ln, false)?,

            "as" => {
                // the derived tag: `as <word> <col> <value>` — the word names the equality
                // mask over the live column, recomputed at each use. The word is an entry
                // name and folds; the value is a value and never does.
                if nw != 4 {
                    return Err(rerr(ln, "usage: as <word> <name> | as <word> <col> <value>".into()));
                }
                let cidx = find_ent(&reg, words[2].1).filter(|&i| {
                    matches!(reg.ents[i].kind, RegEntryKind::Col { .. } | RegEntryKind::Field { .. })
                });
                let Some(cidx) = cidx else {
                    return Err(rerr(ln, format!("as: no column '{}'", words[2].1)));
                };
                let (cty, cnn, is_field) = match &reg.ents[cidx].kind {
                    RegEntryKind::Col { ty, nums, .. } => (*ty, nums.len(), false),
                    RegEntryKind::Field { ty, nums, .. } => (*ty, nums.len(), true),
                    _ => unreachable!(),
                };
                if cty == ColType::Char {
                    return Err(rerr(ln, "as: derived tag over a char column is unsupported".into()));
                }
                let crows = if is_field { reg.lat_w.wrapping_mul(reg.lat_h) } else { reg.n };
                if cty == ColType::Num && crows > 0 && cnn as i64 == 2 * crows as i64 {
                    return Err(rerr(ln, "as: derived tag over a vec column is unsupported".into()));
                }
                let name = gate_name(&reg, words[1].1, ln)?;
                let col = wname(words[2].1, ANO_NAMESZ, ln)?; // the word AS WRITTEN
                let (num_v, sym_v) = if cty == ColType::Sym {
                    (0.0, Some(wname(words[3].1, ANO_NAMESZ, ln)?))
                } else {
                    (wnums(&words, 3, 1, ln)?[0], None)
                };
                reg.ents.push(RegEntry {
                    name,
                    defval: 0.0,
                    kind: RegEntryKind::Tag { col, carrier_ty: cty, num: num_v, sym: sym_v },
                });
            }

            "role" => {
                // point a system role at a native column; the emitter routes spawn machinery
                // through reg_role, so `role pos 位置` gives a kanji column the `pos` behaviour.
                if nw != 3 {
                    return Err(rerr(ln, "usage: role <role> <col>".into()));
                }
                const KNOWN: [&str; 5] = ["keys", "id", "parent", "proto", "pos"];
                if !KNOWN.contains(&words[1].1) {
                    return Err(rerr(ln, format!("unknown role '{}' (keys id parent proto pos)", words[1].1)));
                }
                let cidx = find_ent(&reg, words[2].1)
                    .filter(|&i| matches!(reg.ents[i].kind, RegEntryKind::Col { .. }));
                if cidx.is_none() {
                    return Err(rerr(ln, format!("role: no column '{}'", words[2].1)));
                }
                if reg.roles.len() >= ANO_NROLES {
                    return Err(rerr(ln, "too many roles".into()));
                }
                let rname = wname(words[1].1, ANO_NAMESZ, ln)?;
                let rcol = wname(words[2].1, ANO_NAMESZ, ln)?;
                reg.roles.push((rname, rcol));
            }

            "def" => {
                // the proto, a registered archetype: named field=value pairs. Spawn fill
                // layer one; layers two and three are `default` and the type zero.
                if nw < 2 {
                    return Err(rerr(ln, "usage: def <name> [<col>=<v> ...]".into()));
                }
                let name = gate_name(&reg, words[1].1, ln)?;
                let mut fields = Vec::with_capacity(nw - 2);
                for &(_, fw) in &words[2..] {
                    let eq = fw.find('=');
                    let bad = match eq {
                        None | Some(0) => true,
                        Some(p) => p + 1 == fw.len(),
                    };
                    if bad {
                        return Err(rerr(ln, format!("def {}: field '{}' is not <col>=<v>", name, fw)));
                    }
                    let p = eq.unwrap();
                    let (cw, vw) = (&fw[..p], &fw[p + 1..]);
                    let cidx = find_ent(&reg, cw).filter(|&i| {
                        matches!(reg.ents[i].kind, RegEntryKind::Col { .. } | RegEntryKind::Rel { .. })
                    });
                    let Some(cidx) = cidx else {
                        return Err(rerr(ln, format!("def {}: no column '{}'", name, cw)));
                    };
                    let cname = reg.ents[cidx].name.clone();
                    if is_uniq_col(&reg.ents[cidx]) {
                        return Err(rerr(ln, format!("def {}: unique column '{}' is minted, not defaulted", name, cname)));
                    }
                    // the id/keys role column mints at spawn even when merely magic-named — a
                    // proto field on it would be silently outranked by the mint (mid-load view)
                    if reg_role(&reg, "keys") == Some(cidx) || reg_role(&reg, "id") == Some(cidx) {
                        return Err(rerr(ln, format!("def {}: key column '{}' is minted, not defaulted", name, cname)));
                    }
                    let (is_rel, cty, cnn, crng) = match &reg.ents[cidx].kind {
                        RegEntryKind::Col { ty, nums, rng, .. } => (false, *ty, nums.len(), *rng),
                        RegEntryKind::Rel { .. } => (true, ColType::Num, 0, None),
                        _ => unreachable!(),
                    };
                    let crows = reg.n;
                    if !is_rel && cty == ColType::Num && crows > 0 && cnn as i64 == 2 * crows as i64 {
                        return Err(rerr(ln, format!("def {}: proto field over vec column '{}' unsupported", name, cname)));
                    }
                    if !is_rel && cty == ColType::Char {
                        return Err(rerr(ln, format!("def {}: proto field over char column '{}' unsupported", name, cname)));
                    }
                    let mut num_v = 0.0;
                    if is_rel || cty != ColType::Sym {
                        match num::wnum(vw) {
                            Some(x) => num_v = x,
                            None => return Err(rerr(ln, format!("def {}: bad number '{}' for '{}'", name, vw, cname))),
                        }
                    }
                    // a proto value is spawn fill layer one: it must sit in the carrier set
                    if !is_rel {
                        if !ty_admits(cty, num_v) {
                            return Err(rerr(
                                ln,
                                format!("def {}: value {} outside {} for '{}'", name, num::fmt_g(6, num_v), ty_word(cty), cname),
                            ));
                        }
                        if let Some((lo, hi)) = crng {
                            if num_v < lo || num_v > hi {
                                return Err(rerr(
                                    ln,
                                    format!("def {}: value {} outside {}..{} for '{}'", name, num::fmt_g(6, num_v), num::fmt_g(6, lo), num::fmt_g(6, hi), cname),
                                ));
                            }
                        }
                    }
                    let col = wname(&cname, ANO_NAMESZ, ln)?;
                    let spelling = wname(vw, ANO_NAMESZ, ln)?;
                    fields.push(ProtoField { col, spelling, num: num_v });
                }
                reg.ents.push(RegEntry { name, defval: 0.0, kind: RegEntryKind::Proto { fields } });
            }

            "reap" => {
                // Storage metadata only; the emitter does not consult it.
                if nw != 2 || (words[1].1 != "seal" && words[1].1 != "host") {
                    return Err(rerr(ln, "usage: reap <seal|host>".into()));
                }
                if reg.reap.is_some() {
                    return Err(rerr(ln, "reap redeclared".into()));
                }
                reg.reap = Some(if words[1].1 == "seal" { Reap::Seal } else { Reap::Host });
            }

            _ => return Err(rerr(ln, format!("unknown kind '{}'", k))),
        }
    }
    crate::relationship::validate_registry(&reg)?;
    Ok(reg)
}

// Inputs: two names. Output: equality under the ASCII fold — bytes A-Z fold (+32), every
// other byte exact (kanji and all UTF-8 untouched by construction). The ONE comparator for
// every registry name resolution; kind/type/role/reap keywords compare exact-byte instead.
// Never str::to_lowercase — no Unicode folding, fold bytes with u8::to_ascii_lowercase.
pub fn names_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| x.to_ascii_lowercase() == y.to_ascii_lowercase())
}

// Inputs: registry, surface word. Output: entry INDEX — entry names first (declaration
// order), then the spelling-alias table (one hop, no transitivity; a dangling target is None),
// both under names_eq. Indices replace C's entry pointers everywhere downstream.
pub fn reg_find(reg: &Registry, name: &str) -> Option<usize> {
    if let Some(i) = find_ent(reg, name) {
        return Some(i);
    }
    for a in &reg.aliases {
        if names_eq(&a.from, name) {
            return find_ent(reg, &a.to);
        }
    }
    None
}

// Inputs: registry, role word (keys id parent proto pos). Output: entry index by the ladder:
// (1) a declared role line matching under names_eq, its col resolved among entries;
// (2) for id/keys only, the first declared unique column (declared beats guess);
// (3) reg_find on the literal role name (entries then aliases, one resolver).
pub fn reg_role(reg: &Registry, role: &str) -> Option<usize> {
    for (rn, rc) in &reg.roles {
        if names_eq(rn, role) {
            if let Some(j) = find_ent(reg, rc) {
                return Some(j);
            }
        }
    }
    if names_eq(role, "id") || names_eq(role, "keys") {
        if let Some(u) = reg.ents.iter().position(is_uniq_col) {
            return Some(u);
        }
    }
    reg_find(reg, role)
}

// Inputs: sink, keyword (col/field), entry name, type, values, rows. Output: one column
// line — sym words, the char glyph run, the structurally re-detected pair column as vec,
// else num/bool with dnum-canonical values. Trailing newline included.
fn dump_column(b: &mut String, kw: &str, name: &str, ty: ColType, nums: &[f64], syms: &[String], rows: i32) {
    match ty {
        ColType::Sym => {
            let _ = write!(b, "{} {} sym", kw, name);
            for s in syms {
                b.push(' ');
                b.push_str(s);
            }
        }
        ColType::Char => {
            let _ = write!(b, "{} {} char {}", kw, name, syms.first().map(String::as_str).unwrap_or(""));
        }
        _ => {
            if rows > 0 && nums.len() as i64 == 2 * rows as i64 {
                // pair-column convention
                let _ = write!(b, "{} {} vec", kw, name);
                for j in 0..rows as usize {
                    if j > 0 {
                        b.push_str(" |");
                    }
                    b.push(' ');
                    b.push_str(&num::dnum(nums[2 * j]));
                    b.push(' ');
                    b.push_str(&num::dnum(nums[2 * j + 1]));
                }
            } else {
                let _ = write!(b, "{} {} {}", kw, name, ty_word(ty));
                for &v in nums {
                    b.push(' ');
                    b.push_str(&num::dnum(v));
                }
            }
        }
    }
    b.push('\n');
}

// Inputs: loaded registry, target path. Output: () or an unprefixed write diagnostic.
// Emits n, lattice, reap, entries, roles, then aliases in declaration order. Inverse fibers
// recompute on load. fs_write_commit performs the staged replacement.
pub fn reg_dump(reg: &Registry, path: &str) -> Result<(), Diag> {
    crate::relationship::validate_registry(reg)?;
    let mut b = String::new();
    let _ = writeln!(b, "n {}", reg.n);
    if reg.lat_w != 0 || reg.lat_h != 0 {
        let _ = writeln!(b, "lattice {} {}", reg.lat_w, reg.lat_h);
    }
    match reg.reap {
        Some(Reap::Seal) => b.push_str("reap seal\n"),
        Some(Reap::Host) => b.push_str("reap host\n"),
        None => {}
    }
    for e in &reg.ents {
        // the range rider dumps AFTER the default line: load checks the declared default
        // against the range at the range line, so default must precede it on reload
        let mut ent_rng: Option<(f64, f64)> = None;
        match &e.kind {
            RegEntryKind::Col { ty, uniq, nums, syms, pres, rng } => {
                if *uniq {
                    // the kind word rides the line only when refined: bare unique = num
                    match ty {
                        ColType::Num => {
                            let _ = write!(b, "unique {}", e.name);
                        }
                        _ => {
                            let _ = write!(b, "unique {} {}", e.name, ty_word(*ty));
                        }
                    }
                    for &v in nums {
                        b.push(' ');
                        b.push_str(&num::dnum(v));
                    }
                    b.push('\n');
                } else {
                    dump_column(&mut b, "col", &e.name, *ty, nums, syms, reg.n);
                    if let Some(p) = pres {
                        let _ = write!(b, "pres {}", e.name);
                        for &v in p {
                            b.push(' ');
                            b.push_str(&num::dnum(v));
                        }
                        b.push('\n');
                    }
                    ent_rng = *rng;
                }
            }
            RegEntryKind::Field { ty, nums, syms, rng } => {
                dump_column(&mut b, "field", &e.name, *ty, nums, syms, reg.lat_w.wrapping_mul(reg.lat_h));
                ent_rng = *rng;
            }
            RegEntryKind::Rel { targets, key_of } => {
                match key_of {
                    Some(kf) => {
                        let _ = write!(b, "rel {} {}", kf, e.name);
                    }
                    None => {
                        let _ = write!(b, "rel {}", e.name);
                    }
                }
                for &v in targets {
                    b.push(' ');
                    b.push_str(&num::dnum(v));
                }
                b.push('\n');
            }
            RegEntryKind::AliasMask { mask } => {
                let _ = write!(b, "alias {}", e.name);
                for &v in mask {
                    b.push(' ');
                    b.push_str(&num::dnum(v));
                }
                b.push('\n');
            }
            RegEntryKind::SRel { fib, inv_of, key_of } => {
                if let Some(iv) = inv_of {
                    // fibers recompute at load, never dumped
                    let _ = writeln!(b, "inv {} {}", e.name, iv);
                } else {
                    match key_of {
                        Some(kf) => {
                            let _ = write!(b, "srel {} {}", kf, e.name);
                        }
                        None => {
                            let _ = write!(b, "srel {}", e.name);
                        }
                    }
                    for (f, fiber) in fib.iter().enumerate() {
                        if f > 0 {
                            b.push_str(" |");
                        }
                        for &v in fiber {
                            b.push(' ');
                            b.push_str(&num::dnum(v));
                        }
                    }
                    b.push('\n');
                }
            }
            RegEntryKind::Bind { kind, vals } => {
                let ks = match kind {
                    BindKind::Entity => "entity",
                    BindKind::Mask => "mask",
                    BindKind::Point => "point",
                    BindKind::Num => "num",
                    BindKind::Vec => "vec",
                };
                let _ = write!(b, "bind {} {}", e.name, ks);
                for &v in vals {
                    b.push(' ');
                    b.push_str(&num::dnum(v));
                }
                b.push('\n');
            }
            RegEntryKind::Fn { body } => match body {
                Some(s) => {
                    let _ = writeln!(b, "fn {} {}", e.name, s);
                }
                None => {
                    let _ = writeln!(b, "fn {}", e.name);
                }
            },
            RegEntryKind::Tag { col, carrier_ty, num: nv, sym } => {
                let _ = write!(b, "as {} {} ", e.name, col);
                if *carrier_ty == ColType::Sym {
                    b.push_str(sym.as_deref().unwrap_or(""));
                } else {
                    b.push_str(&num::dnum(*nv));
                }
                b.push('\n');
            }
            RegEntryKind::Proto { fields } => {
                let _ = write!(b, "def {}", e.name);
                for f in fields {
                    let _ = write!(b, " {}={}", f.col, f.spelling);
                }
                b.push('\n');
            }
        }
        if e.defval != 0.0 {
            let _ = write!(b, "default {} ", e.name);
            b.push_str(&num::dnum(e.defval));
            b.push('\n');
        }
        if let Some((lo, hi)) = ent_rng {
            let _ = writeln!(b, "range {} {} {}", e.name, num::dnum(lo), num::dnum(hi));
        }
    }
    for (rn, rc) in &reg.roles {
        let _ = writeln!(b, "role {} {}", rn, rc);
    }
    for a in &reg.aliases {
        let _ = writeln!(b, "{} {} {}", if a.ja { "ja" } else { "as" }, a.from, a.to);
    }
    match fs::fs_write_commit(path, b.as_bytes()) {
        Ok(()) => Ok(()),
        Err(e) => Err(rerr(0, format!("cannot write '{}': {}", path, fs::strerror(&e)))),
    }
}
