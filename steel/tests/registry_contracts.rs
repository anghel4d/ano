// The high-integrity registry boundary: canonical text, capability sealing, typed planning,
// service-versioned aliases, and schema evolution. Everything exercises public Rust APIs; raw BQN
// is inspected as a lowering artifact and is never treated as a semantic oracle.

use steel::alias::AliasEnvironment;
use steel::emit::emit_with_aliases;
use steel::lex::lex;
use steel::migration::{
    DeclarationId, MigrationExtension, MigrationOutcome, MigrationPlan, SchemaManifest, migrate,
    migrate_with_extension,
};
use steel::parse::parse;
use steel::registry::{
    construct, reg_dump, reg_load, seal_resident_array, validate_registry_contracts,
};
use steel::{
    ConstructedPayload, ConstructorInput, Diag, Directives, Interner, RegEntryKind, Registry,
    ResidentArrayValue,
};

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ano-registry-{}-{}", std::process::id(), tag));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn load(tag: &str, text: &str) -> Registry {
    let path = scratch(tag).join("world.reg");
    std::fs::write(&path, text).unwrap();
    reg_load(path.to_str().unwrap()).unwrap_or_else(|diag| panic!("{}: {}", tag, diag.msg))
}

fn load_error(tag: &str, text: &str) -> String {
    let path = scratch(tag).join("world.reg");
    std::fs::write(&path, text).unwrap();
    reg_load(path.to_str().unwrap()).unwrap_err().msg
}

fn full_registry_text() -> &'static str {
    "n 2\n\
col Health num 10 20\n\
service input.entity id:0000000000000001 v:3 input sig:unit->entity trust:checked\n\
service telemetry.emit id:0000000000000002 v:2 output sig:sym->unit trust:trusted\n\
enum Faction id:0000000000000003 v:1 Bandit=1 Player=2 reserve:7,9\n\
ctor HitPoints id:0000000000000004 v:1 range:0..100\n\
ctor FactionValue id:0000000000000005 v:1 enum:Faction\n\
array Scores id:0000000000000006 v:1 num entity\n\
array Factions id:0000000000000007 v:1 Faction fixed:2\n\
fn choose id:0000000000000008 v:1 sig:num,num->num fx:pure det:deterministic trust:trusted read:- write:- use:- = {𝕨⌈𝕩}\n\
fn both id:0000000000000009 v:1 sig:mask,mask->mask fx:pure det:deterministic trust:trusted read:- write:- use:- = {𝕨∧𝕩}\n\
fn heal id:000000000000000a v:1 sig:num->unit fx:write det:deterministic trust:trusted read:- write:Health use:- = {𝕩}\n\
fn announce id:000000000000000b v:1 sig:sym->unit fx:service det:nondeterministic trust:trusted read:- write:- use:telemetry.emit = {𝕩}\n\
fn tri id:000000000000000c v:1 sig:num,num,num->num fx:pure det:deterministic trust:trusted read:- write:- use:- = {𝕩}\n\
as chooser choose\n\
alias Static 1 0\n"
}

fn full_registry(tag: &str) -> Registry {
    load(tag, full_registry_text())
}

fn plan(source: &str, registry: &Registry) -> Result<String, Diag> {
    let environment = AliasEnvironment::for_registry(registry);
    let mut interner = Interner::new();
    let tokens = lex(source.as_bytes(), false, &mut interner)?;
    let program = parse(&tokens, &mut interner)?;
    let snapshot = environment.snapshot(registry)?;
    emit_with_aliases(
        &program,
        registry,
        &Directives::default(),
        &interner,
        snapshot,
    )
}

fn migration(
    old_text: &str,
    candidate_text: &str,
    map: &str,
    tag: &str,
) -> Result<MigrationOutcome, Diag> {
    let old = load(&format!("{}-old", tag), old_text);
    let candidate = load(&format!("{}-candidate", tag), candidate_text);
    let aliases = AliasEnvironment::for_registry(&old);
    let manifest = SchemaManifest::for_registry(&old);
    let plan = MigrationPlan::parse(map)?;
    migrate(&old, &candidate, &aliases, &manifest, &plan)
}

