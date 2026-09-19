// The registered row-display effect. `fn show` selects the standard implementation;
// a callable with an explicit body continues to use its own implementation.
use crate::{Diag, Interner, Node, NodeKind, RegEntry, RegEntryKind, Registry};

pub(crate) fn registered(entry: &RegEntry) -> bool {
    crate::registry::names_eq(&entry.name, "show")
        && matches!(entry.kind, RegEntryKind::Fn { body: None })
}

fn entity_column(entry: &RegEntry, rows: usize) -> bool {
    match &entry.kind {
        RegEntryKind::Col { .. } => true,
        RegEntryKind::Rel { targets, .. } => targets.len() == rows,
        RegEntryKind::SRel { fib, .. } => fib.len() == rows,
        _ => false,
    }
}

pub(crate) fn columns(reg: &Registry, names: &Interner, args: &[Node], line: i32) -> Result<Vec<usize>, Diag> {
    if args.is_empty() {
        return Ok(reg.ents.iter().enumerate()
            .filter(|(_, entry)| entity_column(entry, reg.n as usize))
            .map(|(i, _)| i).collect());
    }
    args.iter().map(|arg| {
        // Normalization marks a declared set-valued column as a forward relation.
        // Display still accepts its declaration name, never an arbitrary relation expression.
        let arg = match &arg.kind {
            NodeKind::Relation { source, inverse: false } => source.as_ref(),
            _ => arg,
        };
        let NodeKind::Name(name) = arg.kind else {
            return Err(Diag::refuse(format!("emit: line {line}: show arguments must name entity columns")));
        };
        let spelling = names.resolve(name);
        let Some(index) = crate::registry::reg_find(reg, spelling) else {
            return Err(Diag::refuse(format!("emit: line {line}: show: unknown column '{spelling}'")));
        };
        if !entity_column(&reg.ents[index], reg.n as usize) {
            return Err(Diag::refuse(format!("emit: line {line}: show: '{spelling}' is not an entity column")));
        }
        Ok(index)
    }).collect()
}

// Headers, selected column values, carrier kinds (numeric/text/char), presence, row indices.
// Width follows Kore's terminal-cell ranges (kore/src/text.rs::cw). Control characters are
// escaped before output so cell contents cannot create rows, labels, or save/trace records.
pub(crate) const BODY: &str = r#"{
  names‿cols‿kinds‿pres‿ids ← 𝕩
  Clean ← {∾{c←𝕩-@ ⋄ (c<32)∨((c≥127)∧c≤159) ? "\x"∾(⌊c÷16)‿(16|c)⊏"0123456789abcdef" ; ⋈𝕩}¨𝕩}
  Number ← {∾{𝕩='¯' ? "-" ; 𝕩='∞' ? "inf" ; ⋈𝕩}¨•Repr 𝕩}
  Cell ← {kind‿present‿value: {𝕊: present=0 ? "_" ; kind=1 ? Clean value ; kind=2 ? Clean ⋈value ; Clean Number value} 0}
  Width ← {+´{c←𝕩-@ ⋄ lo←⟨4352,8986,11035,11904,12353,13312,19968,40960,44032,63744,65072,65280,65504,127744,131072⟩ ⋄ hi←⟨4447,8987,11036,12350,13311,19903,40959,42191,55203,64255,65103,65376,65510,129791,262141⟩ ⋄ 1+∨´(c≥lo)∧c≤hi}¨𝕩}
  data ← {i←𝕩 ⋄ kind←i⊑kinds ⋄ cells←(i⊑pres) {Cell ⟨kind,𝕨,𝕩⟩}¨ (i⊑cols) ⋄ (<Clean i⊑names)∾cells}¨↕≠names
  data ↩ (<(⟨"row"⟩∾Number¨ids))∾data
  widths ← {⌈´Width¨𝕩}¨data
  rows ← ⍉>data
  {padded←(widths+2) {𝕩∾(𝕨-Width 𝕩)⥊' '}¨𝕩 ⋄ •Out (∾¯1↓padded)∾⊑⌽𝕩 ⋄ 0}˘rows
}"#;
