// Category casts resolve declarations; consumers separately validate domains and signatures.
use crate::{Diag, RefCategory, RegEntryKind, Registry};

pub(crate) fn resolve(reg: &Registry, name: &str, category: RefCategory) -> Result<String, Diag> {
    let label = match category {
        RefCategory::Function => "function",
        RefCategory::Column => "column",
    };
    if let Some(index) = crate::registry::reg_find(reg, name) {
        let entry = &reg.ents[index];
        let valid = match category {
            RefCategory::Function => matches!(
                entry.kind,
                RegEntryKind::Fn { .. } | RegEntryKind::TypedFn { .. }
            ),
            RefCategory::Column => matches!(
                entry.kind,
                RegEntryKind::Col { .. }
                    | RegEntryKind::Field { .. }
                    | RegEntryKind::Array { .. }
                    | RegEntryKind::Rel { .. }
                    | RegEntryKind::SRel { .. }
                    | RegEntryKind::Tag { .. }
                    | RegEntryKind::AliasMask { .. }
            ),
        };
        if !valid {
            return Err(Diag::refuse(format!(
                "'{name}' is not a {label} declaration"
            )));
        }
        return Ok(entry.name.clone());
    }
    if category == RefCategory::Function
        && matches!(
            name,
            "abs"
                | "sin"
                | "rank"
                | "entities"
                | "selection"
                | "+"
                | "-"
                | "*"
                | "/"
                | "&"
                | "|"
                | "#"
                | "min"
                | "max"
                | "avg"
        )
    {
        return Ok(name.to_string());
    }
    Err(Diag::refuse(format!(
        "unknown {label} declaration '{name}'"
    )))
}
