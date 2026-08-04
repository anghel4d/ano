// Emitted-plan integration tests for todo/01 sigil semantics: `name` versus `^name`.
// Everything runs in process through the public boundary (lex -> parse -> emit_with_aliases)
// with an in-memory AliasEnvironment; no sidecar files, no demo fixtures, no .reg files.

use steel::alias::AliasEnvironment;
use steel::emit::emit_with_aliases;
use steel::lex::lex;
use steel::parse::parse;
use steel::{
    AliasRow, BindKind, ColType, Diag, Directives, Interner, RegEntry, RegEntryKind, Registry,
};

// The shared world: three rows, one column of each mask-interpretation kind (boolean, sparse,
// total), three bindings (two entity, one point), one STATIC alias-mask fixture, one
// spelling-alias row.
fn registry() -> Registry {
    Registry {
        n: 3,
        ents: vec![
            RegEntry {
                name: "Flag".to_string(),
                defval: 0.0,
                kind: RegEntryKind::Col {
                    ty: ColType::Bool,
                    uniq: false,
                    nums: vec![1.0, 0.0, 1.0],
                    syms: Vec::new(),
                    pres: None,
                    rng: None,
                },
            },
            RegEntry {
                name: "Sca".to_string(),
                defval: 0.0,
                kind: RegEntryKind::Col {
                    ty: ColType::Num,
                    uniq: false,
                    nums: vec![1.0, 2.0, 3.0],
                    syms: Vec::new(),
                    pres: Some(vec![1.0, 0.0, 1.0]),
                    rng: None,
                },
            },
            RegEntry {
                name: "Gold".to_string(),
                defval: 0.0,
                kind: RegEntryKind::Col {
                    ty: ColType::Num,
                    uniq: false,
                    nums: vec![1.0, 2.0, 3.0],
                    syms: Vec::new(),
                    pres: None,
                    rng: None,
                },
            },
            RegEntry {
                name: "Silver".to_string(),
                defval: 0.0,
                kind: RegEntryKind::Col {
                    ty: ColType::Num,
                    uniq: false,
                    nums: vec![4.0, 5.0, 6.0],
                    syms: Vec::new(),
                    pres: None,
                    rng: None,
                },
            },
            RegEntry {
                name: "anchor".to_string(),
                defval: 0.0,
                kind: RegEntryKind::Bind {
                    kind: BindKind::Entity,
                    vals: vec![1.0],
                },
            },
            RegEntry {
                name: "beacon".to_string(),
                defval: 0.0,
                kind: RegEntryKind::Bind {
                    kind: BindKind::Entity,
                    vals: vec![2.0],
                },
            },
            RegEntry {
                name: "spot".to_string(),
                defval: 0.0,
                kind: RegEntryKind::Bind {
                    kind: BindKind::Point,
                    vals: vec![1.0, 2.0],
                },
            },
            RegEntry {
                name: "cursor".to_string(),
                defval: 0.0,
                kind: RegEntryKind::AliasMask {
                    mask: vec![1.0, 0.0, 1.0],
                },
            },
        ],
        aliases: vec![AliasRow {
            from: "X".to_string(),
            to: "Gold".to_string(),
            ja: false,
        }],
        ..Registry::default()
    }
}

// Inputs: source, a registry, a live overlay, the emission directives. Output: the emitted BQN,
// or the refusal. One emission per call: a fresh interner and a snapshot frozen at the boundary.
fn plan_with(
    src: &str,
    reg: &Registry,
    env: &AliasEnvironment,
    dirs: &Directives,
) -> Result<String, Diag> {
    let mut it = Interner::new();
    let toks = lex(src.as_bytes(), false, &mut it)?;
    let prog = parse(&toks, &mut it)?;
    let snap = env.snapshot(reg)?;
    emit_with_aliases(&prog, reg, dirs, &it, snap)
}

fn plan(src: &str, reg: &Registry, env: &AliasEnvironment) -> Result<String, Diag> {
    plan_with(src, reg, env, &Directives::default())
}

fn ok(src: &str, reg: &Registry, env: &AliasEnvironment) -> String {
    match plan(src, reg, env) {
        Ok(text) => text,
        Err(d) => panic!("{}: {}", src, d.msg),
    }
}

