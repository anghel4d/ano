mod support;
use support::Fixture;

#[test]
fn forward_construction_preserves_order_duplicates_and_nested_shape() {
    let f = Fixture::new("n 2\ncol Gold num 10 20\nfn paircode {(10×⊑𝕩)+1⊑𝕩}\nfn size {≠𝕩}\n");
    f.accepts("[[1..5] -> x |=> x * x]\n--! out 1 4 9 16 25");
    f.accepts("[[3, 1, 3] -> x |=> x + 1]\n--! out 4 2 4");
    f.accepts("[[] -> x |=> x * x]\n--! out");
    f.accepts("[5..1]\n--! out");
    f.accepts("[[[1..3] -> x |=> [x, x * x]] -> pair |=> paircode(pair)]\n--! out 11 24 39");
    f.accepts("[[[1..3] -> x |=> [[1..x] -> y |=> [x, y]]] -> rows |=> size(rows)]\n--! out 1 2 3");
    f.accepts("[[[1..2] -> x |=> [[2..3] -> x |=> x]] -> row |=> paircode(row)]\n--! out 23 23");
    f.accepts("--! ja\n[ [ 1 .. 5 ] -> 値 |=> 値 * 値 ]\n--! out 1 4 9 16 25");
    f.accepts("[Gold -> x |=> x + 1]\n--! out 11 21\n--! expect Gold = 10 20");
}

#[test]
fn lexical_bindings_do_not_escape_or_capture_definitions() {
    let f = Fixture::new("n 1\ncol x num 99\ncol Gold num 10\n");
    f.accepts("[[1..3] -> x |=> x]\nx\n--! out 1 2 3\n--! out 99");
    f.accepts("def saved = x\n[[1..2] -> x |=> saved]\n--! expect Gold = 10");
    f.refuses("[[1..3] -> y |=> y]\ny", "unregistered");
    f.refuses("Gold , Gold = [1, 2]", "temporary collection");
    f.refuses("[a & b , +Gold | a <- Gold, b <- Gold]", "expression");
}

