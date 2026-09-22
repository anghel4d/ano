// Outer-product values own a rank-2 product domain. These tests exercise the public compiler
// boundary and assert that lowering preserves that rank while the effect border rejects scatter.

use steel::alias::AliasEnvironment;
use steel::emit::emit_with_aliases;
use steel::lex::lex;
use steel::parse::parse;
use steel::{ColType, Diag, Directives, Interner, RegEntry, RegEntryKind, Registry};

fn col(name: &str, ty: ColType, nums: Vec<f64>) -> RegEntry {
    RegEntry {
        name: name.to_string(),
        defval: 0.0,
        kind: RegEntryKind::Col {
            ty,
            uniq: false,
            nums,
            syms: Vec::new(),
            pres: None,
            rng: None,
        },
    }
}

fn registry() -> Registry {
    Registry {
        n: 3,
        ents: vec![
            col("A", ColType::Bool, vec![1.0, 1.0, 0.0]),
            col("B", ColType::Bool, vec![0.0, 1.0, 1.0]),
            col("Out", ColType::Num, vec![0.0, 0.0, 0.0]),
            RegEntry {
                name: "dist".to_string(),
                defval: 0.0,
                kind: RegEntryKind::Fn {
                    body: Some("{𝕨-𝕩}".to_string()),
                },
            },
        ],
        ..Registry::default()
    }
}

fn plan(src: &str) -> Result<String, Diag> {
    let reg = registry();
    let env = AliasEnvironment::for_registry(&reg);
    let mut interner = Interner::new();
    let toks = lex(src.as_bytes(), false, &mut interner)?;
    let program = parse(&toks, &mut interner)?;
    let aliases = env.snapshot(&reg)?;
    emit_with_aliases(
        &program,
        &reg,
        &Directives::default(),
        &interner,
        aliases,
    )
}

fn ok(src: &str) -> String {
    match plan(src) {
        Ok(text) => text,
        Err(diag) => panic!("{}: {}", src.trim_end(), diag.msg),
    }
}

fn err(src: &str) -> String {
    match plan(src) {
        Ok(_) => panic!("{}: expected refusal", src.trim_end()),
        Err(diag) => diag.msg,
    }
}

#[test]
fn cross_preserves_the_product_rank() {
    let emitted = ok("cross dist A B\n");
    assert!(emitted.contains('⌜'), "{emitted}");
    assert!(!emitted.contains("⥊(/"), "{emitted}");
}

#[test]
fn product_lineage_cannot_scatter_into_the_world() {
    assert_eq!(
        err("A, Out = cross dist A B\n"),
        "emit: line 1: cannot scatter product value to entity column 'Out'"
    );
}