fn err(src: &str, reg: &Registry, env: &AliasEnvironment) -> String {
    match plan(src, reg, env) {
        Ok(_) => panic!("{}: expected a refusal", src),
        Err(d) => d.msg,
    }
}

// Gate 4: with an empty overlay `^name` is the bare lookup, so the backend text is identical.
#[test]
fn fallback_plans_are_byte_identical() {
    let reg = registry();
    let env = AliasEnvironment::for_registry(&reg);
    let pairs = [
        // mask position, one pair per mask interpretation: boolean, sparse, total
        ("Flag , Silver = 0", "^Flag , Silver = 0"),
        ("!Flag , Silver = 0", "!^Flag , Silver = 0"),
        ("Sca , Silver = 0", "^Sca , Silver = 0"),
        ("!Sca , Silver = 0", "!^Sca , Silver = 0"),
        ("Gold , Silver = 0", "^Gold , Silver = 0"),
        ("!Gold , Silver = 0", "!^Gold , Silver = 0"),
        // value position
        ("Silver > Gold", "Silver > ^Gold"),
        // fold operand
        ("+/ Gold", "+/ ^Gold"),
        // mirror-read hop base
        ("anchor.Gold", "^anchor.Gold"),
    ];
    for (bare, sigil) in pairs {
        assert_eq!(
            ok(bare, &reg, &env),
            ok(sigil, &reg, &env),
            "{} vs {}",
            bare,
            sigil
        );
    }
}

// Gate 2: the mask-context interpretation table is unchanged, and `^name` inherits it.
#[test]
fn mask_interpretation_pins() {
    let reg = registry();
    let env = AliasEnvironment::for_registry(&reg);

    // the selection mask of statement 1 — pinned as the whole binding, not a fixture substring
    let sel = |src: &str| {
        let text = ok(src, &reg, &env);
        assert!(text.contains("\ns1m ← "), "{}", text);
        text
    };

    // boolean column: the values are the mask; no presence, no all-present fill
    let boolean = sel("Flag , Silver = 0");
    assert!(boolean.contains("\ns1m ← flag\n"), "{}", boolean);

    // sparse value column: presence is the mask
    let sparse = sel("Sca , Silver = 0");
    assert!(sparse.contains("\ns1m ← pres_Sca\n"), "{}", sparse);

    // ! on a sparse value column: the absent-component fast path
    let negated = sel("!Sca , Silver = 0");
    assert!(negated.contains("\ns1m ← (¬pres_Sca)\n"), "{}", negated);

    // total value column: all present
    let total = sel("Gold , Silver = 0");
    assert!(total.contains("\ns1m ← (1¨gold)\n"), "{}", total);

    // and the sigiled spellings read the same table
    assert!(sel("^Flag , Silver = 0").contains("\ns1m ← flag\n"));
    assert!(sel("^Sca , Silver = 0").contains("\ns1m ← pres_Sca\n"));
    assert!(sel("!^Sca , Silver = 0").contains("\ns1m ← (¬pres_Sca)\n"));
    assert!(sel("^Gold , Silver = 0").contains("\ns1m ← (1¨gold)\n"));
}

// An unresolved `^name` says which resolver request the source made.
#[test]
fn unresolved_alias_diagnostic_keeps_the_sigil() {
    let reg = registry();
    let env = AliasEnvironment::for_registry(&reg);

    assert!(
        err("^Nope , Silver = 0", &reg, &env).contains("unregistered mask name '^Nope'"),
        "{}",
        err("^Nope , Silver = 0", &reg, &env)
    );
    assert!(
        err("Silver > ^Nope", &reg, &env).contains("unregistered name '^Nope'"),
        "{}",
        err("Silver > ^Nope", &reg, &env)
    );

    let bare_mask = err("Nope , Silver = 0", &reg, &env);
    assert!(bare_mask.contains("'Nope'"), "{}", bare_mask);
    assert!(!bare_mask.contains("^Nope"), "{}", bare_mask);
    let bare_val = err("Silver > Nope", &reg, &env);
    assert!(bare_val.contains("'Nope'"), "{}", bare_val);
    assert!(!bare_val.contains("^Nope"), "{}", bare_val);
}

