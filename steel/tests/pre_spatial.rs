use steel::alias::{AliasEnvironment, ResolvedAlias};
use steel::reducer::{self, Carrier, Form};
use steel::relationship::{self, TargetCarrier};
use steel::{ColType, RegEntry, RegEntryKind, Registry};

fn registry() -> Registry {
    Registry {
        n: 3,
        ents: vec![RegEntry {
            name: "Value".to_string(),
            defval: 0.0,
            kind: RegEntryKind::Col {
                ty: ColType::Num,
                uniq: false,
                nums: vec![1.0, 2.0, 3.0],
                syms: Vec::new(),
                pres: None,
                rng: None,
            },
        }],
        ..Registry::default()
    }
}

#[test]
fn alias_snapshot_is_frozen_and_absence_only_falls_back() {
    let reg = registry();
    let mut live = AliasEnvironment::for_registry(&reg);
    let before = live.snapshot(&reg).unwrap();
    live.install_binding(&reg, "focus", "Value").unwrap();
    assert!(before.resolve(&reg, "focus").unwrap().is_none());
    assert_eq!(live.snapshot(&reg).unwrap().resolve(&reg, "focus").unwrap(), Some(ResolvedAlias::Entry(0)));
}

#[test]
fn reducer_carriers_are_not_glyph_only() {
    let head = |spelling, carrier| reducer::resolve_head(spelling, Form::Fold, carrier, false).unwrap();
    assert_eq!(head("|", Carrier::Mask).reducer().unwrap().direct_step(), Some("∨"));
    assert_eq!(head("|", Carrier::Number).reducer().unwrap().direct_step(), Some("⌈"));
    assert!(head("&", Carrier::Mask).empty_identity().is_some());
    assert!(head("&", Carrier::Number).empty_identity().is_none());
}

#[test]
fn relationship_sealing_follows_the_carrier() {
    assert!(relationship::validate_target(-1.0, TargetCarrier::RowIndex, "r").is_ok());
    // negative non-sentinels are valid dead links (A17); only carrier-invalid values refuse
    assert!(relationship::validate_target(-2.0, TargetCarrier::RowIndex, "r").is_ok());
    for invalid in [1.5, f64::NAN, f64::INFINITY] {
        assert!(relationship::validate_target(invalid, TargetCarrier::RowIndex, "r").is_err());
    }
}