#[test]
fn canonical_round_trip_and_explicit_manifest_identities() {
    let registry = full_registry("round-trip");
    validate_registry_contracts(&registry).unwrap();
    let fingerprint = steel::alias::registry_fingerprint(&registry);
    let manifest = SchemaManifest::for_registry(&registry);

    assert_eq!(
        manifest.declaration("Faction"),
        Some(DeclarationId(0x0000000000000003))
    );
    assert_eq!(
        manifest.declaration("choose"),
        Some(DeclarationId(0x0000000000000008))
    );

    let path = scratch("round-trip-dump").join("canonical.reg");
    reg_dump(&registry, path.to_str().unwrap()).unwrap();
    let dumped = std::fs::read_to_string(&path).unwrap();
    assert!(dumped.contains(
        "fn choose id:0000000000000008 v:1 sig:num,num->num fx:pure det:deterministic trust:trusted read:- write:- use:- = {𝕨⌈𝕩}\n"
    ));
    assert!(
        dumped.contains("enum Faction id:0000000000000003 v:1 Bandit=1 Player=2 reserve:7,9\n")
    );
    assert!(dumped.contains("array Factions id:0000000000000007 v:1 Faction fixed:2\n"));
    assert!(dumped.find("alias Static").unwrap() < dumped.find("as chooser choose").unwrap());

    let reloaded = reg_load(path.to_str().unwrap()).unwrap();
    assert_eq!(format!("{:?}", reloaded), format!("{:?}", registry));
    assert_eq!(steel::alias::registry_fingerprint(&reloaded), fingerprint);
}
#[test]
fn schema_manifests_have_one_canonical_byte_form() {
    let registry = full_registry("manifest-canonical");
    let manifest = SchemaManifest::for_registry(&registry);
    let directory = scratch("manifest-canonical");
    let path = directory.join("world.reg.schema");
    let canonical = manifest.encode();
    std::fs::write(&path, &canonical).unwrap();
    assert_eq!(SchemaManifest::load(&path, &registry).unwrap(), manifest);

    let mut no_newline = canonical.clone();
    no_newline.pop();
    let canonical_text = String::from_utf8(canonical.clone()).unwrap();
    let fingerprint = format!("{:016x}", manifest.identity().fingerprint);
    let corruptions = [
        ("missing-newline", no_newline, "canonical final newline"),
        (
            "padded-version",
            canonical_text.replacen("\t0\t", "\t00\t", 1).into_bytes(),
            "bad canonical schema version",
        ),
        (
            "uppercase-fingerprint",
            canonical_text
                .replacen(&fingerprint, "ABCDEFABCDEFABCD", 1)
                .into_bytes(),
            "bad canonical schema fingerprint",
        ),
        (
            "zero-id",
            canonical_text
                .replace(
                    "decl\t0000000000000003\tFaction",
                    "decl\t0000000000000000\tFaction",
                )
                .into_bytes(),
            "identity zero is reserved",
        ),
    ];
    for (tag, bytes, expected) in corruptions {
        std::fs::write(&path, bytes).unwrap();
        let message = SchemaManifest::load(&path, &registry).unwrap_err().msg;
        assert!(message.contains(expected), "{}: {}", tag, message);
    }

    std::fs::write(&path, &canonical).unwrap();
    assert_eq!(SchemaManifest::load(&path, &registry).unwrap(), manifest);
}