// A4: the overlay is consulted only for `^name`; the bare spelling never sees it.
#[test]
fn overlay_moves_only_the_sigiled_spelling() {
    let reg = registry();
    let empty = AliasEnvironment::for_registry(&reg);
    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_binding(&reg, "focus", "Silver").unwrap();

    assert_eq!(
        ok("Gold > ^focus", &reg, &env),
        ok("Gold > Silver", &reg, &empty)
    );
    assert!(err("Gold > focus", &reg, &env).contains("unregistered name 'focus'"));
}

// The dynamic-alias name fold is the registry fold: ASCII A-Z only.
#[test]
fn dynamic_alias_case_folds_ascii() {
    let reg = registry();
    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_binding(&reg, "focus", "Silver").unwrap();
    assert_eq!(
        ok("Gold > ^FOCUS", &reg, &env),
        ok("Gold > ^focus", &reg, &env)
    );
}

// Gate 3: a static alias-mask fixture and a dynamic overlay mask of the same name stay split.
#[test]
fn static_alias_mask_fixture_stays_static() {
    let reg = registry();
    let empty = AliasEnvironment::for_registry(&reg);
    let base = ok("cursor , Silver = 0", &reg, &empty);
    // fallback reaches the static fixture
    assert_eq!(ok("^cursor , Silver = 0", &reg, &empty), base);

    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_mask(&reg, "cursor", &[0.0, 1.0, 0.0]).unwrap();
    assert_eq!(ok("cursor , Silver = 0", &reg, &env), base);
    assert_ne!(ok("^cursor , Silver = 0", &reg, &env), base);
}

// The cold ゼロが default is the exact `^cursor` resolver request. It follows overlay-first
// lookup with bare fallback; once an antecedent exists, later elided effects reuse its saved mask
// and do not consult a changed cursor merely because the source omitted the subject again.
#[test]
fn cold_elided_subject_uses_dynamic_cursor_then_bare() {
    let reg = registry();
    let empty = AliasEnvironment::for_registry(&reg);
    assert_eq!(
        ok("+Flag", &reg, &empty),
        ok("^cursor , +Flag", &reg, &empty)
    );

    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_mask(&reg, "cursor", &[0.0, 1.0, 0.0]).unwrap();
    assert_eq!(ok("+Flag", &reg, &env), ok("^cursor , +Flag", &reg, &env));
    assert_ne!(ok("+Flag", &reg, &env), ok("cursor , +Flag", &reg, &env));

    // Only the first statement is cold. The second inherits the first statement's mask.
    assert_eq!(
        ok("Gold > 1 , Silver = 0\n+Flag", &reg, &env),
        ok("Gold > 1 , Silver = 0\n, +Flag", &reg, &env)
    );
}

#[test]
fn cold_continuations_still_require_an_antecedent() {
    let reg = registry();
    let env = AliasEnvironment::for_registry(&reg);
    assert!(err(", +Flag", &reg, &env).contains("continuation with no antecedent"));
    assert!(err("~", &reg, &env).contains("continuation with no antecedent"));
}

// The `^X` fallback routes through bare lookup, which applies the spelling-alias table.
// Pins today's behavior; the overlay/spelling-table asymmetry is not decided here.
#[test]
fn spelling_alias_control() {
    let reg = registry();
    let env = AliasEnvironment::for_registry(&reg);
    assert_eq!(
        ok("X , Silver = 0", &reg, &env),
        ok("^X , Silver = 0", &reg, &env)
    );
}


#[test]
fn entity_bind_mirror_reads_resolve_stable_keys() {
    let mut reg = registry();
    reg.ents.push(RegEntry {
        name: "Id".to_string(),
        defval: 0.0,
        kind: RegEntryKind::Col {
            ty: ColType::Num,
            uniq: true,
            nums: vec![10.0, 20.0, 30.0],
            syms: Vec::new(),
            pres: None,
            rng: None,
        },
    });
    reg.roles.push(("id".to_string(), "Id".to_string()));
    let anchor = reg
        .ents
        .iter_mut()
        .find(|entry| entry.name == "anchor")
        .expect("anchor");
    let RegEntryKind::Bind { vals, .. } = &mut anchor.kind else {
        panic!("anchor binding");
    };
    vals[0] = 20.0;

    let env = AliasEnvironment::for_registry(&reg);
    let text = ok("anchor.Gold", &reg, &env);
    assert!(text.contains("((⊑(id⊐20))⊑gold)"), "{}", text);
    assert!(err("cursor.Gold", &reg, &env)
        .contains("selection 'cursor' is not a unique entity mirror-read"));
}

