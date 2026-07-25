//! Relationship endpoint sealing.
//!
//! Exactly `-1` is the functional no-link sentinel and stays diagnostically silent (A17).
//! Every other target must be finite and admitted by the declared endpoint carrier; the
//! carrier alone decides sign (A11), so a negative target through an int-keyed endpoint is
//! ordinary data.  Carrier validity is distinct from foundness: any valid target that does
//! not resolve in the current world is DEAD, never refused.

use crate::num;
use crate::registry::{names_eq, reg_find};
use crate::{ColType, Diag, RegEntryKind, Registry, ANO_NATMAX};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetCarrier {
    RowIndex,
    Key { ty: ColType, range: Option<(f64, f64)> },
}

fn fail(context: &str, value: f64, detail: &str) -> Diag {
    Diag::refuse(format!(
        "{}: relationship target {} {}",
        context,
        num::fmt_g(17, value),
        detail
    ))
}

pub fn is_no_link(value: f64) -> bool {
    value == -1.0
}

pub fn key_carrier(reg: &Registry, key_name: &str) -> Result<TargetCarrier, Diag> {
    let Some(entry) = reg.ents.iter().find(|entry| names_eq(&entry.name, key_name)) else {
        return Err(Diag::refuse(format!(
            "relationship key column '{}' is missing",
            key_name
        )));
    };
    match &entry.kind {
        RegEntryKind::Col { ty, uniq: true, rng, .. } => match ty {
            ColType::Num | ColType::Bool | ColType::Nat | ColType::Int => {
                Ok(TargetCarrier::Key { ty: *ty, range: *rng })
            }
            ColType::Sym | ColType::Char => Err(Diag::refuse(format!(
                "relationship key column '{}' has a nonnumeric carrier",
                entry.name
            ))),
        },
        _ => Err(Diag::refuse(format!(
            "relationship key '{}' is not a unique column",
            key_name
        ))),
    }
}

pub fn carrier_for(reg: &Registry, key_of: Option<&str>) -> Result<TargetCarrier, Diag> {
    match key_of {
        None => Ok(TargetCarrier::RowIndex),
        Some(key) => key_carrier(reg, key),
    }
}

fn type_admits(ty: ColType, value: f64) -> bool {
    match ty {
        ColType::Num => true,
        ColType::Bool => value == 0.0 || value == 1.0,
        ColType::Nat => value >= 0.0 && value <= ANO_NATMAX && value.fract() == 0.0,
        ColType::Int => value >= -ANO_NATMAX && value <= ANO_NATMAX && value.fract() == 0.0,
        ColType::Sym | ColType::Char => false,
    }
}

pub fn validate_target(value: f64, carrier: TargetCarrier, context: &str) -> Result<(), Diag> {
    if is_no_link(value) {
        return Ok(());
    }
    if !value.is_finite() {
        return Err(fail(context, value, "is not finite"));
    }
    match carrier {
        TargetCarrier::RowIndex => {
            if value.fract() != 0.0 || value.abs() > ANO_NATMAX {
                return Err(fail(context, value, "is not an exact row index"));
            }
        }
        TargetCarrier::Key { ty, range } => {
            if !type_admits(ty, value) {
                return Err(fail(context, value, "is outside the declared key carrier"));
            }
            if let Some((lower, upper)) = range {
                if value < lower || value > upper {
                    return Err(fail(context, value, "is outside the declared key range"));
                }
            }
        }
    }
    Ok(())
}

pub fn validate_targets(
    values: &[f64],
    carrier: TargetCarrier,
    context: &str,
) -> Result<(), Diag> {
    for (row, &value) in values.iter().enumerate() {
        validate_target(value, carrier, &format!("{} row {}", context, row))?;
    }
    Ok(())
}

/// Set relationships use an empty fiber for no links.  `-1` is never a set member sentinel.
pub fn validate_set_targets(
    values: &[f64],
    carrier: TargetCarrier,
    context: &str,
) -> Result<(), Diag> {
    for (member, &value) in values.iter().enumerate() {
        if is_no_link(value) {
            return Err(fail(
                &format!("{} member {}", context, member),
                value,
                "is reserved for functional no-link; use an empty fiber",
            ));
        }
        validate_target(value, carrier, &format!("{} member {}", context, member))?;
    }
    Ok(())
}

