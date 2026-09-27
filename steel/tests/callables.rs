mod support;
use support::Fixture;

fn callable(name: &str, id: u32, signature: &str, body: &str) -> String {
    format!("fn {name} id:{id:016x} v:1 sig:{signature} fx:pure det:deterministic trust:trusted read:- write:- use:- = {body}
")
}

#[test]
fn application_forms_compute_their_contract_for_seeded_columns() {
    let mut seed = 0x51ed270bu32;
    for count in 1..9 {
        let values: Vec<i32> = (0..count).map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed % 101) as i32 - 50
        }).collect();
        let row = |xs: Vec<i32>| xs.into_iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ");
        let world = format!("n {count}
col Gold num {}
col All bool {}
{}{}as sumPair add
ja 倍 twice
",
            row(values.clone()), row(vec![1;count]),
            callable("twice", 1, "num->num", "{2×𝕩}"),
            callable("add", 2, "num,num->num", "{𝕨+𝕩}"));
        let fixture = Fixture::new(&world);
        let doubled = row(values.iter().map(|v| 2*v).collect());
        let mut acc = 0;
        let prefixes = row(values.iter().map(|v| { acc += v; acc }).collect());
        for (source, expected) in [
            ("twice(Gold)", doubled.clone()),
            ("(twice(Gold))", doubled.clone()),
            ("add(Gold, Gold)", doubled.clone()),
            ("add/ Gold", acc.to_string()),
            ("fold(sumPair) Gold", acc.to_string()),
            (r"add\ Gold", prefixes.clone()),
            ("scan(sumPair) Gold", prefixes),
            ("--! ja
倍 ( Gold )", doubled.clone()),
        ] {
            fixture.accepts(&format!("-- seed 0x51ed270b rows {count}
--! out {expected}
{source}"));
        }
        fixture.accepts(&format!("All , Gold = twice(Gold)
--! expect Gold = {doubled}"));
    }
}

#[test]
fn bare_callables_never_become_values_masks_or_presence() {
    let fixture = Fixture::new(&format!("n 2
col Gold num 10 20
col All bool 1 1
{}fn show
as Double twice
",
        callable("twice", 1, "num->num", "{2×𝕩}")));
    for source in ["twice", "twice _", "!twice", "#/ twice", "def result = twice
Gold", "All , Gold = twice", "Double", "Double _", "^Double", "^Double _"] {
        let output = fixture.run(source, &["--emit"]);
        assert_eq!(output.status.code(), Some(2), "{source}: {}", String::from_utf8_lossy(&output.stderr));
    }
    for source in ["All , show", "All , twice Gold", "All |> twice", "All |> twice Gold", "(twice Gold)", "(twice Gold Gold)"] {
        fixture.refuses(source, "argument list");
    }
    fixture.refuses("twice Gold", "argument list");
    fixture.refuses("twice()", "expects 1 arguments");
    fixture.refuses("twice/ Gold", "A,A->A");
    fixture.refuses(r"twice\ Gold", "A,A->A");
    fixture.accepts("All , show(Gold)");
}

#[test]
fn data_declarations_and_definitions_cannot_be_called_or_reduced_as_functions() {
    for name in ["Gold", "rank", "abs", "sin"] {
        let fixture = Fixture::new(&format!("n 2
col {name} num 10 20
as Alias {name}
"));
        fixture.accepts(&format!("--! out 10 20
{name}"));
        for source in [format!("{name}({name})"), format!("Alias({name})"), format!("{name}/ {name}"), format!(r"{name}\ {name}")] {
            fixture.refuses(&source, "not callable");
        }
    }
    let fixture = Fixture::new("n 2
col Gold num 10 20
");
    for source in ["def f = Gold
f(Gold)", "def f = Gold
f/ Gold", "def rank = Gold
rank(Gold)"] {
        fixture.refuses(source, "not callable");
    }
    for name in ["rank", "abs", "sin"] {
        fixture.refuses(&format!("{name}(Gold, Gold)"), "expects 1 argument");
    }
    fixture.accepts("--! out 0 1
rank(Gold)");
}

#[test]
fn explicit_effect_calls_and_pipeline_calls_keep_their_stage_meaning() {
    let fixture = Fixture::new("n 3
col Gold num 10 20 30
col All bool 1 1 1
fn pay id:0000000000000001 v:1 sig:num->unit fx:write det:deterministic trust:trusted read:- write:Gold use:- = {s 𝕊 c‿a: c+s×a}
fn keep {⊑𝕩}
");
    fixture.accepts("All , pay(Gold)
--! expect Gold = 20 40 60");
    fixture.accepts("All , pay(Gold) |> pay(Gold)
--! expect Gold = 40 80 120");
    fixture.accepts("--! out 0 1 2
All |> keep()");
    fixture.refuses("pay(Gold) + 1", "effect-only");
    fixture.refuses("All , pay()", "expects 1 arguments");
}

#[test]
fn specialized_lowerings_do_not_discard_callable_identity_or_arguments() {
    let fixture = Fixture::new("n 3
col All bool 1 1 1
col Gold num 10 20 30
col Marked bool 0 0 0
fn less {𝕨<𝕩}
");
    fixture.accepts("[a & b , +Marked | a <- All, b <- All, less(a, b)]
--! expect Marked = 1 1 1");
    for args in ["a", "b, a", "a, a", "0, 1"] {
        fixture.refuses(&format!("[a & b , +Marked | a <- All, b <- All, less({args})]"), "two generator bindings in order");
    }
    fixture.refuses("def less = Gold
cross less All All", "not callable");
    let fixture = Fixture::new(&format!("n 2
col Gold num 10 20
{}",
        callable("neighbor", 1, "num->num", "{2×𝕩}")));
    fixture.accepts("--! out 20 40
neighbor(Gold)");
    fixture.refuses("neighbor(Gold).Gold", "unsupported hop");
    fixture.refuses("neighborly(Gold).Gold", "unsupported hop");
}