// The synthetic materialization names are reserved for the emission that generates them.
#[test]
fn reserved_synthetic_name_refuses_end_to_end() {
    let reg = registry();
    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_mask(&reg, "hot", &[1.0, 0.0, 1.0]).unwrap();
    assert!(
        err("AnoDynMask0 & ^hot , Silver = 0", &reg, &env)
            .contains("reserved synthetic name 'AnoDynMask0'"),
        "{}",
        err("AnoDynMask0 & ^hot , Silver = 0", &reg, &env)
    );

    let empty = AliasEnvironment::for_registry(&reg);
    assert!(
        err("AnoDynMask0 , Silver = 0", &reg, &empty)
            .contains("unregistered mask name 'AnoDynMask0'"),
        "{}",
        err("AnoDynMask0 , Silver = 0", &reg, &empty)
    );
}

// An operand that cannot denote a mask refuses, identically under either spelling.
#[test]
fn non_mask_operand_refuses_both_spellings() {
    let reg = registry();
    let env = AliasEnvironment::for_registry(&reg);
    for src in ["!spot , Silver = 0", "!^spot , Silver = 0"] {
        assert!(
            err(src, &reg, &env).contains("binding 'spot' (point) as mask"),
            "{}",
            src
        );
    }
}

// An overlay entry spelled like a registry column: the two spellings denote different things in
// one world, and the sigiled one denotes exactly what the bare target denotes.  Equality is the
// correct assertion — `fallback_plans_are_byte_identical` already proves Alias-node lowering is
// Name-node lowering for one symbol, so a moved lookup can only differ in which symbol it names.
#[test]
fn shadowed_and_bare_diverge_in_one_world() {
    let reg = registry();
    let empty = AliasEnvironment::for_registry(&reg);
    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_binding(&reg, "gold", "Silver").unwrap();

    let sigiled = ok("^Gold , Silver = 0", &reg, &env);
    assert_eq!(sigiled, ok("Silver , Silver = 0", &reg, &empty));
    assert_ne!(sigiled, ok("Gold , Silver = 0", &reg, &empty));
    // the bare half of the same world is untouched by the overlay
    assert_eq!(
        ok("Gold , Silver = 0", &reg, &env),
        ok("Gold , Silver = 0", &reg, &empty)
    );

    // both halves in ONE emission: the bare operand stays Gold, the sigiled one is Silver
    let both = ok("Gold > ^Gold , Silver = 0", &reg, &env);
    assert_eq!(both, ok("Gold > Silver , Silver = 0", &reg, &empty));
    assert_ne!(both, ok("Gold > Gold , Silver = 0", &reg, &empty));
}

// The stem of a Bind entry is shadowable too: `^anchor` reaches the overlay target while the
// bare hop base still reads the registry binding it always read.
#[test]
fn bind_shadowing() {
    let reg = registry();
    let empty = AliasEnvironment::for_registry(&reg);
    let bare_before = ok("anchor.Gold", &reg, &empty);

    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_binding(&reg, "anchor", "beacon").unwrap();
    assert_eq!(
        ok("^anchor.Gold", &reg, &env),
        ok("beacon.Gold", &reg, &empty)
    );
    assert_ne!(ok("^anchor.Gold", &reg, &env), bare_before);
    assert_eq!(ok("anchor.Gold", &reg, &env), bare_before);
}

// Bullet 5: `!` reads the resolved operand, during an overlay entry's life and after its death.
#[test]
fn negation_during_and_after() {
    let reg = registry();
    let empty = AliasEnvironment::for_registry(&reg);

    // (a) a materialized dynamic mask negates as a mask, by the mask-interpretation table
    let mut masked = AliasEnvironment::for_registry(&reg);
    masked.install_mask(&reg, "hot", &[1.0, 0.0, 1.0]).unwrap();
    let plain = ok("^hot , Silver = 0", &reg, &masked);
    assert!(plain.contains("\ns1m ← anoDynMask0\n"), "{}", plain);
    let negated = ok("!^hot , Silver = 0", &reg, &masked);
    assert!(negated.contains("\ns1m ← (¬anoDynMask0)\n"), "{}", negated);

    // (b) a binding alias negates as its target, not as its stem
    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_binding(&reg, "Flag", "Sca").unwrap();
    let during = ok("!^Flag , Silver = 0", &reg, &env);
    assert_eq!(during, ok("!Sca , Silver = 0", &reg, &empty));
    assert_ne!(during, ok("!Flag , Silver = 0", &reg, &empty));

    // (c) after the delete the sigiled spelling is the bare spelling again
    assert!(env.delete("flag"));
    assert_eq!(
        ok("!^Flag , Silver = 0", &reg, &env),
        ok("!Flag , Silver = 0", &reg, &empty)
    );
}

