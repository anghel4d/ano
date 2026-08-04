use steel::alias::AliasEnvironment;
use steel::emit::emit_with_aliases;
use steel::lex::lex;
use steel::parse::parse;
use steel::{ColType, Directives, Interner, RegEntry, RegEntryKind, Registry};

fn registry() -> Registry {
    let col = |name: &str, ty: ColType, nums: Vec<f64>| RegEntry {
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
    };
    Registry {
        n: 2,
        ents: vec![
            col("Plot", ColType::Bool, vec![1.0, 1.0]),
            col("Planted", ColType::Bool, vec![0.0, 0.0]),
            col("Gold", ColType::Num, vec![0.0, 0.0]),
        ],
        ..Registry::default()
    }
}

fn emit(source: &str, ja: bool) -> Result<String, String> {
    let registry = registry();
    let aliases = AliasEnvironment::for_registry(&registry);
    let mut interner = Interner::new();
    let tokens = lex(source.as_bytes(), ja, &mut interner).map_err(|diag| diag.msg)?;
    let program = parse(&tokens, &mut interner).map_err(|diag| diag.msg)?;
    emit_with_aliases(
        &program,
        &registry,
        &Directives::default(),
        &interner,
        aliases.snapshot(&registry).map_err(|diag| diag.msg)?,
    )
    .map_err(|diag| diag.msg)
}

#[test]
fn undef_retracts_exactly_one_named_installation_at_a_barrier() {
    let text = emit(
        "def spread = Plot => +Planted\nPlot , Gold += 1\nundef spread\nPlot , Gold += 1\n",
        false,
    )
    .unwrap();
    assert_eq!(text.matches("planted ↩").count(), 1, "{}", text);
    assert_eq!(text.matches("gold ↩").count(), 2, "{}", text);
}

#[test]
fn undef_refuses_unknown_and_duplicate_rule_names() {
    assert_eq!(
        emit("undef absent\n", false).unwrap_err(),
        "emit: line 1: rule 'absent' is not installed"
    );
    assert_eq!(
        emit(
            "def spread = Plot => +Planted\ndef spread = Plot => +Planted\n",
            false,
        )
        .unwrap_err(),
        "emit: line 2: rule 'spread' is already installed"
    );
}

#[test]
fn japanese_retraction_is_the_same_ast_and_plan() {
    let ascii = emit(
        "def spread = Plot => +Planted\nPlot , Gold += 1\nundef spread\n",
        false,
    )
    .unwrap();
    let japanese = emit(
        "定義 spread = Plot なる Planted 付\nPlot 、 Gold に 1 たす\n解除 spread\n",
        true,
    )
    .unwrap();
    assert_eq!(japanese, ascii);
}