#[test]
fn resident_arrays_and_checked_constructors_seal_nominal_values() {
    let registry = full_registry("capabilities");

    let scores = seal_resident_array(
        &registry,
        "Scores",
        ResidentArrayValue::Numbers(vec![1.5, 2.5]),
    )
    .unwrap();
    scores.validate(&registry).unwrap();
    assert_eq!(scores.stamp().declaration.0, 0x0000000000000006);

    let zeros = seal_resident_array(
        &registry,
        "Scores",
        ResidentArrayValue::Numbers(vec![-0.0, 0.0]),
    )
    .unwrap();
    let ResidentArrayValue::Numbers(zero_values) = zeros.value() else {
        unreachable!()
    };
    assert_eq!(
        zero_values
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        vec![0, 0]
    );
    let negative_zero_array = String::from_utf8(zeros.encode()).unwrap().replacen(
        "\tnumbers\t0000000000000000",
        "\tnumbers\t8000000000000000",
        1,
    );
    assert!(
        steel::ResidentArrayHandle::decode(negative_zero_array.as_bytes(), &registry)
            .unwrap_err()
            .msg
            .contains("canonical form")
    );

    let factions = seal_resident_array(
        &registry,
        "Factions",
        ResidentArrayValue::Symbols(vec!["Bandit".into(), "Player".into()]),
    )
    .unwrap();
    assert_eq!(
        factions.value(),
        &ResidentArrayValue::Discriminants(vec![1, 2])
    );
    factions.validate(&registry).unwrap();

    let hit_points = construct(&registry, "HitPoints", ConstructorInput::Number(37.0)).unwrap();
    assert_eq!(hit_points.payload(), &ConstructedPayload::Number(37.0));
    hit_points.validate(&registry).unwrap();

    let zero_hit_points =
        construct(&registry, "HitPoints", ConstructorInput::Number(-0.0)).unwrap();
    let ConstructedPayload::Number(zero_value) = zero_hit_points.payload() else {
        unreachable!()
    };
    assert_eq!(zero_value.to_bits(), 0);
    let negative_zero_value = String::from_utf8(zero_hit_points.encode())
        .unwrap()
        .replacen(
            "\tnumber\t0000000000000000",
            "\tnumber\t8000000000000000",
            1,
        );
    assert!(
        steel::ConstructedValue::decode(negative_zero_value.as_bytes(), &registry)
            .unwrap_err()
            .msg
            .contains("canonical form")
    );

    let faction = construct(
        &registry,
        "FactionValue",
        ConstructorInput::Case("Player".into()),
    )
    .unwrap();
    assert_eq!(
        faction.payload(),
        &ConstructedPayload::Enum {
            enumeration: steel::DeclId(0x0000000000000003),
            discriminant: 2,
        }
    );
    faction.validate(&registry).unwrap();
    let sidecars = scratch("capability-sidecars");
    let scores_path = sidecars.join("scores.array");
    scores
        .save(scores_path.to_str().unwrap(), &registry)
        .unwrap();
    let scores_bytes = std::fs::read(&scores_path).unwrap();
    let reloaded_scores =
        steel::ResidentArrayHandle::load(scores_path.to_str().unwrap(), &registry).unwrap();
    assert_eq!(reloaded_scores, scores);
    assert_eq!(reloaded_scores.encode(), scores_bytes);

    let hit_points_path = sidecars.join("hit-points.value");
    hit_points
        .save(hit_points_path.to_str().unwrap(), &registry)
        .unwrap();
    assert_eq!(
        steel::ConstructedValue::load(hit_points_path.to_str().unwrap(), &registry).unwrap(),
        hit_points
    );

    let faction_path = sidecars.join("faction.value");
    faction
        .save(faction_path.to_str().unwrap(), &registry)
        .unwrap();
    let faction_bytes = std::fs::read(&faction_path).unwrap();
    let reloaded_faction =
        steel::ConstructedValue::load(faction_path.to_str().unwrap(), &registry).unwrap();
    assert_eq!(reloaded_faction, faction);
    assert_eq!(reloaded_faction.encode(), faction_bytes);

    let wrong_name = String::from_utf8(scores.encode())
        .unwrap()
        .replace("\tScores\tnumbers", "\tFactions\tnumbers");
    assert!(
        steel::ResidentArrayHandle::decode(wrong_name.as_bytes(), &registry)
            .unwrap_err()
            .msg
            .contains("wrong declaration name")
    );

    let stale_schema = String::from_utf8(scores.encode()).unwrap().replacen(
        &format!("\t{:016x}\t", scores.stamp().schema),
        &format!("\t{:016x}\t", scores.stamp().schema ^ 1),
        1,
    );
    assert!(
        steel::ResidentArrayHandle::decode(stale_schema.as_bytes(), &registry)
            .unwrap_err()
            .msg
            .contains("stale schema")
    );

    let wrong_enum = String::from_utf8(faction.encode())
        .unwrap()
        .replace("\tenum\t0000000000000003\t", "\tenum\t0000000000000004\t");
    assert!(
        steel::ConstructedValue::decode(wrong_enum.as_bytes(), &registry)
            .unwrap_err()
            .msg
            .contains("wrong enum identity")
    );

    let mut no_newline = scores.encode();
    no_newline.pop();
    assert!(
        steel::ResidentArrayHandle::decode(&no_newline, &registry)
            .unwrap_err()
            .msg
            .contains("missing canonical final newline")
    );
    let uppercase_bits = String::from_utf8(scores.encode())
        .unwrap()
        .replace("3ff8000000000000", "3FF8000000000000");
    assert!(
        steel::ResidentArrayHandle::decode(uppercase_bits.as_bytes(), &registry)
            .unwrap_err()
            .msg
            .contains("bad 16-digit hexadecimal value")
    );

    let padded_version =
        String::from_utf8(scores.encode())
            .unwrap()
            .replacen("\t1\tScores", "\t01\tScores", 1);
    assert!(
        steel::ResidentArrayHandle::decode(padded_version.as_bytes(), &registry)
            .unwrap_err()
            .msg
            .contains("noncanonical declaration version")
    );

    assert!(
        seal_resident_array(&registry, "Scores", ResidentArrayValue::Numbers(vec![1.0]))
            .unwrap_err()
            .msg
            .contains("expected 2 values")
    );
    assert!(
        seal_resident_array(
            &registry,
            "Factions",
            ResidentArrayValue::Discriminants(vec![1, 7])
        )
        .unwrap_err()
        .msg
        .contains("discriminant 7 is not a live case")
    );
    assert!(
        construct(&registry, "HitPoints", ConstructorInput::Number(101.0))
            .unwrap_err()
            .msg
            .contains("outside 0..100")
    );
    assert!(
        construct(
            &registry,
            "FactionValue",
            ConstructorInput::Case("Retired".into())
        )
        .unwrap_err()
        .msg
        .contains("no live case 'Retired'")
    );

    let mut moved = registry.clone();
    let index = steel::registry::reg_find(&moved, "Factions").unwrap();
    let RegEntryKind::Array { descriptor } = &mut moved.ents[index].kind else {
        unreachable!()
    };
    descriptor.meta.version += 1;
    assert!(
        factions
            .validate(&moved)
            .unwrap_err()
            .msg
            .contains("stale schema")
    );
}

