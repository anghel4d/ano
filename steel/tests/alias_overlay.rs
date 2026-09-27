// The overlay as an environment rather than as a spelling: snapshot barriers, resolver targets,
// the install/rebind/delete lifecycle across emissions, and the sidecar as it reaches the CLI.
//
// Barrier model under test: one statement evaluates against one coherent world, overlay, and
// host-input snapshot.  Batch Steel freezes one (registry, environment) pair per emission and
// Kore's host boundary is the statement barrier; both are that one law seen from the two hosts
// that run Ano.

use steel::alias::{AliasEnvironment, AliasSnapshot};
use steel::emit::emit_with_aliases;
use steel::lex::lex;
use steel::parse::parse;
use steel::{ColType, Diag, Directives, Interner, RegEntry, RegEntryKind, Registry};

// Three rows, three total value columns: every column can be a selection, a value, or a target.
fn registry() -> Registry {
    let col = |name: &str, nums: Vec<f64>| RegEntry {
        name: name.to_string(),
        defval: 0.0,
        kind: RegEntryKind::Col {
            ty: ColType::Num,
            uniq: false,
            nums,
            syms: Vec::new(),
            pres: None,
            rng: None,
        },
    };
    Registry {
        n: 3,
        ents: vec![
            col("Gold", vec![1.0, 2.0, 3.0]),
            col("Silver", vec![4.0, 5.0, 6.0]),
            col("Bronze", vec![7.0, 8.0, 9.0]),
        ],
        ..Registry::default()
    }
}

// Inputs: source, the registry, an already-frozen snapshot. Output: the emitted BQN or the
// refusal.  Taking the snapshot by value is the point: a caller may retain one across host
// transitions and re-emit under it.
fn plan(src: &str, reg: &Registry, snap: AliasSnapshot) -> Result<String, Diag> {
    let mut it = Interner::new();
    let toks = lex(src.as_bytes(), false, &mut it)?;
    let prog = parse(&toks, &mut it)?;
    emit_with_aliases(&prog, reg, &Directives::default(), &it, snap)
}

fn ok(src: &str, reg: &Registry, snap: AliasSnapshot) -> String {
    match plan(src, reg, snap) {
        Ok(text) => text,
        Err(d) => panic!("{}: {}", src, d.msg),
    }
}

fn live(src: &str, reg: &Registry, env: &AliasEnvironment) -> String {
    ok(src, reg, env.snapshot(reg).expect("snapshot"))
}