// The dynamic-alias name fold is the registry fold: non-ASCII bytes are exact, so a stem outside
// A-Z round-trips end to end and a Greek case pair is two distinct stems.
// Behavior-pin for the ASCII-only fold; see docs/ano-keywords.md:589.
#[test]
fn nonascii_alias_end_to_end() {
    let reg = registry();
    let empty = AliasEnvironment::for_registry(&reg);

    let mut wide = AliasEnvironment::for_registry(&reg);
    wide.install_mask(&reg, "世界", &[0.0, 1.0, 0.0]).unwrap();
    let resolved = ok("^世界 , Silver = 0", &reg, &wide);
    assert!(
        resolved.contains("\nanoDynMask0 ← ⟨0, 1, 0⟩\n"),
        "{}",
        resolved
    );
    assert!(resolved.contains("\ns1m ← anoDynMask0\n"), "{}", resolved);
    assert!(err("^世界 , Silver = 0", &reg, &empty).contains("'^世界'"));

    // `Σ` does not fold to `σ`, so the overlay misses and the bare fallback refuses — spelling
    // the sigiled request back, uppercase intact.
    let mut greek = AliasEnvironment::for_registry(&reg);
    greek.install_binding(&reg, "σ", "Gold").unwrap();
    let refusal = err("^Σ , Silver = 0", &reg, &greek);
    assert!(
        refusal.contains("unregistered mask name '^Σ'"),
        "{}",
        refusal
    );
    assert_eq!(
        ok("^σ , Silver = 0", &reg, &greek),
        ok("Gold , Silver = 0", &reg, &empty)
    );
}

// The overlay is keyed by the written stem and only the bare fallback goes on to apply the
// registry spelling table, so the overlay key is read before that table applies.  An entry
// installed under a column's own name and one installed under that column's accepted `as`/`ja`
// spelling are two distinct entries.
#[test]
fn overlay_keys_before_the_spelling_table() {
    let reg = registry();
    let empty = AliasEnvironment::for_registry(&reg);

    let mut under_target = AliasEnvironment::for_registry(&reg);
    under_target
        .install_binding(&reg, "gold", "Silver")
        .unwrap();
    assert_eq!(
        ok("^X , Silver = 0", &reg, &under_target),
        ok("X , Silver = 0", &reg, &empty)
    );

    let mut under_spelling = AliasEnvironment::for_registry(&reg);
    under_spelling.install_binding(&reg, "x", "Silver").unwrap();
    assert_eq!(
        ok("^X , Silver = 0", &reg, &under_spelling),
        ok("Silver , Silver = 0", &reg, &empty)
    );
    assert_ne!(
        ok("^X , Silver = 0", &reg, &under_spelling),
        ok("X , Silver = 0", &reg, &empty)
    );
}

// Bullet 11: the spelling table, a static alias-mask fixture, and the dynamic overlay are three
// mechanisms sharing one word, each keeping its own lifecycle.  All three answer to `X`/`cursor`
// at once; only the sigiled spellings move.
#[test]
fn triple_coexistence() {
    let reg = registry();
    let empty = AliasEnvironment::for_registry(&reg);
    let bare_spelling = ok("X , Silver = 0", &reg, &empty);
    let bare_static = ok("cursor , Silver = 0", &reg, &empty);

    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_binding(&reg, "x", "Silver").unwrap();
    env.install_mask(&reg, "cursor", &[0.0, 1.0, 0.0]).unwrap();

    // unmoved: the spelling table still routes bare X to Gold, the fixture is still the fixture
    assert_eq!(ok("X , Silver = 0", &reg, &env), bare_spelling);
    assert_eq!(ok("cursor , Silver = 0", &reg, &env), bare_static);
    // moved: both sigiled spellings read the overlay
    assert_ne!(ok("^X , Silver = 0", &reg, &env), bare_spelling);
    assert_ne!(ok("^cursor , Silver = 0", &reg, &env), bare_static);
    assert_eq!(
        ok("^X , Silver = 0", &reg, &env),
        ok("Silver , Silver = 0", &reg, &empty)
    );
}