#[test]
fn resident_symbol_sidecars_are_total_utf8() {
    let registry = load(
        "symbol-sidecar",
        "n 0\narray Text id:0000000000000001 v:1 sym fixed:4\n",
    );
    let payload = ResidentArrayValue::Symbols(vec![
        String::new(),
        "two words".into(),
        String::from("\t\n\0"),
        "雪".into(),
    ]);
    let symbols = seal_resident_array(&registry, "Text", payload.clone()).unwrap();
    assert_eq!(symbols.value(), &payload);

    let encoded = symbols.encode();
    assert_eq!(
        steel::ResidentArrayHandle::decode(&encoded, &registry).unwrap(),
        symbols
    );
    let path = scratch("symbol-sidecar-round-trip").join("text.array");
    symbols.save(path.to_str().unwrap(), &registry).unwrap();
    assert_eq!(
        steel::ResidentArrayHandle::load(path.to_str().unwrap(), &registry).unwrap(),
        symbols
    );
}

#[test]
fn loader_refuses_inert_or_fabricating_metadata() {
    let cases = [
        (
            "duplicate-id",
            "n 0\nservice A id:0000000000000001 v:1 input sig:unit->num trust:checked\nservice B id:0000000000000001 v:1 input sig:unit->num trust:checked\n",
            "already owned by 'A'",
        ),
        (
            "raw-checked",
            "n 0\nfn f id:0000000000000001 v:1 sig:num->num fx:pure det:deterministic trust:checked read:- write:- use:- = {𝕩}\n",
            "raw BQN requires trust:trusted",
        ),
        (
            "raw-trailing",
            "n 0\nfn f id:0000000000000001 v:1 sig:num->num fx:pure det:deterministic trust:trusted read:- write:- use:- = {𝕩} junk\n",
            "one brace-delimited dfn",
        ),
        (
            "effect-disagreement",
            "n 0\ncol X num\nfn f id:0000000000000001 v:1 sig:num->unit fx:pure det:deterministic trust:trusted read:- write:X use:- = {𝕩}\n",
            "fx:write and write footprint disagree",
        ),
        (
            "output-direction",
            "n 0\nservice emit id:0000000000000001 v:1 output sig:sym->sym trust:trusted\n",
            "an output service must return unit",
        ),
        (
            "live-reserved",
            "n 0\nenum Color id:0000000000000001 v:1 Red=1 reserve:1\n",
            "live discriminant 1 is also reserved",
        ),
        (
            "missing-enum",
            "n 0\nctor ColorValue id:0000000000000001 v:1 enum:Color\n",
            "no enum 'Color'",
        ),
        (
            "nominal-output",
            "n 0\nenum Color id:0000000000000001 v:1 Red=1 reserve:-\nfn forge id:0000000000000002 v:1 sig:num->Color fx:pure det:deterministic trust:trusted read:- write:- use:- = {𝕩}\n",
            "nominal outputs require a checked constructor",
        ),
        (
            "high-default",
            "n 0\narray A id:0000000000000001 v:1 num scalar\ndefault A 1\n",
            "default on high-integrity declaration 'A'",
        ),
        (
            "inert-array-read",
            "n 0\narray A id:0000000000000001 v:1 num scalar\nfn f id:0000000000000002 v:1 sig:num->num fx:read det:deterministic trust:trusted read:A write:- use:- = {𝕩}\n",
            "read footprint cannot name unreadable declaration 'A'",
        ),
        (
            "primitive-nominal-name",
            "n 0\nenum num id:0000000000000001 v:1 Value=1 reserve:-\n",
            "nominal declaration 'num' collides with a primitive carrier word",
        ),
    ];
    for (tag, text, expected) in cases {
        let message = load_error(tag, text);
        assert!(message.contains(expected), "{}: {}", tag, message);
    }
}