/// Validate every relationship-bearing registry surface before emission or publication.
pub fn validate_registry(reg: &Registry) -> Result<(), Diag> {
    for entry in &reg.ents {
        match &entry.kind {
            RegEntryKind::Rel { targets, key_of } => {
                let carrier = carrier_for(reg, key_of.as_deref())?;
                validate_targets(targets, carrier, &format!("rel {}", entry.name))?;
                validate_target(entry.defval, carrier, &format!("default {}", entry.name))?;
            }
            RegEntryKind::SRel { fib, inv_of, key_of } => {
                if inv_of.is_none() {
                    let carrier = carrier_for(reg, key_of.as_deref())?;
                    for (row, fiber) in fib.iter().enumerate() {
                        validate_set_targets(
                            fiber,
                            carrier,
                            &format!("srel {} row {}", entry.name, row),
                        )?;
                    }
                }
            }
            RegEntryKind::Proto { fields } => {
                for field in fields {
                    let Some(index) = reg_find(reg, &field.col) else {
                        continue;
                    };
                    if let RegEntryKind::Rel { key_of, .. } = &reg.ents[index].kind {
                        // a sym-shaped fill has no numeric target; it must not validate as row 0
                        if num::wnum(&field.spelling).is_none() {
                            return Err(Diag::refuse(format!(
                                "def {} field {}: relationship target '{}' is not a number",
                                entry.name, field.col, field.spelling
                            )));
                        }
                        let carrier = carrier_for(reg, key_of.as_deref())?;
                        validate_target(
                            field.num,
                            carrier,
                            &format!("def {} field {}", entry.name, field.col),
                        )?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn bqn_number(value: f64) -> String {
    let mut spelling = num::dnum(value);
    if spelling.starts_with('-') {
        spelling.replace_range(..1, "¯");
    }
    spelling
}

/// Elementwise BQN carrier predicate.  The result admits `-1` only for functional targets.
pub fn bqn_validity(value: &str, carrier: TargetCarrier, functional: bool) -> String {
    // BQN is right-to-left: an unparenthesized compound (`∾x`) would swallow the dyad below it.
    let value = format!("({})", value);
    let value = value.as_str();
    let finite = format!("(({}={})∧(|{})<∞)", value, value, value);
    let typed = match carrier {
        TargetCarrier::RowIndex => {
            format!("({}=⌊{})∧((|{})≤9007199254740992)", value, value, value)
        }
        TargetCarrier::Key { ty, range } => {
            let base = match ty {
                ColType::Num => "1".to_string(),
                ColType::Bool => format!("({}=0)∨({}=1)", value, value),
                ColType::Nat => {
                    format!("(0≤{})∧({}=⌊{})∧({}≤9007199254740992)", value, value, value, value)
                }
                ColType::Int => {
                    format!("({}=⌊{})∧((|{})≤9007199254740992)", value, value, value)
                }
                ColType::Sym | ColType::Char => "0".to_string(),
            };
            if let Some((lower, upper)) = range {
                format!(
                    "({})∧({}≤{})∧({}≤{})",
                    base,
                    bqn_number(lower),
                    value,
                    value,
                    bqn_number(upper)
                )
            } else {
                base
            }
        }
    };
    let valid = format!("({})∧({})", finite, typed);
    if functional {
        format!("(¯1={})∨({})", value, valid)
    } else {
        format!("(¯1≠{})∧({})", value, valid)
    }
}

/// Foundness guard for a functional relationship value.  Valid but absent targets remain
/// visible to the diagnostic path as DEAD rather than becoming malformed or silent.
pub fn bqn_found(value: &str, key: Option<&str>) -> String {
    match key {
        None => format!("((¯1≠{})∧(0≤{})∧({}<anoN))", value, value, value),
        Some(key) => format!(
            "((¯1≠{})∧(0≤{})∧(({}⊐{})<≠{}))",
            value, value, key, value, key
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinel_and_negative_row_targets_validate() {
        assert!(validate_target(-1.0, TargetCarrier::RowIndex, "r").is_ok());
        // a negative non-sentinel is a valid dead link (A17), not malformed carrier data
        assert!(validate_target(-2.0, TargetCarrier::RowIndex, "r").is_ok());
        assert!(validate_target(1.5, TargetCarrier::RowIndex, "r").is_err());
        assert!(validate_target(1.0, TargetCarrier::RowIndex, "r").is_ok());
    }

    #[test]
    fn set_members_do_not_accept_functional_sentinel() {
        assert!(validate_set_targets(&[], TargetCarrier::RowIndex, "s").is_ok());
        assert!(validate_set_targets(&[-1.0], TargetCarrier::RowIndex, "s").is_err());
    }

    #[test]
    fn key_carrier_admits_only_its_own_type() {
        let bool_key = TargetCarrier::Key { ty: ColType::Bool, range: None };
        assert!(validate_target(1.0, bool_key, "k").is_ok());
        assert!(validate_target(2.0, bool_key, "k").is_err());
        assert!(validate_target(-1.0, bool_key, "k").is_ok());
        let nat_key = TargetCarrier::Key { ty: ColType::Nat, range: None };
        assert!(validate_target(7.0, nat_key, "k").is_ok());
        assert!(validate_target(7.5, nat_key, "k").is_err());
        // nat's nonnegativity comes from the carrier, not relationship machinery
        assert!(validate_target(-5.0, nat_key, "k").is_err());
        // an int carrier admits negative keys (A11); deadness is the found guard's business
        let int_key = TargetCarrier::Key { ty: ColType::Int, range: None };
        assert!(validate_target(-5.0, int_key, "k").is_ok());
        let num_key = TargetCarrier::Key { ty: ColType::Num, range: None };
        assert!(validate_target(7.5, num_key, "k").is_ok());
    }

    #[test]
    fn key_carrier_honors_the_declared_range() {
        let ranged = TargetCarrier::Key { ty: ColType::Nat, range: Some((10.0, 20.0)) };
        assert!(validate_target(10.0, ranged, "k").is_ok());
        assert!(validate_target(20.0, ranged, "k").is_ok());
        assert!(validate_target(9.0, ranged, "k").is_err());
        assert!(validate_target(21.0, ranged, "k").is_err());
        // the no-link sentinel is never range-checked
        assert!(validate_target(-1.0, ranged, "k").is_ok());
    }

    #[test]
    fn validity_template_parenthesizes_its_value() {
        let text = bqn_validity("∾anoRelStage0", TargetCarrier::RowIndex, false);
        assert!(text.contains("(∾anoRelStage0)"), "{}", text);
        assert!(!text.contains("∾anoRelStage0="), "{}", text);
    }
}