// Completion gate (todo/02:103): a refusal reached through a moved lookup spells the sigiled
// source request AND names the target it reached.  The bare-path pin above stays untouched.
#[test]
fn provenance_diagnostic_spells_request() {
    let reg = registry();
    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_binding(&reg, "focus", "spot").unwrap();
    let refusal = err("!^focus , Silver = 0", &reg, &env);
    assert!(refusal.contains("^focus"), "{}", refusal);
    assert!(refusal.contains("spot"), "{}", refusal);
}

// todo/02:77: with the trace channel enabled each consultation names the mechanism that answered
// it and the overlay version.  The flagless emission of the same program is unchanged — the
// master invariant at steel/src/lib.rs:550, here against the plan the overlay target denotes.
#[test]
fn trace_provenance_line() {
    let reg = registry();
    let empty = AliasEnvironment::for_registry(&reg);
    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_binding(&reg, "focus", "Gold").unwrap();

    let flagless = ok("^focus , Silver = 0", &reg, &env);
    assert_eq!(flagless, ok("Gold , Silver = 0", &reg, &empty));

    let traced = match plan_with(
        "^focus , Silver = 0",
        &reg,
        &env,
        &Directives {
            trace: true,
            ..Directives::default()
        },
    ) {
        Ok(text) => text,
        Err(d) => panic!("{}", d.msg),
    };
    assert!(traced.contains("TRACE-ALIAS ^focus"), "{}", traced);
    assert!(traced.contains("overlay v1"), "{}", traced);
    assert!(traced.contains("binding 'Gold'"), "{}", traced);
    assert_ne!(traced, flagless);
    // and the bare fallback names itself
    let fallback = match plan_with(
        "^Gold , Silver = 0",
        &reg,
        &empty,
        &Directives {
            trace: true,
            ..Directives::default()
        },
    ) {
        Ok(text) => text,
        Err(d) => panic!("{}", d.msg),
    };
    assert!(
        fallback.contains("TRACE-ALIAS ^Gold -> bare fallback"),
        "{}",
        fallback
    );
}

// The ruled `^cursor` cold default (A4, todo/02): an elided statement with no antecedent resolves
// its subject as `^cursor` — overlay first, bare fallback.  The overlay hit
// is byte-identical to spelling `^cursor` explicitly, and an unrelated overlay entry leaves the
// bare-fallback plan byte-identical to the empty-overlay plan.
#[test]
fn cold_default_is_alias_first_with_bare_fallback() {
    let reg = registry();
    let empty = AliasEnvironment::for_registry(&reg);
    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_mask(&reg, "cursor", &[0.0, 1.0, 0.0]).unwrap();

    // elided subject: the overlay hit is exactly the explicit sigiled spelling
    assert_eq!(
        ok("+Flag", &reg, &env),
        ok("^cursor , +Flag", &reg, &env),
        "elided cold default is ^cursor"
    );
    assert_ne!(
        ok("+Flag", &reg, &env),
        ok("+Flag", &reg, &empty),
        "the overlay entry moves the elided subject"
    );

    // bare fallback: no cursor stem in the overlay leaves the pre-overlay plan byte-identical,
    // even while an unrelated alias is installed
    let mut unrelated = AliasEnvironment::for_registry(&reg);
    unrelated.install_binding(&reg, "focus", "Gold").unwrap();
    assert_eq!(ok("+Flag", &reg, &empty), ok("+Flag", &reg, &unrelated));
}

// An antecedent wins over the cold default: only the first statement of a session lacks one, so
// an installed `^cursor` alias must not move a continuation that follows a real selection.
#[test]
fn cold_default_defers_to_the_antecedent() {
    let reg = registry();
    let empty = AliasEnvironment::for_registry(&reg);
    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_mask(&reg, "cursor", &[0.0, 1.0, 0.0]).unwrap();

    let src = "Gold > 1 , Silver = 0\n+Flag\n, Silver = 2\n";
    assert_eq!(
        ok(src, &reg, &env),
        ok(src, &reg, &empty),
        "with an antecedent the overlay entry is not consulted"
    );
}