#[test]
fn programmatic_registries_cross_the_same_contract_gate() {
    let registry = full_registry("programmatic");
    let mut duplicate = registry.clone();
    let score = steel::registry::reg_find(&duplicate, "Scores").unwrap();
    let faction = steel::registry::reg_find(&duplicate, "Factions").unwrap();
    let score_id = duplicate.ents[score].kind.declaration_meta().unwrap().id;
    let RegEntryKind::Array { descriptor } = &mut duplicate.ents[faction].kind else {
        unreachable!()
    };
    descriptor.meta.id = score_id;
    assert!(
        validate_registry_contracts(&duplicate)
            .unwrap_err()
            .msg
            .contains("already owned by 'Scores'")
    );

    let mut noncanonical = registry.clone();
    let constructor = steel::registry::reg_find(&noncanonical, "FactionValue").unwrap();
    let RegEntryKind::Ctor { descriptor } = &mut noncanonical.ents[constructor].kind else {
        unreachable!()
    };
    let steel::ConstructorRefinement::Enum { enumeration } = &mut descriptor.refinement else {
        unreachable!()
    };
    *enumeration = "faction".into();
    assert!(
        validate_registry_contracts(&noncanonical)
            .unwrap_err()
            .msg
            .contains("must use canonical spelling 'Faction'")
    );

    let mut undumpable = registry.clone();
    let score = steel::registry::reg_find(&undumpable, "Scores").unwrap();
    undumpable.ents[score].name = "#Scores".into();
    assert!(
        validate_registry_contracts(&undumpable)
            .unwrap_err()
            .msg
            .contains("not one word")
    );
}

#[test]
fn callable_footprints_are_canonical_sets() {
    let registry = load(
        "footprint-order",
        "n 0\ncol First num\ncol Second num\nfn observe id:0000000000000001 v:1 sig:num->num fx:read det:deterministic trust:trusted read:Second,First write:- use:- = {𝕩}\n",
    );
    let callable = steel::registry::reg_find(&registry, "observe").unwrap();
    let RegEntryKind::TypedFn { descriptor, .. } = &registry.ents[callable].kind else {
        unreachable!()
    };
    assert_eq!(
        descriptor.reads,
        &["First".to_string(), "Second".to_string()]
    );

    let path = scratch("footprint-order-dump").join("canonical.reg");
    reg_dump(&registry, path.to_str().unwrap()).unwrap();
    let dumped = std::fs::read_to_string(&path).unwrap();
    assert!(dumped.contains(
        "fn observe id:0000000000000001 v:1 sig:num->num fx:read det:deterministic trust:trusted read:First,Second write:- use:- = {𝕩}\n"
    ));
    let reloaded = reg_load(path.to_str().unwrap()).unwrap();
    assert_eq!(format!("{:?}", reloaded), format!("{:?}", registry));

    let mut unsorted = registry.clone();
    let callable = steel::registry::reg_find(&unsorted, "observe").unwrap();
    let RegEntryKind::TypedFn { descriptor, .. } = &mut unsorted.ents[callable].kind else {
        unreachable!()
    };
    descriptor.reads.reverse();
    let message = validate_registry_contracts(&unsorted).unwrap_err().msg;
    assert!(
        message.contains("must follow declaration order"),
        "{}",
        message
    );
}

