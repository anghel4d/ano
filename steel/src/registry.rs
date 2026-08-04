// The .reg world loader and dumper. Every load
// diagnostic is prefixed "registry line %d: " (1-based physical line, comments counted);
// the two path-level ones ("cannot read '%s'", "cannot write '%s': %s") carry no prefix.
// Numbers parse as extended-real f64 without NaN and dump with round-trip spellings. Reserved names refuse via
// lex::lex_reserved_fold; files move through fs::fs_read and fs::fs_write_commit.

use crate::{
    AliasRow, ArrayDescriptor, ArrayDomain, BindKind, CallableDescriptor, CallableSignature,
    ColType, ConstructedPayload, ConstructedValue, ConstructorDescriptor, ConstructorInput,
    ConstructorRefinement, DeclId, DeclMeta, DeclarationStamp, Determinism, Diag, EffectSet,
    EnumCase, EnumDescriptor, ProtoField, Reap, RegEntry, RegEntryKind, RegType,
    ResidentArrayHandle, ResidentArrayValue,
    Registry, ServiceDescriptor, ServiceDirection, TrustBoundary, ANO_NAMESZ, ANO_NATMAX,
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
const PRIMITIVE_REG_TYPES: [&str; 8] =
    ["unit", "mask", "nat", "int", "num", "sym", "char", "entity"];

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

fn validate_nominal_name(name: &str, ln: i32) -> Result<(), Diag> {
    if PRIMITIVE_REG_TYPES.contains(&name) {
        return Err(rerr(
            ln,
            format!("nominal declaration '{}' collides with a primitive carrier word", name),
        ));
    }
    Ok(())
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
    for alias in &reg.aliases {
        if names_eq(&alias.from, name) {
            return Err(rerr(
                ln,
                format!("'{}' collides with alias source '{}' under the case fold", name, alias.from),
            ));
        }
    }
    Ok(())
}

// Inputs: registry, spelling-alias source word. Output: refusal when an already declared
// spelling-alias source folds equal — same rule as wuniq, per name kind.
fn wuniq_alias(reg: &Registry, w: &str, ln: i32) -> Result<(), Diag> {
    for entry in &reg.ents {
        if names_eq(&entry.name, w) {
            return Err(rerr(
                ln,
                format!("alias source '{}' collides with entry '{}' under the case fold", w, entry.name),
            ));
        }
    }
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

// Inputs: a declared scalar carrier and a value. Output: whether the value sits in its
// denotation. Num is the extended real (finite or ±∞), never NaN; refinements remain finite.
pub fn type_admits(ty: ColType, v: f64) -> bool {
    match ty {
        ColType::Num => !v.is_nan(),
        ColType::Bool => v == 0.0 || v == 1.0,
        ColType::Nat => v >= 0.0 && v <= ANO_NATMAX && v.fract() == 0.0,
        ColType::Int => v >= -ANO_NATMAX && v <= ANO_NATMAX && v.fract() == 0.0,
        ColType::Sym | ColType::Char => false,
    }
}

// Reject loaded values outside the declared carrier. Effects normalize at commit; loaded data does not.
fn seal_ty(kw: &str, name: &str, ty: ColType, nums: &[f64], ln: i32) -> Result<(), Diag> {
    for &v in nums {
        if !type_admits(ty, v) {
            return Err(rerr(ln, format!("{} {}: value {} outside {}", kw, name, num::fmt_g(6, v), ty_word(ty))));
        }
    }
    Ok(())
}

// High-integrity declarations use explicit nonzero 64-bit hexadecimal identities and positive
// decimal versions. Legacy rows retain manifest-derived version-zero identities.
fn declaration_meta(reg: &Registry, id_word: &str, version_word: &str, ln: i32) -> Result<DeclMeta, Diag> {
    let Some(id_text) = id_word.strip_prefix("id:") else {
        return Err(rerr(ln, format!("expected id:<16-hex>, got '{}'", id_word)));
    };
    if id_text.len() != 16 || !id_text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(rerr(ln, format!("bad declaration id '{}': expected 16 hex digits", id_text)));
    }
    let id = u64::from_str_radix(id_text, 16).unwrap();
    if id == 0 {
        return Err(rerr(ln, "declaration id 0000000000000000 is reserved".into()));
    }
    let Some(version_text) = version_word.strip_prefix("v:") else {
        return Err(rerr(ln, format!("expected v:<positive-decimal>, got '{}'", version_word)));
    };
    let version = version_text
        .parse::<u64>()
        .ok()
        .filter(|version| *version > 0)
        .ok_or_else(|| rerr(ln, format!("bad declaration version '{}'", version_text)))?;
    for entry in &reg.ents {
        if entry.kind.declaration_meta().is_some_and(|meta| meta.id == DeclId(id)) {
            return Err(rerr(
                ln,
                format!("declaration id {:016x} is already owned by '{}'", id, entry.name),
            ));
        }
    }
    Ok(DeclMeta { id: DeclId(id), version })
}

pub fn reg_type_word(carrier: &RegType) -> &str {
    match carrier {
        RegType::Unit => "unit",
        RegType::Mask => "mask",
        RegType::Nat => "nat",
        RegType::Int => "int",
        RegType::Num => "num",
        RegType::Sym => "sym",
        RegType::Char => "char",
        RegType::Entity => "entity",
        RegType::Named(name) => name,
    }
}

fn parse_reg_type(reg: &Registry, word: &str, unit: bool, ln: i32) -> Result<RegType, Diag> {
    let builtin = match word {
        "unit" if unit => Some(RegType::Unit),
        "mask" => Some(RegType::Mask),
        "nat" => Some(RegType::Nat),
        "int" => Some(RegType::Int),
        "num" => Some(RegType::Num),
        "sym" => Some(RegType::Sym),
        "char" => Some(RegType::Char),
        "entity" => Some(RegType::Entity),
        _ => None,
    };
    if let Some(carrier) = builtin {
        return Ok(carrier);
    }
    let Some(index) = find_ent(reg, word) else {
        return Err(rerr(ln, format!("unknown registry carrier '{}'", word)));
    };
    if !matches!(
        reg.ents[index].kind,
        RegEntryKind::Enum { .. } | RegEntryKind::Ctor { .. }
    ) {
        return Err(rerr(
            ln,
            format!("'{}' is not an enum or constructor carrier", word),
        ));
    }
    Ok(RegType::Named(reg.ents[index].name.clone()))
}

fn parse_signature(reg: &Registry, word: &str, ln: i32) -> Result<CallableSignature, Diag> {
    let Some(text) = word.strip_prefix("sig:") else {
        return Err(rerr(ln, format!("expected sig:<inputs>-><output>, got '{}'", word)));
    };
    let Some((input_text, output_text)) = text.split_once("->") else {
        return Err(rerr(ln, format!("bad callable signature '{}'", text)));
    };
    if input_text.is_empty() || output_text.is_empty() || output_text.contains("->") {
        return Err(rerr(ln, format!("bad callable signature '{}'", text)));
    }
    let inputs = if input_text == "unit" {
        Vec::new()
    } else {
        input_text
            .split(',')
            .map(|carrier| {
                if carrier.is_empty() || carrier == "unit" {
                    Err(rerr(ln, format!("bad callable signature '{}'", text)))
                } else {
                    parse_reg_type(reg, carrier, false, ln)
                }
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    let output = parse_reg_type(reg, output_text, true, ln)?;
    Ok(CallableSignature { inputs, output })
}

fn parse_effects(word: &str, ln: i32) -> Result<EffectSet, Diag> {
    let Some(text) = word.strip_prefix("fx:") else {
        return Err(rerr(ln, format!("expected fx:<effect-row>, got '{}'", word)));
    };
    if text == "pure" {
        return Ok(EffectSet::default());
    }
    let mut effects = EffectSet::default();
    for effect in text.split(',') {
        let slot = match effect {
            "read" => &mut effects.read,
            "write" => &mut effects.write,
            "service" => &mut effects.service,
            _ => return Err(rerr(ln, format!("unknown callable effect '{}'", effect))),
        };
        if std::mem::replace(slot, true) {
            return Err(rerr(ln, format!("duplicate callable effect '{}'", effect)));
        }
    }
    Ok(effects)
}

fn parse_determinism(word: &str, ln: i32) -> Result<Determinism, Diag> {
    let Some(text) = word.strip_prefix("det:") else {
        return Err(rerr(ln, format!("expected det:<boundary>, got '{}'", word)));
    };
    match text {
        "deterministic" => Ok(Determinism::Deterministic),
        "snapshot" => Ok(Determinism::Snapshot),
        "nondeterministic" => Ok(Determinism::Nondeterministic),
        _ => Err(rerr(ln, format!("unknown determinism boundary '{}'", text))),
    }
}

fn parse_trust(word: &str, ln: i32) -> Result<TrustBoundary, Diag> {
    let Some(text) = word.strip_prefix("trust:") else {
        return Err(rerr(ln, format!("expected trust:<boundary>, got '{}'", word)));
    };
    match text {
        "checked" => Ok(TrustBoundary::Checked),
        "trusted" => Ok(TrustBoundary::Trusted),
        _ => Err(rerr(ln, format!("unknown trust boundary '{}'", text))),
    }
}

#[derive(Clone, Copy)]
enum FootprintKind {
    Read,
    Write,
    Service,
}

fn footprint_admits(entry: &RegEntry, kind: FootprintKind) -> bool {
    match kind {
        FootprintKind::Read => matches!(
            entry.kind,
            RegEntryKind::Col { .. }
                | RegEntryKind::Field { .. }
                | RegEntryKind::Rel { .. }
                | RegEntryKind::SRel { .. }
                | RegEntryKind::AliasMask { .. }
                | RegEntryKind::Bind {
                    kind: BindKind::Mask | BindKind::Vec | BindKind::Num,
                    ..
                }
        ),
        FootprintKind::Write => matches!(
            entry.kind,
            RegEntryKind::Col { uniq: false, .. }
                | RegEntryKind::Field { .. }
        ),
        FootprintKind::Service => matches!(entry.kind, RegEntryKind::Service { .. }),
    }
}

fn parse_footprint(
    reg: &Registry,
    word: &str,
    prefix: &str,
    kind: FootprintKind,
    ln: i32,
) -> Result<Vec<String>, Diag> {
    let Some(text) = word.strip_prefix(prefix) else {
        return Err(rerr(ln, format!("expected {}<names|->, got '{}'", prefix, word)));
    };
    if text == "-" {
        return Ok(Vec::new());
    }
    if text.is_empty() {
        return Err(rerr(ln, format!("{} footprint is empty; spell '-'", &prefix[..prefix.len() - 1])));
    }
    let mut out: Vec<(usize, String)> = Vec::new();
    for spelling in text.split(',') {
        let Some(index) = find_ent(reg, spelling) else {
            return Err(rerr(ln, format!("{} footprint names no declaration '{}'", &prefix[..prefix.len() - 1], spelling)));
        };
        if !footprint_admits(&reg.ents[index], kind) {
            return Err(rerr(
                ln,
                format!(
                    "{} footprint cannot name {} '{}'",
                    &prefix[..prefix.len() - 1],
                    match kind {
                        FootprintKind::Read => "unreadable declaration",
                        FootprintKind::Write => "unwritable declaration",
                        FootprintKind::Service => "non-service declaration",
                    },
                    spelling
                ),
            ));
        }
        let canonical = reg.ents[index].name.clone();
        if out.iter().any(|(_, existing)| names_eq(existing, &canonical)) {
            return Err(rerr(ln, format!("duplicate {} footprint '{}'", &prefix[..prefix.len() - 1], spelling)));
        }
        out.push((index, canonical));
    }
    out.sort_by_key(|(index, _)| *index);
    Ok(out.into_iter().map(|(_, name)| name).collect())
}

fn validate_callable_descriptor(reg: &Registry, name: &str, descriptor: &CallableDescriptor, ln: i32) -> Result<(), Diag> {
    if descriptor.trust != TrustBoundary::Trusted {
        return Err(rerr(ln, format!("fn {}: raw BQN requires trust:trusted", name)));
    }
    for (declared, populated, label) in [
        (descriptor.effects.read, !descriptor.reads.is_empty(), "read"),
        (descriptor.effects.write, !descriptor.writes.is_empty(), "write"),
        (descriptor.effects.service, !descriptor.services.is_empty(), "service"),
    ] {
        if declared != populated {
            return Err(rerr(
                ln,
                format!("fn {}: fx:{} and {} footprint disagree", name, label, label),
            ));
        }
    }
    let service_directions = descriptor
        .services
        .iter()
        .filter_map(|service| {
            find_ent(reg, service).and_then(|index| match &reg.ents[index].kind {
                RegEntryKind::Service { descriptor } => Some(descriptor.direction),
                _ => None,
            })
        })
        .collect::<Vec<_>>();
    match descriptor.determinism {
        Determinism::Deterministic if !descriptor.services.is_empty() => {
            return Err(rerr(ln, format!("fn {}: service use requires det:snapshot or det:nondeterministic", name)));
        }
        Determinism::Snapshot
            if service_directions
                .iter()
                .any(|direction| *direction == ServiceDirection::Output) =>
        {
            return Err(rerr(ln, format!("fn {}: an output service is nondeterministic", name)));
        }
        Determinism::Nondeterministic
            if !service_directions
                .iter()
                .any(|direction| *direction == ServiceDirection::Output) =>
        {
            return Err(rerr(ln, format!("fn {}: det:nondeterministic requires an output service", name)));
        }
        _ => {}
    }
    Ok(())
}

fn parse_domain(word: &str, ln: i32) -> Result<ArrayDomain, Diag> {
    match word {
        "scalar" => Ok(ArrayDomain::Scalar),
        "entity" => Ok(ArrayDomain::Entity),
        _ => {
            let Some(text) = word.strip_prefix("fixed:") else {
                return Err(rerr(ln, format!("unknown array domain '{}'", word)));
            };
            let extent = text
                .parse::<u64>()
                .map_err(|_| rerr(ln, format!("bad fixed array extent '{}'", text)))?;
            Ok(ArrayDomain::Fixed(extent))
        }
    }
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

// Entity bindings store stable keys, never row offsets. Without a declared key column the row
// iota is the key space. Validation is deferred until the complete role table is available.
fn validate_entity_binds(reg: &Registry) -> Result<(), Diag> {
    let key_values = reg_role(reg, "id")
        .or_else(|| reg_role(reg, "keys"))
        .and_then(|index| match &reg.ents[index].kind {
            RegEntryKind::Col { nums, .. } => Some(nums.as_slice()),
            _ => None,
        });
    for entry in &reg.ents {
        let RegEntryKind::Bind { kind: BindKind::Entity, vals } = &entry.kind else {
            continue;
        };
        let key = vals.first().copied().unwrap_or(f64::NAN);
        let found = match key_values {
            Some(keys) => keys.iter().filter(|value| **value == key).count() == 1,
            None => {
                key.is_finite()
                    && key >= 0.0
                    && key.fract() == 0.0
                    && key < reg.n.max(0) as f64
            }
        };
        if !found {
            return Err(rerr(
                0,
                format!(
                    "entity binding '{}' has no unique key {}",
                    entry.name,
                    num::fmt_g(17, key)
                ),
            ));
        }
    }
    Ok(())
}

// Inputs: .reg path. Output: the loaded world (entries in declaration order) or Diag.
// Invariants: two passes (capacity count, then per-line dispatch on the EXACT-BYTE first
// word); name gate order wfree -> wname -> wuniq selects which diagnostic fires; wuniq also
// refuses the twelve reserved emitter identifiers and the anoRelStage<n> family under the fold;
// forward-only references (n before cols, lattice before fields, rel before inv); the value
// domain is checked per carrier after the extended-real wnum choke point; num admits infinity;
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
                let Some(v) = v.filter(|&v| {
                    v.is_finite()
                        && v >= 0.0
                        && v <= i32::MAX as f64
                        && v.fract() == 0.0
                }) else {
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
                let Some((w, h)) = wh.filter(|&(w, h)| {
                    w.is_finite()
                        && h.is_finite()
                        && w >= 0.0
                        && h >= 0.0
                        && w <= i32::MAX as f64
                        && h <= i32::MAX as f64
                        && w.fract() == 0.0
                        && h.fract() == 0.0
                }) else {
                    return Err(rerr(ln, "usage: lattice <w> <h>".into()));
                };
                if saw_lat {
                    return Err(rerr(ln, "lattice redeclared".into()));
                }
                saw_lat = true;
                reg.lat_w = w as i32;
                reg.lat_h = h as i32;
            }

            "array" => {
                if nw != 6 {
                    return Err(rerr(
                        ln,
                        "usage: array <name> id:<16-hex> v:<n> <carrier> <scalar|entity|fixed:N>".into(),
                    ));
                }
                let name = gate_name(&reg, words[1].1, ln)?;
                let meta = declaration_meta(&reg, words[2].1, words[3].1, ln)?;
                let carrier = parse_reg_type(&reg, words[4].1, false, ln)?;
                let domain = parse_domain(words[5].1, ln)?;
                reg.ents.push(RegEntry {
                    name,
                    defval: 0.0,
                    kind: RegEntryKind::Array {
                        descriptor: ArrayDescriptor { meta, carrier, domain },
                    },
                });
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
                if nums.iter().any(|value| !value.is_finite()) {
                    return Err(rerr(
                        ln,
                        format!("unique {}: keys must be finite", name),
                    ));
                }
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
                if mask.iter().any(|value| *value != 0.0 && *value != 1.0) {
                    return Err(rerr(
                        ln,
                        format!("pres {}: mask values must be 0 or 1", reg.ents[idx].name),
                    ));
                }
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
                if reg.ents[idx].kind.declaration_meta().is_some() {
                    return Err(rerr(
                        ln,
                        format!("default on high-integrity declaration '{}'", reg.ents[idx].name),
                    ));
                }
                // a unique column's values are minted, never defaulted: a shared default is
                // a standing violation of the declared injectivity
                if is_uniq_col(&reg.ents[idx]) {
                    return Err(rerr(ln, format!("default on unique column '{}'", reg.ents[idx].name)));
                }
                // the default is the spawn fill: it must sit in the column's carrier set
                if let RegEntryKind::Col { ty, rng, .. } | RegEntryKind::Field { ty, rng, .. } = &reg.ents[idx].kind {
                    let name = &reg.ents[idx].name;
                    if !type_admits(*ty, v) {
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
                    if vals.iter().any(|value| *value != 0.0 && *value != 1.0) {
                        return Err(rerr(ln, format!("alias {}: mask values must be 0 or 1", name)));
                    }
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
                if reg.n == 0 && words.len() == ni + 1 {
                    fib.clear();
                }
                if fib.len() != reg.n.max(0) as usize {
                    return Err(rerr(
                        ln,
                        format!("srel {}: expected {} fibers, got {}", name, reg.n.max(0), fib.len()),
                    ));
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
                match kind {
                    BindKind::Entity if !vals[0].is_finite() => {
                        return Err(rerr(ln, format!("bind {}: entity key must be finite", name)));
                    }
                    BindKind::Point if vals.iter().any(|value| !value.is_finite()) => {
                        return Err(rerr(ln, format!("bind {}: point values must be finite", name)));
                    }
                    BindKind::Mask
                        if vals.iter().any(|value| *value != 0.0 && *value != 1.0) =>
                    {
                        return Err(rerr(ln, format!("bind {}: mask values must be 0 or 1", name)));
                    }
                    _ => {}
                }
                reg.ents.push(RegEntry { name, defval: 0.0, kind: RegEntryKind::Bind { kind, vals } });
            }

            "service" => {
                if nw != 7 {
                    return Err(rerr(
                        ln,
                        "usage: service <name> id:<16-hex> v:<n> <input|output> sig:<...> trust:<checked|trusted>".into(),
                    ));
                }
                let name = gate_name(&reg, words[1].1, ln)?;
                let meta = declaration_meta(&reg, words[2].1, words[3].1, ln)?;
                let direction = match words[4].1 {
                    "input" => ServiceDirection::Input,
                    "output" => ServiceDirection::Output,
                    word => return Err(rerr(ln, format!("unknown service direction '{}'", word))),
                };
                let signature = parse_signature(&reg, words[5].1, ln)?;
                let trust = parse_trust(words[6].1, ln)?;
                if direction == ServiceDirection::Input && signature.output == RegType::Unit {
                    return Err(rerr(ln, format!("service {}: an input service must return a value", name)));
                }
                if direction == ServiceDirection::Output && signature.output != RegType::Unit {
                    return Err(rerr(ln, format!("service {}: an output service must return unit", name)));
                }
                reg.ents.push(RegEntry {
                    name,
                    defval: 0.0,
                    kind: RegEntryKind::Service {
                        descriptor: ServiceDescriptor { meta, direction, signature, trust },
                    },
                });
            }

            "enum" => {
                if nw < 6 || !words[nw - 1].1.starts_with("reserve:") {
                    return Err(rerr(
                        ln,
                        "usage: enum <name> id:<16-hex> v:<n> <Case=u32>... reserve:<u32,...|->".into(),
                    ));
                }
                let name = gate_name(&reg, words[1].1, ln)?;
                validate_nominal_name(&name, ln)?;
                let meta = declaration_meta(&reg, words[2].1, words[3].1, ln)?;
                let mut cases: Vec<EnumCase> = Vec::new();
                for &(_, case_word) in &words[4..nw - 1] {
                    let Some((case_name, discriminant)) = case_word.split_once('=') else {
                        return Err(rerr(ln, format!("enum {}: case '{}' is not <name>=<u32>", name, case_word)));
                    };
                    let case_name = wname(case_name, ANO_NAMESZ, ln)?;
                    let discriminant = discriminant
                        .parse::<u32>()
                        .map_err(|_| rerr(ln, format!("enum {}: bad discriminant '{}'", name, discriminant)))?;
                    if cases.iter().any(|case| names_eq(&case.name, &case_name)) {
                        return Err(rerr(ln, format!("enum {}: duplicate case '{}'", name, case_name)));
                    }
                    if cases.iter().any(|case| case.discriminant == discriminant) {
                        return Err(rerr(ln, format!("enum {}: discriminant {} is reused", name, discriminant)));
                    }
                    cases.push(EnumCase { name: case_name, discriminant });
                }
                let reserve_word = words[nw - 1].1;
                let reserve_text = reserve_word.strip_prefix("reserve:").unwrap();
                let mut reserved: Vec<u32> = Vec::new();
                if reserve_text != "-" {
                    if reserve_text.is_empty() {
                        return Err(rerr(ln, format!("enum {}: empty reserve list must be '-'", name)));
                    }
                    for spelling in reserve_text.split(',') {
                        let discriminant = spelling
                            .parse::<u32>()
                            .map_err(|_| rerr(ln, format!("enum {}: bad reserved discriminant '{}'", name, spelling)))?;
                        if reserved.contains(&discriminant) {
                            return Err(rerr(ln, format!("enum {}: discriminant {} is reserved twice", name, discriminant)));
                        }
                        if cases.iter().any(|case| case.discriminant == discriminant) {
                            return Err(rerr(ln, format!("enum {}: live discriminant {} is also reserved", name, discriminant)));
                        }
                        reserved.push(discriminant);
                    }
                    reserved.sort_unstable();
                }
                reg.ents.push(RegEntry {
                    name,
                    defval: 0.0,
                    kind: RegEntryKind::Enum {
                        descriptor: EnumDescriptor { meta, cases, reserved },
                    },
                });
            }

            "ctor" => {
                if nw != 5 {
                    return Err(rerr(
                        ln,
                        "usage: ctor <name> id:<16-hex> v:<n> <range:lo..hi|enum:name>".into(),
                    ));
                }
                let name = gate_name(&reg, words[1].1, ln)?;
                validate_nominal_name(&name, ln)?;
                let meta = declaration_meta(&reg, words[2].1, words[3].1, ln)?;
                let refinement = if let Some(range) = words[4].1.strip_prefix("range:") {
                    let Some((lo_word, hi_word)) = range.split_once("..") else {
                        return Err(rerr(ln, format!("ctor {}: bad range '{}'", name, range)));
                    };
                    let lo = num::wnum(lo_word)
                        .filter(|value| value.is_finite())
                        .ok_or_else(|| rerr(ln, format!("ctor {}: bad finite lower bound '{}'", name, lo_word)))?;
                    let hi = num::wnum(hi_word)
                        .filter(|value| value.is_finite())
                        .ok_or_else(|| rerr(ln, format!("ctor {}: bad finite upper bound '{}'", name, hi_word)))?;
                    if lo > hi {
                        return Err(rerr(ln, format!("ctor {}: lower bound is above upper bound", name)));
                    }
                    ConstructorRefinement::Range { lo, hi }
                } else if let Some(enum_name) = words[4].1.strip_prefix("enum:") {
                    let Some(index) = find_ent(&reg, enum_name) else {
                        return Err(rerr(ln, format!("ctor {}: no enum '{}'", name, enum_name)));
                    };
                    if !matches!(reg.ents[index].kind, RegEntryKind::Enum { .. }) {
                        return Err(rerr(ln, format!("ctor {}: '{}' is not an enum", name, enum_name)));
                    }
                    ConstructorRefinement::Enum {
                        enumeration: reg.ents[index].name.clone(),
                    }
                } else {
                    return Err(rerr(ln, format!("ctor {}: expected range:lo..hi or enum:name", name)));
                };
                reg.ents.push(RegEntry {
                    name,
                    defval: 0.0,
                    kind: RegEntryKind::Ctor {
                        descriptor: ConstructorDescriptor { meta, refinement },
                    },
                });
            }

            "fn" => {
                if nw < 2 {
                    return Err(rerr(ln, "usage: fn <name> [bqn]".into()));
                }
                let name = gate_name(&reg, words[1].1, ln)?;
                if nw >= 3 && words[2].1.starts_with("id:") {
                    if nw < 13 || words[11].1 != "=" {
                        return Err(rerr(
                            ln,
                            "usage: fn <name> id:<16-hex> v:<n> sig:<...> fx:<...> det:<...> trust:<...> read:<...> write:<...> use:<...> = <bqn-dfn>".into(),
                        ));
                    }
                    let meta = declaration_meta(&reg, words[2].1, words[3].1, ln)?;
                    let signature = parse_signature(&reg, words[4].1, ln)?;
                    let effects = parse_effects(words[5].1, ln)?;
                    let determinism = parse_determinism(words[6].1, ln)?;
                    let trust = parse_trust(words[7].1, ln)?;
                    let reads = parse_footprint(&reg, words[8].1, "read:", FootprintKind::Read, ln)?;
                    let writes = parse_footprint(&reg, words[9].1, "write:", FootprintKind::Write, ln)?;
                    let services = parse_footprint(&reg, words[10].1, "use:", FootprintKind::Service, ln)?;
                    let body = line[words[12].0..].to_string();
                    if !body.starts_with('{') || !body.ends_with('}') {
                        return Err(rerr(ln, format!("fn {}: typed raw BQN must be one brace-delimited dfn", name)));
                    }
                    let descriptor = CallableDescriptor {
                        meta,
                        signature,
                        effects,
                        determinism,
                        trust,
                        reads,
                        writes,
                        services,
                    };
                    validate_callable_descriptor(&reg, &name, &descriptor, ln)?;
                    reg.ents.push(RegEntry {
                        name,
                        defval: 0.0,
                        kind: RegEntryKind::TypedFn { body, descriptor },
                    });
                } else {
                    let body = raw2.map(str::to_string);
                    reg.ents.push(RegEntry { name, defval: 0.0, kind: RegEntryKind::Fn { body } });
                }
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
                        if !type_admits(cty, num_v) {
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
    for alias in &reg.aliases {
        if find_ent(&reg, &alias.to).is_none() {
            return Err(rerr(
                0,
                format!("spelling alias '{}' has no declared target '{}'", alias.from, alias.to),
            ));
        }
    }
    validate_registry_contracts(&reg)?;
    validate_entity_binds(&reg)?;
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
// High-integrity declarations must survive construction outside the text loader with the same
// guarantees as parsed declarations. References are canonical and backward so a dump reloads.
fn declaration_before<'a>(
    reg: &'a Registry,
    owner: usize,
    name: &str,
    label: &str,
) -> Result<(usize, &'a RegEntry), Diag> {
    let Some(index) = find_ent(reg, name) else {
        return Err(rerr(0, format!("{} names no declaration '{}'", label, name)));
    };
    if index >= owner {
        return Err(rerr(0, format!("{} must name an earlier declaration '{}'", label, name)));
    }
    if reg.ents[index].name != name {
        return Err(rerr(
            0,
            format!(
                "{} must use canonical spelling '{}', got '{}'",
                label, reg.ents[index].name, name
            ),
        ));
    }
    Ok((index, &reg.ents[index]))
}

fn validate_reg_type(
    reg: &Registry,
    owner: usize,
    carrier: &RegType,
    unit: bool,
    label: &str,
) -> Result<(), Diag> {
    match carrier {
        RegType::Unit if !unit => {
            Err(rerr(0, format!("{} cannot use unit as an input carrier", label)))
        }
        RegType::Named(name) => {
            let (_, entry) = declaration_before(reg, owner, name, label)?;
            if !matches!(entry.kind, RegEntryKind::Enum { .. } | RegEntryKind::Ctor { .. }) {
                return Err(rerr(
                    0,
                    format!("{} '{}' is not an enum or constructor carrier", label, name),
                ));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_signature_contract(
    reg: &Registry,
    owner: usize,
    signature: &CallableSignature,
    label: &str,
) -> Result<(), Diag> {
    for carrier in &signature.inputs {
        validate_reg_type(reg, owner, carrier, false, label)?;
    }
    validate_reg_type(reg, owner, &signature.output, true, label)
}

fn validate_footprint_contract(
    reg: &Registry,
    owner: usize,
    names: &[String],
    kind: FootprintKind,
    label: &str,
) -> Result<(), Diag> {
    let mut previous = None;
    for (position, name) in names.iter().enumerate() {
        if names[..position].iter().any(|prior| names_eq(prior, name)) {
            return Err(rerr(0, format!("duplicate {} footprint '{}'", label, name)));
        }
        let (reference, entry) = declaration_before(reg, owner, name, label)?;
        if previous.is_some_and(|prior| reference <= prior) {
            return Err(rerr(
                0,
                format!("{} footprint must follow declaration order", label),
            ));
        }
        previous = Some(reference);
        if !footprint_admits(entry, kind) {
            return Err(rerr(
                0,
                format!("{} footprint cannot name declaration '{}'", label, name),
            ));
        }
    }
    Ok(())
}

fn validate_high_name(reg: &Registry, index: usize) -> Result<(), Diag> {
    let name = &reg.ents[index].name;
    wfree(name, 0)?;
    wname(name, ANO_NAMESZ, 0)?;
    if name.starts_with('#')
        || name.bytes().any(|byte| byte == 0 || byte.is_ascii_whitespace())
    {
        return Err(rerr(0, format!("declaration name '{}' is not one word", name)));
    }
    for reserved in RESV {
        if names_eq(name, reserved) {
            return Err(rerr(
                0,
                format!("'{}' collides with the emitter's reserved '{}'", name, reserved),
            ));
        }
    }
    for prefix in RESVPFX {
        if name.get(..prefix.len()).is_some_and(|head| names_eq(head, prefix)) {
            return Err(rerr(
                0,
                format!("'{}' collides with the emitter's reserved '{}<n>' names", name, prefix),
            ));
        }
    }
    for (other, entry) in reg.ents.iter().enumerate() {
        if other != index && names_eq(name, &entry.name) {
            return Err(rerr(
                0,
                format!("'{}' collides with entry '{}'", name, entry.name),
            ));
        }
    }
    if let Some(alias) = reg.aliases.iter().find(|alias| names_eq(name, &alias.from)) {
        return Err(rerr(
            0,
            format!("'{}' collides with alias source '{}'", name, alias.from),
        ));
    }
    Ok(())
}

fn validate_enum_contract(name: &str, descriptor: &EnumDescriptor) -> Result<(), Diag> {
    if descriptor.cases.is_empty() {
        return Err(rerr(0, format!("enum {}: at least one live case is required", name)));
    }
    for (index, case) in descriptor.cases.iter().enumerate() {
        wname(&case.name, ANO_NAMESZ, 0)?;
        if case.name.starts_with('#')
            || case.name.contains('=')
            || case.name.bytes().any(|byte| byte == 0 || byte.is_ascii_whitespace())
        {
            return Err(rerr(
                0,
                format!("enum {}: case '{}' is not one case word", name, case.name),
            ));
        }
        if descriptor.cases[..index]
            .iter()
            .any(|prior| names_eq(&prior.name, &case.name))
        {
            return Err(rerr(0, format!("enum {}: duplicate case '{}'", name, case.name)));
        }
        if descriptor.cases[..index]
            .iter()
            .any(|prior| prior.discriminant == case.discriminant)
        {
            return Err(rerr(
                0,
                format!("enum {}: discriminant {} is reused", name, case.discriminant),
            ));
        }
        if descriptor.reserved.contains(&case.discriminant) {
            return Err(rerr(
                0,
                format!(
                    "enum {}: live discriminant {} is also reserved",
                    name, case.discriminant
                ),
            ));
        }
    }
    if descriptor
        .reserved
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(rerr(
            0,
            format!("enum {}: reserved discriminants must be strictly ascending", name),
        ));
    }
    Ok(())
}

fn validate_meta_contract(
    reg: &Registry,
    index: usize,
    name: &str,
    meta: DeclMeta,
) -> Result<(), Diag> {
    if meta.id.0 == 0 {
        return Err(rerr(0, format!("declaration '{}' uses reserved id zero", name)));
    }
    if meta.version == 0 {
        return Err(rerr(0, format!("declaration '{}' uses version zero", name)));
    }
    if let Some(owner) = reg.ents[..index].iter().find(|entry| {
        entry
            .kind
            .declaration_meta()
            .is_some_and(|other| other.id == meta.id)
    }) {
        return Err(rerr(
            0,
            format!(
                "declaration id {:016x} is already owned by '{}'",
                meta.id.0, owner.name
            ),
        ));
    }
    Ok(())
}

/// Validate every explicit registry capability, including registries assembled by host code.
pub fn validate_registry_contracts(reg: &Registry) -> Result<(), Diag> {
    for (index, entry) in reg.ents.iter().enumerate() {
        validate_high_name(reg, index)?;
        let Some(meta) = entry.kind.declaration_meta() else {
            continue;
        };
        if matches!(entry.kind, RegEntryKind::Enum { .. } | RegEntryKind::Ctor { .. }) {
            validate_nominal_name(&entry.name, 0)?;
        }

        validate_meta_contract(reg, index, &entry.name, meta)?;
        if entry.defval != 0.0 {
            return Err(rerr(
                0,
                format!("default on high-integrity declaration '{}'", entry.name),
            ));
        }
        match &entry.kind {
            RegEntryKind::Array { descriptor } => {
                validate_reg_type(reg, index, &descriptor.carrier, false, "array carrier")?;
                if let ArrayDomain::Fixed(extent) = descriptor.domain {
                    usize::try_from(extent).map_err(|_| {
                        rerr(
                            0,
                            format!("array {}: fixed extent {} is not addressable", entry.name, extent),
                        )
                    })?;
                }
            }
            RegEntryKind::Service { descriptor } => {
                validate_signature_contract(reg, index, &descriptor.signature, "service signature")?;
                match (descriptor.direction, &descriptor.signature.output) {
                    (ServiceDirection::Input, RegType::Unit) => {
                        return Err(rerr(
                            0,
                            format!("service {}: an input service must return a value", entry.name),
                        ));
                    }
                    (ServiceDirection::Output, output) if *output != RegType::Unit => {
                        return Err(rerr(
                            0,
                            format!("service {}: an output service must return unit", entry.name),
                        ));
                    }
                    _ => {}
                }
            }
            RegEntryKind::Enum { descriptor } => {
                validate_enum_contract(&entry.name, descriptor)?;
            }
            RegEntryKind::Ctor { descriptor } => match &descriptor.refinement {
                ConstructorRefinement::Range { lo, hi } => {
                    if !lo.is_finite() || !hi.is_finite() || lo > hi {
                        return Err(rerr(
                            0,
                            format!("ctor {}: range must be finite and ordered", entry.name),
                        ));
                    }
                }
                ConstructorRefinement::Enum { enumeration } => {
                    let (_, target) =
                        declaration_before(reg, index, enumeration, "constructor enum")?;
                    if !matches!(target.kind, RegEntryKind::Enum { .. }) {
                        return Err(rerr(
                            0,
                            format!("ctor {}: '{}' is not an enum", entry.name, enumeration),
                        ));
                    }
                }
            },
            RegEntryKind::TypedFn { body, descriptor } => {
                validate_signature_contract(reg, index, &descriptor.signature, "fn signature")?;
                if matches!(descriptor.signature.output, RegType::Named(_)) {
                    return Err(rerr(
                        0,
                        format!("fn {}: nominal outputs require a checked constructor", entry.name),
                    ));
                }
                validate_footprint_contract(
                    reg,
                    index,
                    &descriptor.reads,
                    FootprintKind::Read,
                    "read",
                )?;
                validate_footprint_contract(
                    reg,
                    index,
                    &descriptor.writes,
                    FootprintKind::Write,
                    "write",
                )?;
                validate_footprint_contract(
                    reg,
                    index,
                    &descriptor.services,
                    FootprintKind::Service,
                    "service",
                )?;
                if !body.starts_with('{')
                    || !body.ends_with('}')
                    || body.bytes().any(|byte| matches!(byte, 0 | b'\n' | b'\r' | b'#'))
                {
                    return Err(rerr(
                        0,
                        format!("fn {}: body must be one comment-free BQN dfn", entry.name),
                    ));
                }
                validate_callable_descriptor(reg, &entry.name, descriptor, 0)?;
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}

fn entity_key_exists(reg: &Registry, value: f64) -> bool {
    let key_values = reg_role(reg, "id")
        .or_else(|| reg_role(reg, "keys"))
        .and_then(|index| match &reg.ents[index].kind {
            RegEntryKind::Col { nums, .. } => Some(nums.as_slice()),
            _ => None,
        });
    match key_values {
        Some(keys) => keys.iter().filter(|key| **key == value).count() == 1,
        None => {
            value.is_finite()
                && value >= 0.0
                && value.fract() == 0.0
                && value < reg.n.max(0) as f64
        }
    }
}

fn resident_len(value: &ResidentArrayValue) -> usize {
    match value {
        ResidentArrayValue::Numbers(values) => values.len(),
        ResidentArrayValue::Symbols(values) => values.len(),
        ResidentArrayValue::Characters(values) => values.len(),
        ResidentArrayValue::Entities(values) => values.len(),
        ResidentArrayValue::Discriminants(values) => values.len(),
    }
}

fn resident_kind(value: &ResidentArrayValue) -> &'static str {
    match value {
        ResidentArrayValue::Numbers(_) => "numbers",
        ResidentArrayValue::Symbols(_) => "symbols",
        ResidentArrayValue::Characters(_) => "characters",
        ResidentArrayValue::Entities(_) => "entities",
        ResidentArrayValue::Discriminants(_) => "discriminants",
    }
}

fn live_enum_discriminant(
    enum_name: &str,
    descriptor: &EnumDescriptor,
    discriminant: u32,
) -> Result<u32, Diag> {
    if descriptor
        .cases
        .iter()
        .any(|case| case.discriminant == discriminant)
    {
        Ok(discriminant)
    } else {
        Err(rerr(
            0,
            format!(
                "enum {}: discriminant {} is not a live case",
                enum_name, discriminant
            ),
        ))
    }
}

fn seal_enum_array(
    enum_name: &str,
    descriptor: &EnumDescriptor,
    value: ResidentArrayValue,
) -> Result<ResidentArrayValue, Diag> {
    let discriminants = match value {
        ResidentArrayValue::Symbols(values) => values
            .into_iter()
            .map(|value| {
                descriptor
                    .cases
                    .iter()
                    .find(|case| names_eq(&case.name, &value))
                    .map(|case| case.discriminant)
                    .ok_or_else(|| {
                        rerr(0, format!("enum {}: no live case '{}'", enum_name, value))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?,
        ResidentArrayValue::Discriminants(values) => values
            .into_iter()
            .map(|value| live_enum_discriminant(enum_name, descriptor, value))
            .collect::<Result<Vec<_>, _>>()?,
        other => {
            return Err(rerr(
                0,
                format!(
                    "enum {}: expected symbols or discriminants, got {}",
                    enum_name,
                    resident_kind(&other)
                ),
            ));
        }
    };
    Ok(ResidentArrayValue::Discriminants(discriminants))
}

fn seal_numeric_array(
    name: &str,
    carrier: &RegType,
    value: ResidentArrayValue,
    range: Option<(f64, f64)>,
) -> Result<ResidentArrayValue, Diag> {
    let ResidentArrayValue::Numbers(mut values) = value else {
        return Err(rerr(
            0,
            format!("array {}: carrier {} requires numbers", name, reg_type_word(carrier)),
        ));
    };
    let primitive = match carrier {
        RegType::Mask => ColType::Bool,
        RegType::Nat => ColType::Nat,
        RegType::Int => ColType::Int,
        RegType::Num | RegType::Named(_) => ColType::Num,
        _ => unreachable!(),
    };
    for value in &mut values {
        if !type_admits(primitive, *value)
            || range.is_some_and(|(lo, hi)| *value < lo || *value > hi)
        {
            return Err(rerr(
                0,
                format!(
                    "array {}: value {} is outside {}",
                    name,
                    num::fmt_g(17, *value),
                    reg_type_word(carrier)
                ),
            ));
        }
        if *value == 0.0 {
            *value = 0.0;
        }
    }
    Ok(ResidentArrayValue::Numbers(values))
}

fn seal_array_value(
    reg: &Registry,
    name: &str,
    descriptor: &ArrayDescriptor,
    value: ResidentArrayValue,
) -> Result<ResidentArrayValue, Diag> {
    let expected = match descriptor.domain {
        ArrayDomain::Scalar => 1,
        ArrayDomain::Entity => reg.n.max(0) as usize,
        ArrayDomain::Fixed(extent) => usize::try_from(extent)
            .map_err(|_| rerr(0, format!("array {}: extent is not addressable", name)))?,
    };
    if resident_len(&value) != expected {
        return Err(rerr(
            0,
            format!(
                "array {}: expected {} values for its domain, got {}",
                name,
                expected,
                resident_len(&value)
            ),
        ));
    }
    match &descriptor.carrier {
        RegType::Mask | RegType::Nat | RegType::Int | RegType::Num => {
            seal_numeric_array(name, &descriptor.carrier, value, None)
        }
        RegType::Sym => match value {
            ResidentArrayValue::Symbols(values) => Ok(ResidentArrayValue::Symbols(values)),
            other => Err(rerr(
                0,
                format!("array {}: carrier sym cannot seal {}", name, resident_kind(&other)),
            )),
        },
        RegType::Char => match value {
            ResidentArrayValue::Characters(values) => {
                Ok(ResidentArrayValue::Characters(values))
            }
            other => Err(rerr(
                0,
                format!("array {}: carrier char cannot seal {}", name, resident_kind(&other)),
            )),
        },
        RegType::Entity => match value {
            ResidentArrayValue::Entities(mut values) => {
                for value in &mut values {
                    if !entity_key_exists(reg, *value) {
                        return Err(rerr(
                            0,
                            format!(
                                "array {}: entity key {} is not live and unique",
                                name,
                                num::fmt_g(17, *value)
                            ),
                        ));
                    }
                    if *value == 0.0 {
                        *value = 0.0;
                    }
                }
                Ok(ResidentArrayValue::Entities(values))
            }
            other => Err(rerr(
                0,
                format!(
                    "array {}: carrier entity cannot seal {}",
                    name,
                    resident_kind(&other)
                ),
            )),
        },
        RegType::Named(type_name) => {
            let Some(index) = find_ent(reg, type_name) else {
                return Err(rerr(0, format!("array {}: missing carrier '{}'", name, type_name)));
            };
            match &reg.ents[index].kind {
                RegEntryKind::Enum { descriptor } => {
                    seal_enum_array(type_name, descriptor, value)
                }
                RegEntryKind::Ctor { descriptor: constructor } => match &constructor.refinement {
                    ConstructorRefinement::Range { lo, hi } => seal_numeric_array(
                        name,
                        &descriptor.carrier,
                        value,
                        Some((*lo, *hi)),
                    ),
                    ConstructorRefinement::Enum { enumeration } => {
                        let Some(enum_index) = find_ent(reg, enumeration) else {
                            return Err(rerr(
                                0,
                                format!("array {}: missing enum '{}'", name, enumeration),
                            ));
                        };
                        let RegEntryKind::Enum { descriptor } = &reg.ents[enum_index].kind else {
                            return Err(rerr(
                                0,
                                format!("array {}: '{}' is not an enum", name, enumeration),
                            ));
                        };
                        seal_enum_array(enumeration, descriptor, value)
                    }
                },
                _ => Err(rerr(
                    0,
                    format!("array {}: '{}' is not a nominal carrier", name, type_name),
                )),
            }
        }
        RegType::Unit => Err(rerr(0, format!("array {}: unit has no resident values", name))),
    }
}

const RESIDENT_ARRAY_MAGIC: &str = "ano-resident-array-v1";
const CONSTRUCTED_VALUE_MAGIC: &str = "ano-constructed-value-v1";

fn sidecar_error(label: &str, message: impl Into<String>) -> Diag {
    rerr(0, format!("{} sidecar: {}", label, message.into()))
}

fn hex_encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(DIGITS[(byte >> 4) as usize] as char);
        encoded.push(DIGITS[(byte & 15) as usize] as char);
    }
    encoded
}

fn canonical_hex(word: &str) -> bool {
    word.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn hex_decode(word: &str, label: &str) -> Result<Vec<u8>, Diag> {
    if word.len() % 2 != 0 || !canonical_hex(word) {
        return Err(sidecar_error(label, format!("bad hexadecimal payload '{}'", word)));
    }
    let mut decoded = Vec::with_capacity(word.len() / 2);
    for pair in word.as_bytes().chunks_exact(2) {
        let pair = std::str::from_utf8(pair).unwrap();
        decoded.push(u8::from_str_radix(pair, 16).unwrap());
    }
    Ok(decoded)
}

fn fixed_hex(word: &str, width: usize, label: &str) -> Result<u64, Diag> {
    if word.len() != width || !canonical_hex(word) {
        return Err(sidecar_error(label, format!("bad {}-digit hexadecimal value '{}'", width, word)));
    }
    u64::from_str_radix(word, 16)
        .map_err(|_| sidecar_error(label, format!("bad hexadecimal value '{}'", word)))
}

fn sidecar_fields<'a>(
    bytes: &'a [u8],
    magic: &str,
    label: &str,
) -> Result<Vec<&'a str>, Diag> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| sidecar_error(label, "payload is not UTF-8"))?;
    let Some(text) = text.strip_suffix('\n') else {
        return Err(sidecar_error(label, "missing canonical final newline"));
    };
    if text.bytes().any(|byte| matches!(byte, b'\n' | b'\r')) {
        return Err(sidecar_error(label, "payload must be exactly one line"));
    }
    let fields = text.split('\t').collect::<Vec<_>>();
    if fields.len() < 6 || fields[0] != magic {
        return Err(sidecar_error(label, "unsupported or malformed header"));
    }
    Ok(fields)
}

fn sidecar_stamp(fields: &[&str], label: &str) -> Result<(DeclarationStamp, String), Diag> {
    let schema = fixed_hex(fields[1], 16, label)?;
    let declaration = fixed_hex(fields[2], 16, label)?;
    let version = fields[3]
        .parse::<u64>()
        .ok()
        .filter(|version| *version > 0)
        .ok_or_else(|| sidecar_error(label, format!("bad declaration version '{}'", fields[3])))?;
    if fields[3] != version.to_string() {
        return Err(sidecar_error(label, format!("noncanonical declaration version '{}'", fields[3])));
    }

    if fields[4].is_empty() {
        return Err(sidecar_error(label, "empty declaration name"));
    }
    Ok((
        DeclarationStamp {
            schema,
            declaration: DeclId(declaration),
            version,
        },
        fields[4].to_string(),
    ))
}

fn write_sidecar(label: &str, path: &str, bytes: &[u8]) -> Result<(), Diag> {
    fs::fs_write_commit(path, bytes).map_err(|error| {
        rerr(
            0,
            format!(
                "cannot write {} sidecar '{}': {}",
                label,
                path,
                fs::strerror(&error)
            ),
        )
    })
}

fn read_sidecar(label: &str, path: &str) -> Result<Vec<u8>, Diag> {
    fs::fs_read(path).map_err(|error| {
        rerr(
            0,
            format!("cannot read {} sidecar '{}': {}", label, path, fs::strerror(&error)),
        )
    })
}

/// Seal a host attachment against the array's carrier, domain, identity, and schema.
pub fn seal_resident_array(
    reg: &Registry,
    name: &str,
    value: ResidentArrayValue,
) -> Result<ResidentArrayHandle, Diag> {
    validate_registry_contracts(reg)?;
    let Some(index) = reg_find(reg, name) else {
        return Err(rerr(0, format!("no resident array '{}'", name)));
    };
    let RegEntryKind::Array { descriptor } = &reg.ents[index].kind else {
        return Err(rerr(0, format!("'{}' is not a resident array", name)));
    };
    let value = seal_array_value(reg, &reg.ents[index].name, descriptor, value)?;
    Ok(ResidentArrayHandle {
        stamp: DeclarationStamp {
            schema: crate::alias::registry_fingerprint(reg),
            declaration: descriptor.meta.id,
            version: descriptor.meta.version,
        },
        name: reg.ents[index].name.clone(),
        value,
    })
}

impl ResidentArrayHandle {
    /// Return the schema, declaration, and version proof carried by this handle.
    pub fn stamp(&self) -> DeclarationStamp {
        self.stamp
    }

    /// Return the canonical declaration name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Borrow the normalized, sealed payload.
    pub fn value(&self) -> &ResidentArrayValue {
        &self.value
    }

    pub fn validate(&self, reg: &Registry) -> Result<(), Diag> {
        validate_registry_contracts(reg)?;
        let schema = crate::alias::registry_fingerprint(reg);
        if self.stamp.schema != schema {
            return Err(rerr(
                0,
                format!("resident array '{}' belongs to a stale schema", self.name),
            ));
        }
        let Some((entry, descriptor)) = reg.ents.iter().find_map(|entry| match &entry.kind {
            RegEntryKind::Array { descriptor }
                if descriptor.meta.id == self.stamp.declaration =>
            {
                Some((entry, descriptor))
            }
            _ => None,
        }) else {
            return Err(rerr(
                0,
                format!("resident array '{}' declaration is missing", self.name),
            ));
        };
        if entry.name != self.name {
            return Err(rerr(
                0,
                format!("resident array '{}' has the wrong declaration name", self.name),
            ));
        }
        if descriptor.meta.version != self.stamp.version {
            return Err(rerr(
                0,
                format!("resident array '{}' declaration version changed", self.name),
            ));
        }
        seal_array_value(reg, &entry.name, descriptor, self.value.clone())?;
        Ok(())
    }
    /// Encode one canonical, schema-stamped resident-array sidecar.
    pub fn encode(&self) -> Vec<u8> {
        let mut encoded = format!(
            "{}\t{:016x}\t{:016x}\t{}\t{}",
            RESIDENT_ARRAY_MAGIC,
            self.stamp.schema,
            self.stamp.declaration.0,
            self.stamp.version,
            self.name
        );
        match &self.value {
            ResidentArrayValue::Numbers(values) => {
                encoded.push_str("\tnumbers");
                for value in values {
                    write!(&mut encoded, "\t{:016x}", value.to_bits()).unwrap();
                }
            }
            ResidentArrayValue::Symbols(values) => {
                encoded.push_str("\tsymbols");
                for value in values {
                    write!(&mut encoded, "\t{}", hex_encode(value.as_bytes())).unwrap();
                }
            }
            ResidentArrayValue::Characters(values) => {
                encoded.push_str("\tcharacters");
                for value in values {
                    write!(&mut encoded, "\t{:08x}", u32::from(*value)).unwrap();
                }
            }
            ResidentArrayValue::Entities(values) => {
                encoded.push_str("\tentities");
                for value in values {
                    write!(&mut encoded, "\t{:016x}", value.to_bits()).unwrap();
                }
            }
            ResidentArrayValue::Discriminants(values) => {
                encoded.push_str("\tdiscriminants");
                for value in values {
                    write!(&mut encoded, "\t{:08x}", value).unwrap();
                }
            }
        }
        encoded.push('\n');
        encoded.into_bytes()
    }

    /// Decode and revalidate one canonical resident-array sidecar.
    pub fn decode(bytes: &[u8], reg: &Registry) -> Result<Self, Diag> {
        let label = "resident array";
        let fields = sidecar_fields(bytes, RESIDENT_ARRAY_MAGIC, label)?;
        let (stamp, name) = sidecar_stamp(&fields, label)?;
        let value = match fields[5] {
            "numbers" => ResidentArrayValue::Numbers(
                fields[6..]
                    .iter()
                    .map(|field| fixed_hex(field, 16, label).map(f64::from_bits))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            "symbols" => ResidentArrayValue::Symbols(
                fields[6..]
                    .iter()
                    .map(|field| {
                        String::from_utf8(hex_decode(field, label)?)
                            .map_err(|_| sidecar_error(label, "symbol payload is not UTF-8"))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            "characters" => ResidentArrayValue::Characters(
                fields[6..]
                    .iter()
                    .map(|field| {
                        let scalar = fixed_hex(field, 8, label)? as u32;
                        char::from_u32(scalar).ok_or_else(|| {
                            sidecar_error(label, format!("invalid character scalar '{field}'"))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            "entities" => ResidentArrayValue::Entities(
                fields[6..]
                    .iter()
                    .map(|field| fixed_hex(field, 16, label).map(f64::from_bits))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            "discriminants" => ResidentArrayValue::Discriminants(
                fields[6..]
                    .iter()
                    .map(|field| fixed_hex(field, 8, label).map(|value| value as u32))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            kind => {
                return Err(sidecar_error(
                    label,
                    format!("unknown resident payload kind '{kind}'"),
                ));
            }
        };
        let handle = Self { stamp, name, value };
        handle.validate(reg)?;
        let sealed = seal_resident_array(reg, &handle.name, handle.value.clone())?;
        if sealed.encode().as_slice() != bytes {
            return Err(sidecar_error(label, "payload is not in canonical form"));
        }
        Ok(sealed)
    }

    /// Validate and atomically save one resident-array sidecar.
    pub fn save(&self, path: &str, reg: &Registry) -> Result<(), Diag> {
        self.validate(reg)?;
        write_sidecar("resident array", path, &self.encode())
    }

    /// Load and revalidate one resident-array sidecar.
    pub fn load(path: &str, reg: &Registry) -> Result<Self, Diag> {
        Self::decode(&read_sidecar("resident array", path)?, reg)
    }
}

fn enum_payload(
    reg: &Registry,
    enumeration: &str,
    input: ConstructorInput,
) -> Result<ConstructedPayload, Diag> {
    let Some(index) = find_ent(reg, enumeration) else {
        return Err(rerr(0, format!("constructor enum '{}' is missing", enumeration)));
    };
    let RegEntryKind::Enum { descriptor } = &reg.ents[index].kind else {
        return Err(rerr(0, format!("'{}' is not an enum", enumeration)));
    };
    let discriminant = match input {
        ConstructorInput::Case(case_name) => descriptor
            .cases
            .iter()
            .find(|case| names_eq(&case.name, &case_name))
            .map(|case| case.discriminant)
            .ok_or_else(|| {
                rerr(0, format!("enum {}: no live case '{}'", enumeration, case_name))
            })?,
        ConstructorInput::Discriminant(discriminant) => {
            live_enum_discriminant(enumeration, descriptor, discriminant)?
        }
        ConstructorInput::Number(value) => {
            return Err(rerr(
                0,
                format!(
                    "enum constructor {} cannot consume number {}",
                    enumeration,
                    num::fmt_g(17, value)
                ),
            ));
        }
    };
    Ok(ConstructedPayload::Enum {
        enumeration: descriptor.meta.id,
        discriminant,
    })
}

/// Apply one checked nominal constructor; no raw carrier value crosses this boundary unchecked.
pub fn construct(
    reg: &Registry,
    name: &str,
    input: ConstructorInput,
) -> Result<ConstructedValue, Diag> {
    validate_registry_contracts(reg)?;
    let Some(index) = reg_find(reg, name) else {
        return Err(rerr(0, format!("no constructor '{}'", name)));
    };
    let RegEntryKind::Ctor { descriptor } = &reg.ents[index].kind else {
        return Err(rerr(0, format!("'{}' is not a constructor", name)));
    };
    let payload = match (&descriptor.refinement, input) {
        (ConstructorRefinement::Range { lo, hi }, ConstructorInput::Number(value))
            if !value.is_nan() && value >= *lo && value <= *hi =>
        {
            ConstructedPayload::Number(if value == 0.0 { 0.0 } else { value })
        }
        (ConstructorRefinement::Range { lo, hi }, ConstructorInput::Number(value)) => {
            return Err(rerr(
                0,
                format!(
                    "constructor {}: value {} is outside {}..{}",
                    reg.ents[index].name,
                    num::fmt_g(17, value),
                    num::fmt_g(17, *lo),
                    num::fmt_g(17, *hi)
                ),
            ));
        }
        (ConstructorRefinement::Range { .. }, _) => {
            return Err(rerr(
                0,
                format!("constructor {} requires a number", reg.ents[index].name),
            ));
        }
        (ConstructorRefinement::Enum { enumeration }, input) => {
            enum_payload(reg, enumeration, input)?
        }
    };
    Ok(ConstructedValue {
        stamp: DeclarationStamp {
            schema: crate::alias::registry_fingerprint(reg),
            declaration: descriptor.meta.id,
            version: descriptor.meta.version,
        },
        name: reg.ents[index].name.clone(),
        payload,
    })
}

impl ConstructedValue {
    /// Return the schema, declaration, and version proof carried by this value.
    pub fn stamp(&self) -> DeclarationStamp {
        self.stamp
    }

    /// Return the canonical constructor name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Borrow the checked nominal payload.
    pub fn payload(&self) -> &ConstructedPayload {
        &self.payload
    }

    pub fn validate(&self, reg: &Registry) -> Result<(), Diag> {
        validate_registry_contracts(reg)?;
        if self.stamp.schema != crate::alias::registry_fingerprint(reg) {
            return Err(rerr(
                0,
                format!("constructed value '{}' belongs to a stale schema", self.name),
            ));
        }
        let Some((entry, descriptor)) = reg.ents.iter().find_map(|entry| match &entry.kind {
            RegEntryKind::Ctor { descriptor }
                if descriptor.meta.id == self.stamp.declaration =>
            {
                Some((entry, descriptor))
            }
            _ => None,
        }) else {
            return Err(rerr(
                0,
                format!("constructed value '{}' has no constructor", self.name),
            ));
        };
        if entry.name != self.name {
            return Err(rerr(
                0,
                format!("constructed value '{}' has the wrong declaration name", self.name),
            ));
        }
        if descriptor.meta.version != self.stamp.version {
            return Err(rerr(
                0,
                format!("constructor '{}' version changed", self.name),
            ));
        }
        match (&descriptor.refinement, &self.payload) {
            (
                ConstructorRefinement::Range { lo, hi },
                ConstructedPayload::Number(value),
            ) if !value.is_nan() && value >= lo && value <= hi => Ok(()),
            (
                ConstructorRefinement::Range { lo, hi },
                ConstructedPayload::Number(value),
            ) => Err(rerr(
                0,
                format!(
                    "constructor {}: value {} is outside {}..{}",
                    self.name,
                    num::fmt_g(17, *value),
                    num::fmt_g(17, *lo),
                    num::fmt_g(17, *hi)
                ),
            )),
            (
                ConstructorRefinement::Enum { enumeration },
                ConstructedPayload::Enum {
                    enumeration: enum_id,
                    discriminant,
                },
            ) => {
                let Some(index) = find_ent(reg, enumeration) else {
                    return Err(rerr(0, format!("constructor enum '{}' is missing", enumeration)));
                };
                let RegEntryKind::Enum { descriptor } = &reg.ents[index].kind else {
                    return Err(rerr(0, format!("'{}' is not an enum", enumeration)));
                };
                if descriptor.meta.id != *enum_id {
                    return Err(rerr(
                        0,
                        format!("constructed value '{}' has the wrong enum identity", self.name),
                    ));
                }
                live_enum_discriminant(enumeration, descriptor, *discriminant)?;
                Ok(())
            }
            _ => Err(rerr(
                0,
                format!("constructed value '{}' has the wrong representation", self.name),
            )),
        }
    }
    /// Encode one canonical, schema-stamped constructed-value sidecar.
    pub fn encode(&self) -> Vec<u8> {
        let mut encoded = format!(
            "{}\t{:016x}\t{:016x}\t{}\t{}",
            CONSTRUCTED_VALUE_MAGIC,
            self.stamp.schema,
            self.stamp.declaration.0,
            self.stamp.version,
            self.name
        );
        match self.payload {
            ConstructedPayload::Number(value) => {
                write!(&mut encoded, "\tnumber\t{:016x}", value.to_bits()).unwrap();
            }
            ConstructedPayload::Enum {
                enumeration,
                discriminant,
            } => {
                write!(
                    &mut encoded,
                    "\tenum\t{:016x}\t{:08x}",
                    enumeration.0, discriminant
                )
                .unwrap();
            }
        }
        encoded.push('\n');
        encoded.into_bytes()
    }

    /// Decode and revalidate one canonical constructed-value sidecar.
    pub fn decode(bytes: &[u8], reg: &Registry) -> Result<Self, Diag> {
        let label = "constructed value";
        let fields = sidecar_fields(bytes, CONSTRUCTED_VALUE_MAGIC, label)?;
        let (stamp, name) = sidecar_stamp(&fields, label)?;
        let payload = match fields[5] {
            "number" if fields.len() == 7 => {
                ConstructedPayload::Number(f64::from_bits(fixed_hex(fields[6], 16, label)?))
            }
            "enum" if fields.len() == 8 => ConstructedPayload::Enum {
                enumeration: DeclId(fixed_hex(fields[6], 16, label)?),
                discriminant: fixed_hex(fields[7], 8, label)? as u32,
            },
            "number" | "enum" => {
                return Err(sidecar_error(label, "wrong payload field count"));
            }
            kind => {
                return Err(sidecar_error(
                    label,
                    format!("unknown constructed payload kind '{kind}'"),
                ));
            }
        };
        let value = Self {
            stamp,
            name,
            payload,
        };
        value.validate(reg)?;
        let sealed = match &value.payload {
            ConstructedPayload::Number(payload) => construct(reg, &value.name, ConstructorInput::Number(*payload))?,
            ConstructedPayload::Enum { discriminant, .. } => construct(reg, &value.name, ConstructorInput::Discriminant(*discriminant))?,
        };
        if sealed.encode().as_slice() != bytes {
            return Err(sidecar_error(label, "payload is not in canonical form"));
        }
        Ok(sealed)
    }

    /// Validate and atomically save one constructed-value sidecar.
    pub fn save(&self, path: &str, reg: &Registry) -> Result<(), Diag> {
        self.validate(reg)?;
        write_sidecar("constructed value", path, &self.encode())
    }

    /// Load and revalidate one constructed-value sidecar.
    pub fn load(path: &str, reg: &Registry) -> Result<Self, Diag> {
        Self::decode(&read_sidecar("constructed value", path)?, reg)
    }
}

fn signature_word(signature: &CallableSignature) -> String {
    let inputs = if signature.inputs.is_empty() {
        "unit".to_string()
    } else {
        signature
            .inputs
            .iter()
            .map(reg_type_word)
            .collect::<Vec<_>>()
            .join(",")
    };
    format!("{}->{}", inputs, reg_type_word(&signature.output))
}

fn effects_word(effects: EffectSet) -> String {
    let mut words = Vec::new();
    if effects.read {
        words.push("read");
    }
    if effects.write {
        words.push("write");
    }
    if effects.service {
        words.push("service");
    }
    if words.is_empty() { "pure".into() } else { words.join(",") }
}

fn determinism_word(determinism: Determinism) -> &'static str {
    match determinism {
        Determinism::Deterministic => "deterministic",
        Determinism::Snapshot => "snapshot",
        Determinism::Nondeterministic => "nondeterministic",
    }
}

fn trust_word(trust: TrustBoundary) -> &'static str {
    match trust {
        TrustBoundary::Checked => "checked",
        TrustBoundary::Trusted => "trusted",
    }
}

fn footprint_word(names: &[String]) -> String {
    if names.is_empty() { "-".into() } else { names.join(",") }
}

fn domain_word(domain: &ArrayDomain) -> String {
    match domain {
        ArrayDomain::Scalar => "scalar".into(),
        ArrayDomain::Entity => "entity".into(),
        ArrayDomain::Fixed(extent) => format!("fixed:{}", extent),
    }
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
    validate_registry_contracts(reg)?;
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
            RegEntryKind::TypedFn { body, descriptor } => {
                let _ = writeln!(
                    b,
                    "fn {} id:{:016x} v:{} sig:{} fx:{} det:{} trust:{} read:{} write:{} use:{} = {}",
                    e.name,
                    descriptor.meta.id.0,
                    descriptor.meta.version,
                    signature_word(&descriptor.signature),
                    effects_word(descriptor.effects),
                    determinism_word(descriptor.determinism),
                    trust_word(descriptor.trust),
                    footprint_word(&descriptor.reads),
                    footprint_word(&descriptor.writes),
                    footprint_word(&descriptor.services),
                    body,
                );
            }
            RegEntryKind::Array { descriptor } => {
                let _ = writeln!(
                    b,
                    "array {} id:{:016x} v:{} {} {}",
                    e.name,
                    descriptor.meta.id.0,
                    descriptor.meta.version,
                    reg_type_word(&descriptor.carrier),
                    domain_word(&descriptor.domain),
                );
            }
            RegEntryKind::Service { descriptor } => {
                let _ = writeln!(
                    b,
                    "service {} id:{:016x} v:{} {} sig:{} trust:{}",
                    e.name,
                    descriptor.meta.id.0,
                    descriptor.meta.version,
                    match descriptor.direction {
                        ServiceDirection::Input => "input",
                        ServiceDirection::Output => "output",
                    },
                    signature_word(&descriptor.signature),
                    trust_word(descriptor.trust),
                );
            }
            RegEntryKind::Enum { descriptor } => {
                let _ = write!(
                    b,
                    "enum {} id:{:016x} v:{}",
                    e.name,
                    descriptor.meta.id.0,
                    descriptor.meta.version,
                );
                for case in &descriptor.cases {
                    let _ = write!(b, " {}={}", case.name, case.discriminant);
                }
                b.push_str(" reserve:");
                if descriptor.reserved.is_empty() {
                    b.push('-');
                } else {
                    for (index, discriminant) in descriptor.reserved.iter().enumerate() {
                        if index > 0 {
                            b.push(',');
                        }
                        let _ = write!(b, "{}", discriminant);
                    }
                }
                b.push('\n');
            }
            RegEntryKind::Ctor { descriptor } => {
                let _ = write!(
                    b,
                    "ctor {} id:{:016x} v:{} ",
                    e.name,
                    descriptor.meta.id.0,
                    descriptor.meta.version,
                );
                match &descriptor.refinement {
                    ConstructorRefinement::Range { lo, hi } => {
                        let _ = writeln!(b, "range:{}..{}", num::dnum(*lo), num::dnum(*hi));
                    }
                    ConstructorRefinement::Enum { enumeration } => {
                        let _ = writeln!(b, "enum:{}", enumeration);
                    }
                }
            }
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