#[test]
fn seeded_ranges_and_maps_agree_with_scalar_arithmetic() {
    let f = Fixture::new("n 0\n");
    let mut seed = 0xc0ffee_u32;
    for length in [0, 1, 2, 5, 17] {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let start = (seed % 41) as i32 - 20;
        let end = start + length - 1;
        let expected = (start..=end)
            .map(|x| (x * x + 3).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        f.accepts(&format!("-- seed 0xc0ffee length {length}\n[[{start}..{end}] -> x |=> x*x+3]\n--! out {expected}"));
    }
}

#[test]
fn constructors_preserve_tuples_and_refuse_retired_or_reverse_spellings() {
    let f =
        Fixture::new("n 2\ncol Gold num 10 20\ncol All bool 1 1\nfn size {≠𝕩}\nfn identity {𝕩}\n");
    f.accepts("[[], [1], [1, 2, 3]]\n--! expect Gold = 10 20");
    f.accepts("[[[], [1], [1, 2, 3]] -> row |=> size(row)]\n--! out 0 1 3");
    f.accepts("[1,]\n--! out 1");
    f.accepts("[[1..3] -> x |=> (x + 2) * 3]\n--! out 9 12 15");
    for source in [
        "[x*x <=| x <- [1..3]]",
        "[x*x <> x <- [1..3]]",
        "[[1..3] -> x => x*x]",
        "[[1..3] -> x, x>1 |=> x]",
    ] {
        let out = f.run(source, &["--emit"]);
        assert_eq!(out.status.code(), Some(2), "{source}");
    }
    f.refuses("(1, 2)", "tuples use brackets");
    f.refuses("Gold , Gold = identity([1, 2])", "temporary collection");
    f.refuses("All , Gold = +\\[1, 2] @ All", "temporary scan");
    f.refuses("lazy([[1..3] -> x |=> x])", "unregistered callable");
}

#[test]
fn registered_pure_calls_keep_their_local_arguments_and_signatures() {
    let f = Fixture::new("n 0\nfn double id:0000000000000001 v:1 sig:num->num fx:pure det:deterministic trust:trusted read:- write:- use:- = {2×𝕩}\nfn sub id:0000000000000002 v:1 sig:num,num->num fx:pure det:deterministic trust:trusted read:- write:- use:- = {𝕨-𝕩}\n");
    f.accepts("[[1..3] -> x |=> double(x)]\n--! out 2 4 6");
    f.accepts("[[1..3] -> x |=> sub(10, x)]\n--! out 9 8 7");
    f.refuses("[[1..3] -> x |=> double(x, x)]", "argument count");
    f.refuses("[[1..3] -> x |=> x()]", "not callable");
}

#[test]
fn invalid_runtime_inputs_do_not_publish_a_world() {
    let f = Fixture::new("n 2\ncol Gold num 10 20\nfn double id:0000000000000001 v:1 sig:num->num fx:pure det:deterministic trust:trusted read:- write:- use:- = {2×𝕩}\n");
    let saved = f.0.join("after.reg");
    for source in [
        "[1.5..3]",
        "[[1..2] -> x & 2 |=> x]",
        "[3 -> x |=> x]",
        "[[1] -> x |=> x / 0 - x / 0]",
        "[[[1, 2]] -> x |=> double(x)]",
    ] {
        let output = f.run(source, &["--run", "--save", saved.to_str().unwrap()]);
        assert!(!output.status.success(), "{source}");
        assert!(
            !saved.exists(),
            "refused construction published a world: {source}"
        );
        // The runner retains failed backend programs for interactive diagnosis; expected
        // refusals in this test own and remove only those temporary diagnostics.
        for line in String::from_utf8_lossy(&output.stderr).lines() {
            if let Some((_, path)) = line.split_once(", kept ") {
                let path = std::path::Path::new(path);
                if path.parent() == Some(std::env::temp_dir().as_path())
                    && path
                        .file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with("steel-"))
                {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }
}

#[test]
fn temporary_values_leave_saved_columns_unchanged_and_observe_later_stages() {
    let f = Fixture::new("n 3\ncol All bool 1 1 1\ncol Gold num 10 20 30\npres Gold 1 0 1\n");
    let saved = f.0.join("after.reg");
    let output = f.run(
        "[Gold -> x |=> x*x]\n--! out 100 900",
        &["--run", "--save", saved.to_str().unwrap()],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let before = steel::registry::reg_load(f.0.join("world.reg").to_str().unwrap()).unwrap();
    let after = steel::registry::reg_load(saved.to_str().unwrap()).unwrap();
    assert_eq!(before.ents, after.ents);
    f.accepts("All , Gold += 1\n[Gold -> x |=> x*x]\n--! out 121 961");
}

#[test]
fn cartesian_qualifiers_preserve_order_multiplicity_and_dependent_sources() {
    let f = Fixture::new("n 0\nfn paircode {(10×⊑𝕩)+1⊑𝕩}\nfn size {≠𝕩}\n");
    f.accepts("[[1..2] -> a & [10..12] -> b |=> 100*a+b]\n--! out 110 111 112 210 211 212");
    f.accepts("[[1..3] -> a & [1..3] -> b & a < b |=> a]\n--! out 1 1 2");
    f.accepts("[[[1..3] -> a & [1..3] -> b & a < b |=> [a,b]] -> pair |=> paircode(pair)]\n--! out 12 13 23");
    f.accepts("[[1..3] -> a & [1..a] -> b |=> 10*a+b]\n--! out 11 21 22 31 32 33");
    f.accepts("[[1..2] -> a & [1..2] -> b & [1..2] -> c |=> 100*a+10*b+c]\n--! out 111 112 121 122 211 212 221 222");
    f.accepts("[[1..3] -> a & a > 9 & [1..a / 0] -> b |=> b]\n--! out");
    f.accepts("[[1..3] -> a & (a == 1 | a == 3) |=> a]\n--! out 1 3");
    f.accepts("[[1..3] -> a & !(a == 2) |=> a]\n--! out 1 3");
    f.accepts("[[1..3] -> a & [] -> b |=> a]\n--! out");
    f.accepts("--! ja\n[ [ 1 .. 3 ] -> a & [ 1 .. 3 ] -> b & a b 未満 |=> a ]\n--! out 1 1 2");
    f.refuses("[[1..2] -> a & [1..3] -> a |=> a]", "duplicate binding");
}

#[test]
fn entity_comprehensions_keep_pairs_but_selection_deduplicates_participants() {
    let f = Fixture::new("n 4\ncol Tower bool 1 1 0 0\ncol Creep bool 0 0 1 1\ncol Gold num 0 0 0 0\ncol Marked bool 0 0 0 0\nfn nearby id:0000000000000001 v:1 sig:entity,entity->mask fx:pure det:deterministic trust:trusted read:- write:- use:- = {(𝕨=0)∨𝕩=3}\nfn size {≠𝕩}\n");
    let pairs = "[entities(Tower) -> t & entities(Creep) -> c & nearby(t,c) |=> [t,c]]";
    f.accepts(&format!("[{pairs} -> pair |=> size(pair)]\n--! out 2 2 2"));
    f.accepts("def matches = [entities(Tower) -> t & entities(Creep) -> c & nearby(t,c) |=> t]\nselection(matches) , Gold += 1\n--! expect Gold = 1 1 0 0");
    f.accepts("selection([entities(Tower) -> t & entities(Creep) -> c & nearby(t,c) |=> c]) , +Marked\n--! expect Marked = 0 0 1 1");
    f.accepts("selection(entities(Tower)) , Gold += 1 |> Gold += 1\n--! expect Gold = 2 2 0 0");
    f.accepts(
        "selection([entities(Tower) -> t & 0 == 1 |=> t]) , Gold += 1\n--! expect Gold = 0 0 0 0",
    );
    f.accepts("Creep , Marked = selection(entities(Creep))\n--! expect Marked = 0 0 1 1");
    f.accepts("selection([]) , Gold += 1\n--! expect Gold = 0 0 0 0");
    for source in [
        "selection([0,1])",
        "selection([[1..2] -> x |=> x])",
        "selection([entities(Tower) -> t |=> [t,t]])",
        "selection([entities(Tower) -> t |=> t+0])",
    ] {
        f.refuses(source, "entity references");
    }
    f.refuses(
        "[[0] -> t & [1] -> c & nearby(t,c) |=> t]",
        "entity provenance",
    );
    f.refuses("[entities(Tower) -> t & t < t |=> t]", "equality only");
    f.refuses("[entities(Tower) -> t & t |=> t]", "not Boolean guards");
    f.refuses("[entities(Tower) -> t & !t |=> t]", "not Boolean guards");
    f.refuses("entities(Tower) , Gold += 1", "explicit selection()");
}

#[test]
fn entity_selection_handles_keys_and_recomputes_after_world_changes() {
    let f = Fixture::new("n 4\ncol All bool 1 1 1 1\ncol Left bool 1 1 0 0\ncol Gold num 0 0 0 0\nunique Id int 90 -4 17 2\nrole id Id\nfn keyMatch id:0000000000000001 v:1 sig:entity->mask fx:pure det:deterministic trust:trusted read:- write:- use:- = {𝕩=¯4}\n");
    f.accepts("selection([entities(All) -> e & keyMatch(e) |=> e]) , Gold += 1\n--! expect Gold = 0 1 0 0");
    f.accepts("def refs = [entities(All) -> e |=> e]\nLeft , ~\nselection(refs) , Gold += 1\n--! expect Gold = 1 1\n--! expect Id = 17 2");
    let f =
        Fixture::new("n 4\ncol All bool 1 1 1 1\ncol Left bool 1 1 0 0\ncol Gold num 0 0 0 0\n");
    f.accepts("Left , ~\nselection([entities(All) -> e |=> e]) , Gold += 1\n--! expect Gold = 1 1");
}

#[test]
fn seeded_cartesian_guards_match_all_comparison_operators() {
    let f = Fixture::new("n 0\n");
    let mut seed = 0xa110ca7eu32;
    for count in 0..6 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let base = (seed % 11) as i32 - 5;
        for op in ["<", "<=", ">", ">=", "==", "!="] {
            let mut expected = Vec::new();
            for a in base..base + count {
                for b in -2..=2 {
                    let keep = match op {
                        "<" => a < b,
                        "<=" => a <= b,
                        ">" => a > b,
                        ">=" => a >= b,
                        "==" => a == b,
                        _ => a != b,
                    };
                    if keep {
                        expected.push((100 * a + b).to_string());
                    }
                }
            }
            f.accepts(&format!("-- seed 0xa110ca7e count {count} op {op}\n[[{base}..{}] -> a & [-2..2] -> b & a {op} b |=> 100*a+b]\n--! out {}", base+count-1, expected.join(" ")));
        }
    }
}

#[test]
fn guards_allow_snapshot_reads_but_never_effect_calls_or_namespace_confusion() {
    let f = Fixture::new("n 2\ncol All bool 1 1\ncol Gold num 0 20\nfn wealthy id:0000000000000001 v:1 sig:entity->mask fx:read det:snapshot trust:trusted read:Gold write:- use:- = {10<𝕩⊑gold}\nfn award Gold {𝕩+1}\nfn show\n");
    f.accepts(
        "selection([entities(All) -> e & wealthy(e) |=> e]) , Gold += 1\n--! expect Gold = 0 21",
    );
    f.accepts("All , Gold = 0\nselection([entities(All) -> e & wealthy(e) |=> e]) , Gold += 1\n--! expect Gold = 0 0");
    f.refuses("[[1] -> x & award(x) |=> x]", "pure value callable");
    f.refuses("[[1] -> x & show() |=> x]", "pure value callable");
    f.refuses(
        "[[1] -> All & entities(All) -> e |=> e]",
        "independent of local bindings",
    );
    for name in ["entities", "selection"] {
        f.refuses(&format!("def {name} = All\n{name}(All)"), "not callable");
        let other = Fixture::new(&format!("n 2\ncol All bool 1 1\ncol {name} num 0 0\n"));
        other.refuses(&format!("{name}(All)"), "not callable");
    }
}