#[test]
fn typed_planning_uses_signatures_effects_and_backend_abi() {
    let registry = full_registry("planning");

    let call = plan("choose(Health, Health)\n", &registry).unwrap();
    assert!(call.contains("Fn_choose"), "{}", call);
    let rank_registry = load(
        "rank-collision",
        "n 2\ncol Health num 1 2\ncol Rank nat 0 0\n",
    );
    let rank = plan("rank(Health)\n", &rank_registry).unwrap();
    assert!(rank.contains("AnoRank"), "{}", rank);
    let rank_arity = plan("rank(Health, Health)\n", &rank_registry)
        .unwrap_err()
        .msg;
    assert!(
        rank_arity.contains("rank expects 1 argument"),
        "{}",
        rank_arity
    );
    let callable_rank_registry = load(
        "callable-rank",
        "n 2\ncol Health num 1 2\nfn rank id:0000000000000011 v:1 sig:num->num fx:pure det:deterministic trust:trusted read:- write:- use:- = {≠⥊𝕩}\n",
    );
    let callable_rank = plan("rank(Health)\n", &callable_rank_registry).unwrap();
    assert!(callable_rank.contains("Fn_rank"), "{}", callable_rank);
    assert!(
        !callable_rank.contains("AnoRank Health"),
        "{}",
        callable_rank
    );

    let fold = plan("fold(choose) Health\n", &registry).unwrap();
    assert!(fold.contains("Fn_choose˜´⌽𝕩"), "{}", fold);
    let exact_reducer_registry = load(
        "exact-reducer",
        "n 2\ncol Count nat 1 2\ncol Scalar num 1 2\nfn greatest id:0000000000000010 v:1 sig:nat,nat->nat fx:pure det:deterministic trust:trusted read:- write:- use:- = {𝕨⌈𝕩}\n",
    );
    let exact_fold = plan("fold(greatest) Count\n", &exact_reducer_registry).unwrap();
    assert!(exact_fold.contains("Fn_greatest"), "{}", exact_fold);
    let wrong_fold = plan("fold(greatest) Scalar\n", &exact_reducer_registry)
        .unwrap_err()
        .msg;
    assert!(
        wrong_fold.contains("operand is num, expected nat"),
        "{}",
        wrong_fold
    );

    let cross = plan("cross both (Health > 0) (Health > 0)\n", &registry).unwrap();
    assert!(cross.contains("Fn_both"), "{}", cross);

    let effect = plan("Health , heal Health\n", &registry).unwrap();
    assert!(effect.contains("Fn_heal"), "{}", effect);
    assert!(effect.contains("health ↩"), "{}", effect);

    let output = plan("Health , announce :Ping\n", &registry).unwrap();
    assert!(output.contains("Fn_announce"), "{}", output);
    assert!(!output.contains("health ↩"), "{}", output);

    let exact = plan("choose(1, 2)\n", &registry).unwrap_err().msg;
    assert!(
        exact.contains("argument 1 is nat, expected num"),
        "{}",
        exact
    );

    let arity = plan("tri(Health, Health, Health)\n", &registry)
        .unwrap_err()
        .msg;
    assert!(arity.contains("has no 3-argument value ABI"), "{}", arity);

    let constructor = plan("HitPoints(Health)\n", &registry).unwrap_err().msg;
    assert!(
        constructor.contains("checked host boundary, not raw BQN"),
        "{}",
        constructor
    );

    let attachment = plan("Scores\n", &registry).unwrap_err().msg;
    assert!(
        attachment.contains("needs a host attachment"),
        "{}",
        attachment
    );

    let pipeline = plan("Health |> choose Health Health\n", &registry)
        .unwrap_err()
        .msg;
    assert!(
        pipeline.contains("needs a declared domain signature"),
        "{}",
        pipeline
    );
}