// With no antecedent, no overlay entry, and no bare cursor entry, the site retains the sigiled
// resolver request in its diagnostic; a static AliasMask fixture remains the bare fallback.
#[test]
fn cold_default_refusal_and_static_fixture_fallback() {
    let reg = registry();
    let empty = AliasEnvironment::for_registry(&reg);
    // the registry's static AliasMask `cursor` answers the bare fallback today, exactly as before
    let fallback = ok("+Flag", &reg, &empty);
    assert!(fallback.contains("\ns1m ← cursor\n"), "{}", fallback);

    let mut bare = registry();
    bare.ents.retain(|e| e.name != "cursor");
    let none = AliasEnvironment::for_registry(&bare);
    assert!(
        err("+Flag", &bare, &none).contains("unregistered mask name '^cursor'"),
        "the refusal preserves the sigiled resolver request"
    );
}

// An overlay mask is not permission to align equal-length buffers: it lowers through the same
// RegEntryKind::AliasMask vehicle as a static registry fixture, so every alignment and lineage
// rule that judges the static mask judges the dynamic one identically.  Pinned by plan identity
// under nothing but the entry-name substitution.
#[test]
fn dynamic_masks_ride_the_static_alias_vehicle() {
    let reg = registry();
    let empty = AliasEnvironment::for_registry(&reg);
    let mut env = AliasEnvironment::for_registry(&reg);
    // the static `cursor` fixture's own values, under a fresh dynamic stem
    env.install_mask(&reg, "dyn", &[1.0, 0.0, 1.0]).unwrap();

    // the fixture prologue legitimately differs by exactly the materialized declaration;
    // everything from the first statement on is identical under the name substitution alone
    let lowered = |text: &str| {
        let at = text
            .find("\n# s1")
            .or_else(|| text.find("\n# q1"))
            .expect("statement or query marker");
        text[at..].to_string()
    };
    for (sigiled, bare) in [
        ("^dyn , Silver = 0", "cursor , Silver = 0"),
        ("!^dyn , Silver = 0", "!cursor , Silver = 0"),
        ("+/ Gold @ ^dyn", "+/ Gold @ cursor"),
    ] {
        let dynamic = ok(sigiled, &reg, &env);
        assert!(
            dynamic.contains("anoDynMask0 ← ⟨1, 0, 1⟩"),
            "{}: the dynamic mask declares its own materialization\n{}",
            sigiled,
            dynamic
        );
        assert_eq!(
            lowered(&dynamic).replace("anoDynMask0", "cursor"),
            lowered(&ok(bare, &reg, &empty)),
            "{} vs {}",
            sigiled,
            bare
        );
    }
}

// Stored selection values are extensional row data: a structural barrier filters them by the
// despawn keep-mask, appends false for every newborn row, and saves that exact post-state.
#[test]
fn stored_masks_track_rows_and_save_exact_post_state() {
    let mut reg = registry();
    reg.ents.push(RegEntry {
        name: "held".to_string(),
        defval: 0.0,
        kind: RegEntryKind::Bind {
            kind: BindKind::Mask,
            vals: vec![0.0, 1.0, 1.0],
        },
    });
    reg.ents.push(RegEntry {
        name: "Seed".to_string(),
        defval: 0.0,
        kind: RegEntryKind::Proto { fields: Vec::new() },
    });
    let env = AliasEnvironment::for_registry(&reg);
    let emitted = plan_with(
        "cursor , ~\nGold , spawn Seed\n",
        &reg,
        &env,
        &Directives {
            save: true,
            ..Directives::default()
        },
    )
    .unwrap();

    for name in ["cursor", "held"] {
        assert!(
            emitted.lines().any(|line|
                line.contains(&format!("{} ↩ (", name))
                    && line.contains(&format!("/{}", name))),
            "missing despawn filter for {}:\n{}",
            name,
            emitted
        );
        assert!(
            emitted
                .lines()
                .any(|line| line.contains(&format!("{} ↩ ", name)) && line.contains("⥊0)")),
            "missing false spawn append for {}:\n{}",
            name,
            emitted
        );
    }
    assert!(emitted.contains("\"alias cursor\""), "{}", emitted);
    assert!(emitted.contains("\"bindmask held\""), "{}", emitted);
}