fn pairs(members: &[(&str, &str)]) -> Vec<(String, String)> {
    members
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

// A private directory under the system temp root, unique per test and per process.
fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ano-overlay-{}-{}", std::process::id(), tag));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// Bullet 7: one emission, one environment.  A rebind that lands after the snapshot is invisible
// to every gather of that emission — the multi-statement program is byte-identical to the same
// program emitted under an environment that never moved at all.
#[test]
fn multi_gather_one_snapshot() {
    let reg = registry();
    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_binding(&reg, "focus", "Gold").unwrap();
    let frozen = env.snapshot(&reg).expect("snapshot");
    let unchanged = env.clone();

    // the host transition happens AFTER the snapshot was taken
    env.install_binding(&reg, "focus", "Silver").unwrap();
    assert_ne!(env.version(), unchanged.version());

    let src = "^focus , Silver = 0\n^focus > 1 , Bronze = 2\n";
    let under_frozen = ok(src, &reg, frozen.clone());
    assert_eq!(under_frozen, live(src, &reg, &unchanged));
    assert_ne!(under_frozen, live(src, &reg, &env));

    // and the barrier is the emission, not the gather: re-emitting under the retained snapshot
    // still observes the old entry after the transition
    assert_eq!(ok(src, &reg, frozen), under_frozen);
}

// Bullet 8: a resolver target is invoked against the frozen (registry, input) pair recorded on
// it, so both gathers of one emission materialize the same entity row.  Changed host input
// becomes visible at the next barrier, which in batch Steel is the next emission.
#[test]
fn resolver_frozen_snapshot_and_next_barrier() {
    let reg = registry();
    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_resolver(&reg, "focus", "input.entity", &pairs(&[("entity", "1")]))
        .unwrap();
    let frozen = env.snapshot(&reg).expect("snapshot");

    let src = "^focus , Silver = 0\n^focus , Bronze = 9\n";
    let first = ok(src, &reg, frozen.clone());
    assert!(first.contains("\ns1m ← ((↕anoN)=1)\n"), "{}", first);
    assert!(first.contains("\ns2m ← ((↕anoN)=1)\n"), "{}", first);

    // the host rebinds the same stem: the next emission observes the new frozen input
    env.install_resolver(&reg, "focus", "input.entity", &pairs(&[("entity", "2")]))
        .unwrap();
    let second = live(src, &reg, &env);
    assert!(second.contains("\ns1m ← ((↕anoN)=2)\n"), "{}", second);
    assert!(second.contains("\ns2m ← ((↕anoN)=2)\n"), "{}", second);
    assert_ne!(first, second);

    // the retained snapshot is still the old barrier
    assert_eq!(ok(src, &reg, frozen), first);
}

// Bullets 3 and 4 end to end: install, rebind, delete, each observed by one emission of one
// source.  Every state denotes exactly its target; the delete restores the empty-overlay
// fallback; the bare spelling and the registry itself never move.
#[test]
fn lifecycle_across_emissions() {
    let reg = registry();
    // Registry has no PartialEq, so the Debug rendering is the structural identity available to
    // an integration test without touching the library.
    let before = format!("{:?}", reg);
    let empty = AliasEnvironment::for_registry(&reg);

    let sigiled = "^Gold , Silver = 0";
    let bare = "Gold , Silver = 0";
    let bare_plan = live(bare, &reg, &empty);
    let fallback = live(sigiled, &reg, &empty);
    assert_eq!(fallback, bare_plan);

    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_binding(&reg, "gold", "Silver").unwrap();
    assert_eq!(
        live(sigiled, &reg, &env),
        live("Silver , Silver = 0", &reg, &empty)
    );
    assert_eq!(live(bare, &reg, &env), bare_plan);

    env.install_binding(&reg, "gold", "Bronze").unwrap();
    assert_eq!(
        live(sigiled, &reg, &env),
        live("Bronze , Silver = 0", &reg, &empty)
    );
    assert_eq!(live(bare, &reg, &env), bare_plan);

    assert!(env.delete("GOLD"));
    assert_eq!(live(sigiled, &reg, &env), fallback);
    assert_eq!(live(bare, &reg, &env), bare_plan);

    assert_eq!(format!("{:?}", reg), before);
}

// Bullet 10: a sidecar survives a reload with its version intact — the counter is the host's
// barrier stamp, so a reload that reset it would silently rewrite the replay record.
#[test]
fn version_preserved_across_reload() {
    let reg = registry();
    let path = scratch("version").join("world.reg.aliases");
    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_binding(&reg, "focus", "Gold").unwrap();
    env.install_mask(&reg, "hot", &[1.0, 0.0, 1.0]).unwrap();
    env.install_resolver(&reg, "here", "input.entity", &pairs(&[("entity", "2")]))
        .unwrap();
    assert_eq!(env.version(), 3);

    env.save(&path).unwrap();
    let loaded = AliasEnvironment::load(&path, &reg).unwrap();
    assert_eq!(loaded.version(), 3);
    assert_eq!(loaded, env);
}

// Bullet 10, steel half: the whole path the host actually uses — a .reg on disk, its sidecar
// beside it, `--! registry` naming the world, `^name` in the source.  Then the operational
// stale-refusal pin: a structural change to the .reg refuses the emission before any lookup.
#[test]
fn steel_cli_sidecar_end_to_end() {
    let dir = scratch("cli");
    let reg_path = dir.join("world.reg");
    std::fs::write(&reg_path, "n 3\ncol Gold num 1 2 3\ncol Silver num 0 0 0\n").unwrap();
    let reg = steel::registry::reg_load(reg_path.to_str().unwrap()).expect("reg_load");

    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_mask(&reg, "focus", &[0.0, 1.0, 0.0]).unwrap();
    env.save(steel::alias::sidecar_path(reg_path.to_str().unwrap()))
        .unwrap();

    // the expect pin is the observation: silver moves on row 1 only if `^focus` reached the
    // sidecar mask; the trailing query prints the post-state so the observation is also visible
    let ano_path = dir.join("world.ano");
    std::fs::write(
        &ano_path,
        "--! registry world.reg\n--! expect silver = 0 7 0\n\n^focus , Silver = 7\n+/ Silver\n",
    )
    .unwrap();

    let run = || {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_steel"))
            .arg("--run")
            .arg(&ano_path)
            .output()
            .expect("spawn steel");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        (out.status, text)
    };

    let (status, text) = run();
    assert!(status.success(), "{}", text);
    assert!(text.contains('7'), "{}", text);

    // the control: without the sidecar the same source has nothing to resolve
    let sidecar = steel::alias::sidecar_path(reg_path.to_str().unwrap());
    let saved = std::fs::read(&sidecar).unwrap();
    std::fs::remove_file(&sidecar).unwrap();
    let (status, text) = run();
    assert!(!status.success(), "{}", text);
    assert!(text.contains("unregistered mask name '^focus'"), "{}", text);
    std::fs::write(&sidecar, saved).unwrap();

    // a structural change to the world stales the sidecar; the emission refuses as a whole
    std::fs::write(
        &reg_path,
        "n 3\ncol Gold num 1 2 3\ncol Silver num 0 0 0\ncol Bronze num 0 0 0\n",
    )
    .unwrap();
    let (status, text) = run();
    assert!(!status.success(), "{}", text);
    assert!(text.contains("alias"), "{}", text);
    assert!(text.contains("schema"), "{}", text);
}

// Host boundary bullet 2 and the failed-installation test row: every refused transition —
// unknown target, wrong-shape or non-Boolean mask, unknown resolver, malformed resolver input,
// wrong-schema world — leaves the environment untouched: version, schema, and entries alike.
#[test]
fn failed_installations_leave_the_environment_unchanged() {
    let reg = registry();
    let mut env = AliasEnvironment::for_registry(&reg);
    env.install_binding(&reg, "focus", "Gold").unwrap();
    let before = env.clone();

    // unknown binding target
    assert!(env.install_binding(&reg, "focus", "Mithril").is_err());
    // mask with the wrong row count, then a value outside 0/1
    assert!(env.install_mask(&reg, "focus", &[1.0, 0.0]).is_err());
    assert!(env.install_mask(&reg, "focus", &[1.0, 0.5, 0.0]).is_err());
    // unknown resolver id, then a malformed input member
    assert!(
        env.install_resolver(&reg, "focus", "input.rift", &pairs(&[("entity", "1")]))
            .is_err()
    );
    assert!(
        env.install_resolver(&reg, "focus", "input.entity", &pairs(&[("", "1")]))
            .is_err()
    );
    // a structurally different world refuses at the schema gate before any target validation
    let mut moved = registry();
    moved.ents.push(RegEntry {
        name: "Mithril".to_string(),
        defval: 0.0,
        kind: RegEntryKind::Col {
            ty: ColType::Num,
            uniq: false,
            nums: vec![0.0, 0.0, 0.0],
            syms: Vec::new(),
            pres: None,
            rng: None,
        },
    });
    assert!(env.install_binding(&moved, "focus", "Silver").is_err());

    assert_eq!(
        env, before,
        "a failed transition leaves the old environment"
    );
}

// The stale-present-entry half of the same row: an entry the current world can no longer
// validate refuses the lookup path outright — at the snapshot barrier and at sidecar load —
// and never exposes the bare binding underneath it.
#[test]
fn stale_present_entries_refuse_rather_than_fall_through() {
    let reg = registry();
    let mut env = AliasEnvironment::for_registry(&reg);
    // deliberately shadow a live bare column: a silent fallthrough would answer with `Gold`
    env.install_binding(&reg, "gold", "Silver").unwrap();

    let mut moved = registry();
    moved.ents.push(RegEntry {
        name: "Mithril".to_string(),
        defval: 0.0,
        kind: RegEntryKind::Col {
            ty: ColType::Num,
            uniq: false,
            nums: vec![0.0, 0.0, 0.0],
            syms: Vec::new(),
            pres: None,
            rng: None,
        },
    });

    // the statement barrier: no snapshot exists against the moved world, so no gather can run
    let refusal = env.snapshot(&moved).unwrap_err();
    assert!(refusal.msg.contains("stale"), "{}", refusal.msg);

    // and the persisted environment refuses the same way at load
    let path = scratch("stale").join("world.reg.aliases");
    env.save(&path).unwrap();
    let refusal = AliasEnvironment::load(&path, &moved).unwrap_err();
    assert!(refusal.msg.contains("belongs to schema"), "{}", refusal.msg);

    // the unmoved world still resolves the entry: the refusal above was staleness, not damage
    assert!(env.snapshot(&reg).is_ok());
}