#[test]
fn declared_input_services_version_dynamic_resolvers() {
    let registry = full_registry("resolver-service");
    let mut aliases = AliasEnvironment::for_registry(&registry);
    aliases
        .install_resolver(
            &registry,
            "focus",
            "input.entity",
            &[("entity".into(), "1".into())],
        )
        .unwrap();
    let description = aliases
        .iter()
        .find(|(name, _)| *name == "focus")
        .unwrap()
        .1
        .describe();
    assert!(description.contains("service 3"), "{}", description);
    assert!(
        description.contains("declaration 0000000000000001"),
        "{}",
        description
    );

    let sidecar = scratch("resolver-service-sidecar").join("world.reg.aliases");
    aliases.save(&sidecar).unwrap();
    let encoded = std::fs::read_to_string(&sidecar).unwrap();
    assert!(encoded.contains("\t0000000000000001@3\t"), "{}", encoded);
    assert_eq!(
        AliasEnvironment::load(&sidecar, &registry).unwrap(),
        aliases
    );

    let identity_erased = scratch("resolver-service-identity-erased").join("world.reg.aliases");
    std::fs::write(&identity_erased, encoded.replace("0000000000000001@3", "3")).unwrap();
    let message = AliasEnvironment::load(&identity_erased, &registry)
        .unwrap_err()
        .msg;
    assert!(message.contains("resolver service changed"), "{}", message);

    let old = load(
        "resolver-migration-old",
        "n 2\nservice input.entity id:0000000000000001 v:3 input sig:unit->entity trust:checked\n",
    );
    let candidate = load(
        "resolver-migration-new",
        "n 2\nservice input.entity id:0000000000000001 v:4 input sig:unit->entity trust:checked\n",
    );
    let mut live_aliases = AliasEnvironment::for_registry(&old);
    live_aliases
        .install_resolver(
            &old,
            "focus",
            "input.entity",
            &[("entity".into(), "1".into())],
        )
        .unwrap();
    let manifest = SchemaManifest::for_registry(&old);
    let map = MigrationPlan::parse("preserve input.entity\n").unwrap();
    let message = migrate(&old, &candidate, &live_aliases, &manifest, &map)
        .unwrap_err()
        .msg;
    assert!(
        message.contains("resolver service or carrier changed"),
        "{}",
        message
    );

    let declared_v1 = load(
        "resolver-downgrade-old",
        "n 2\nservice input.entity id:0000000000000001 v:1 input sig:unit->entity trust:checked\n",
    );
    let legacy = load("resolver-downgrade-new", "n 2\n");
    let mut declared_aliases = AliasEnvironment::for_registry(&declared_v1);
    declared_aliases
        .install_resolver(
            &declared_v1,
            "focus",
            "input.entity",
            &[("entity".into(), "1".into())],
        )
        .unwrap();
    let manifest = SchemaManifest::for_registry(&declared_v1);
    let map = MigrationPlan::parse("drop input.entity\n").unwrap();
    let message = migrate(&declared_v1, &legacy, &declared_aliases, &manifest, &map)
        .unwrap_err()
        .msg;
    assert!(
        message.contains("resolver service or carrier changed"),
        "{}",
        message
    );

    let legacy = load("resolver-upgrade-old", "n 2\n");
    let declared_v1 = load(
        "resolver-upgrade-new",
        "n 2\nservice input.entity id:0000000000000001 v:1 input sig:unit->entity trust:checked\n",
    );
    let mut legacy_aliases = AliasEnvironment::for_registry(&legacy);
    legacy_aliases
        .install_resolver(
            &legacy,
            "focus",
            "input.entity",
            &[("entity".into(), "1".into())],
        )
        .unwrap();
    let manifest = SchemaManifest::for_registry(&legacy);
    let map = MigrationPlan::parse("add input.entity\n").unwrap();
    let message = migrate(&legacy, &declared_v1, &legacy_aliases, &manifest, &map)
        .unwrap_err()
        .msg;
    assert!(
        message.contains("resolver service or carrier changed"),
        "{}",
        message
    );
}

#[test]
fn versions_govern_callable_enum_and_constructor_evolution() {
    let old_fn = "n 0\nfn choose id:0000000000000001 v:1 sig:num,num->num fx:pure det:deterministic trust:trusted read:- write:- use:- = {𝕨⌈𝕩}\n";
    let changed_same = "n 0\nfn choose id:0000000000000001 v:1 sig:num,num->num fx:pure det:deterministic trust:trusted read:- write:- use:- = {𝕨⌊𝕩}\n";
    let changed_bumped = "n 0\nfn choose id:0000000000000001 v:2 sig:num,num->num fx:pure det:deterministic trust:trusted read:- write:- use:- = {𝕨⌊𝕩}\n";
    let message = migration(old_fn, changed_same, "preserve choose\n", "fn-same")
        .unwrap_err()
        .msg;
    assert!(
        message.contains("changes semantics without increasing v:1"),
        "{}",
        message
    );
    migration(old_fn, changed_bumped, "preserve choose\n", "fn-bumped").unwrap();

    let old_enum = "n 0\nenum Color id:0000000000000001 v:1 Red=1 Blue=2 reserve:-\n";
    let removed_unreserved = "n 0\nenum Color id:0000000000000001 v:2 Red=1 reserve:-\n";
    let removed_reserved = "n 0\nenum Color id:0000000000000001 v:2 Red=1 reserve:2\n";
    let message = migration(
        old_enum,
        removed_unreserved,
        "preserve Color\n",
        "enum-unreserved",
    )
    .unwrap_err()
    .msg;
    assert!(
        message.contains("removed enum case 'Blue' must reserve discriminant 2"),
        "{}",
        message
    );
    migration(
        old_enum,
        removed_reserved,
        "preserve Color\n",
        "enum-reserved",
    )
    .unwrap();

    let old_ctor = "n 0\nctor Percent id:0000000000000001 v:1 range:0..100\n";
    let changed_ctor = "n 0\nctor Percent id:0000000000000001 v:1 range:0..200\n";
    let bumped_ctor = "n 0\nctor Percent id:0000000000000001 v:2 range:0..200\n";
    let message = migration(old_ctor, changed_ctor, "preserve Percent\n", "ctor-same")
        .unwrap_err()
        .msg;
    assert!(
        message.contains("changes semantics without increasing v:1"),
        "{}",
        message
    );
    migration(old_ctor, bumped_ctor, "preserve Percent\n", "ctor-bumped").unwrap();
}

#[test]
fn migration_receipts_reseal_runtime_capabilities() {
    let old = load(
        "handle-migration-old",
        "n 2\nctor Percent id:0000000000000001 v:1 range:0..100\narray Samples id:0000000000000002 v:1 Percent fixed:2\n",
    );
    let candidate = load(
        "handle-migration-new",
        "n 2\nctor Percent id:0000000000000001 v:2 range:0..50\narray Samples id:0000000000000002 v:1 Percent fixed:2\n",
    );
    let safe_value = construct(&old, "Percent", ConstructorInput::Number(37.0)).unwrap();
    let retired_value = construct(&old, "Percent", ConstructorInput::Number(80.0)).unwrap();
    let safe_array = seal_resident_array(
        &old,
        "Samples",
        ResidentArrayValue::Numbers(vec![10.0, 40.0]),
    )
    .unwrap();
    let retired_array = seal_resident_array(
        &old,
        "Samples",
        ResidentArrayValue::Numbers(vec![10.0, 80.0]),
    )
    .unwrap();

    let aliases = AliasEnvironment::for_registry(&old);
    let manifest = SchemaManifest::for_registry(&old);
    let map = MigrationPlan::parse("preserve Percent\npreserve Samples\n").unwrap();
    let outcome = migrate(&old, &candidate, &aliases, &manifest, &map).unwrap();
    let registry = outcome.registry();
    let receipt = outcome.receipt();

    let value = receipt
        .revalidate_constructed(&safe_value, &old, registry)
        .unwrap();
    assert_eq!(value.stamp().version, 2);
    value.validate(registry).unwrap();

    let array = receipt
        .revalidate_resident_array(&safe_array, &old, registry)
        .unwrap();
    assert_eq!(array.value(), safe_array.value());
    array.validate(registry).unwrap();

    let value_error = receipt
        .revalidate_constructed(&retired_value, &old, registry)
        .unwrap_err()
        .msg;
    assert!(value_error.contains("outside 0..50"), "{}", value_error);

    let array_error = receipt
        .revalidate_resident_array(&retired_array, &old, registry)
        .unwrap_err()
        .msg;
    assert!(array_error.contains("outside Percent"), "{}", array_error);
}

struct HeaderOnlyExtension;

impl MigrationExtension for HeaderOnlyExtension {
    fn migrate_header(&self, old: &Registry, candidate: &Registry) -> Result<bool, Diag> {
        assert_eq!(old.ents, candidate.ents);
        Ok(true)
    }
}

#[test]
fn migration_extensions_authorize_headers_without_mutable_schema_authority() {
    let old = load(
        "extension-old",
        "n 0\nfn choose id:0000000000000001 v:1 sig:num,num->num fx:pure det:deterministic trust:trusted read:- write:- use:- = {𝕨⌈𝕩}\n",
    );
    let candidate = load(
        "extension-new",
        "n 0\nlattice 1 1\nfn choose id:0000000000000001 v:1 sig:num,num->num fx:pure det:deterministic trust:trusted read:- write:- use:- = {𝕨⌈𝕩}\n",
    );
    let aliases = AliasEnvironment::for_registry(&old);
    let manifest = SchemaManifest::for_registry(&old);
    let map = MigrationPlan::parse("preserve choose\n").unwrap();
    let outcome = migrate_with_extension(
        &old,
        &candidate,
        &aliases,
        &manifest,
        &map,
        &HeaderOnlyExtension,
    )
    .unwrap();
    assert_eq!(outcome.registry().ents, candidate.ents);
    assert_eq!((outcome.registry().lat_w, outcome.registry().lat_h), (1, 1));
}
