// Language regressions observed through Steel's CLI, actual backend, and refusal boundary.
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const WORLD: &str = "n 4\n\
col Nord bool 1 0 1 0\n\
col Breton bool 0 1 0 1\n\
col Gold num 10 20 30 40\n\
col Silver num 1 2 3 4\n\
col Marked bool 0 0 0 0\n\
rel mentor -1 0 0 2\n\
bind cursor entity 1\n\
fn ping Gold {s 𝕊 cs: (⊑cs)+s}\n\
fn award Gold {s 𝕊 c‿a‿b: c+s×(a+b)}\n";

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "ano-parser-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("world.reg"), WORLD).unwrap();
        Self(path)
    }

    fn run(&self, source: &str, execute: bool) -> Output {
        let path = self.0.join("input.ano");
        std::fs::write(&path, format!("--! registry world.reg\n{source}\n")).unwrap();
        Command::new(env!("CARGO_BIN_EXE_steel"))
            .arg(if execute { "--run" } else { "--emit" })
            .arg(path)
            .output()
            .unwrap()
    }

    fn accepts(&self, source: &str) {
        let output = self.run(source, true);
        assert!(
            output.status.success(),
            "{source}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn refuses(&self, source: &str, cause: &str) {
        let output = self.run(source, false);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{source}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(cause),
            "{source}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn effect_calls_keep_empty_and_multiple_argument_lists() {
    Fixture::new().accepts("Nord , ping()\nNord , award(2, 3)\n--! expect Gold = 16 20 36 40");
}

#[test]
fn negation_and_signed_values_compose_over_masks_and_columns() {
    let fixture = Fixture::new();
    let mut seed = 0x6a09e667u32;
    for case in 0..12 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let negations = (seed % 7) as usize;
        let amount = (seed >> 8) % 31;
        let values: Vec<String> = [10i64, 20, 30, 40]
            .iter()
            .enumerate()
            .map(|(i, old)| {
                if (i % 2 == 0) ^ (negations % 2 == 1) {
                    -(i as i64 + 1 + amount as i64)
                } else {
                    *old
                }
            })
            .map(|v| v.to_string())
            .collect();
        fixture.accepts(&format!(
            "-- seed 0x6a09e667 case {case}\n{}Nord , Gold = -(Silver + {amount})\nNord , mentor = -1\n--! expect Gold = {}\n--! expect mentor = -1 0 -1 2",
            "!".repeat(negations), values.join(" ")
        ));
    }
}

#[test]
fn elided_updates_use_the_cursor_or_saved_antecedent() {
    let fixture = Fixture::new();
    fixture.accepts("Gold += 7\n--! expect Gold = 10 27 30 40");
    fixture.accepts(
        "Nord , Gold += 1\nGold *= 2\nGold -= 2\nGold /= 2\n--! expect Gold = 10 20 30 40",
    );
}

#[test]
fn continuation_whitespace_preserves_statement_barriers() {
    Fixture::new().accepts("Nord ,\n Gold += 1;\n Marked = 1\n(Nord\n & Gold > 20),\n Gold += 10\n--! expect Gold = 11 20 41 40\n--! expect Marked = 1 0 1 0");
}

#[test]
fn comprehension_delimiters_leave_grouped_value_operators_intact() {
    let fixture = Fixture::new();
    fixture.accepts("[a & b , Gold += 1 | a <- Nord, b <- Breton]\n--! expect Gold = 11 21 31 41");
    fixture.accepts("[a & b , Gold = (1 | 2) | a <- Nord, b <- Breton]\n--! expect Gold = 2 2 2 2");
    fixture.accepts(
        "[a & b , award(1 | 2, 3) | a <- Nord, b <- Breton]\n--! expect Gold = 15 25 35 45",
    );
}

#[test]
fn long_count_forms_preserve_fold_and_prefix_results() {
    Fixture::new().accepts("--! out 2\nfold(#) Nord\n--! out 1 1 2 2\nscan(#) Nord\n--! out 2\n#/ Nord\n--! out 1 1 2 2\n#\\ Nord");
}

#[test]
fn only_explicit_comparisons_are_values_inside_effects() {
    let fixture = Fixture::new();
    for source in [
        "Nord , Gold = Silver = 1",
        "Nord , Gold = (Silver = 1)",
        "Nord , award(1, Silver = 1)",
    ] {
        fixture.refuses(source, "use '==' for comparison inside an effect");
    }
    fixture.accepts("Nord , Gold = (Silver == 1) + 0\n--! expect Gold = 1 20 0 40");
    fixture.accepts("Silver = 1 , Gold = 99\n--! expect Gold = 99 20 30 40");
}

#[test]
fn excessive_nesting_refuses_without_aborting_the_process() {
    let fixture = Fixture::new();
    fixture.accepts(&format!("--! out 1\n{}1{}", "(".repeat(24), ")".repeat(24)));
    for depth in [128, 1024, 4096] {
        for source in [
            format!("{}1{}", "(".repeat(depth), ")".repeat(depth)),
            format!("{}Nord", "!".repeat(depth)),
            std::iter::repeat_n("1", depth)
                .collect::<Vec<_>>()
                .join(" + "),
        ] {
            fixture.refuses(&source, "expression nested too deeply");
        }
    }
}

#[test]
fn selection_pipelines_preserve_rows_order_and_copy_counts() {
    let fixture = Fixture::new();
    std::fs::write(fixture.0.join("world.reg"), format!("{WORLD}col Count num 2 3 4 5\ncol Minion bool 0 0 0 0\nfn keep {{⊑𝕩}}\n")).unwrap();
    for k in [0, 1, 2, 3, 20] {
        let expected = [0, 2].into_iter().take(k).map(|i| i.to_string()).collect::<Vec<_>>().join(" ");
        fixture.accepts(&format!("--! out {expected}\nNord |> take {k}"));
        fixture.accepts(&format!("--! out\n(Nord & !Nord) |> take {k}"));
    }
    fixture.accepts("--! out 2 0\nNord |> order by Gold desc |> keep");
    fixture.accepts("--! out 2\n(Nord |> order by Gold desc) |> take 1");
    fixture.accepts("--! out 2 2 2 2\nNord |> order by Gold desc |> take 1 |> expand Count");
    fixture.accepts("--! out 4\nNord |> order by Gold desc |> take 1 |> expand Count , spawn Minion\n#/ Minion");
    fixture.accepts("--! out 3\nNord |> expand Count |> take 3 , spawn Minion\n#/ Minion");
    fixture.accepts("--! out 1\nNord |> expand Count |> take 1 |> keep , spawn Minion\n#/ Minion");
    fixture.refuses("Nord |> take 1.5", "take count must be a finite nonnegative integer");
}


#[test]
fn simultaneous_and_sequential_assignments_follow_their_state_contracts() {
    let fixture = Fixture::new();
    let mut seed = 0x243f6a88u32;
    for case in 0..8 {
        let mut next = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed % 101) as i32 - 50
        };
        let gold = [next(), next(), next(), next()];
        let silver = [next(), next(), next(), next()];
        let delta = next();
        let values = |v: &[i32]| v.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ");
        std::fs::write(fixture.0.join("world.reg"), format!(
            "n 4\ncol Nord bool 1 0 1 0\ncol Gold num {}\ncol Silver num {}\n",
            values(&gold), values(&silver)
        )).unwrap();
        for (effects, expected_gold, expected_silver) in [
            ("Silver = Gold ; Gold = Silver".to_string(), [silver[0], gold[1], silver[2], gold[3]], [gold[0], silver[1], gold[2], silver[3]]),
            ("Gold = Silver ; Silver = Gold".to_string(), [silver[0], gold[1], silver[2], gold[3]], [gold[0], silver[1], gold[2], silver[3]]),
            ("Silver = Gold |> Gold = Silver".to_string(), gold, [gold[0], silver[1], gold[2], silver[3]]),
            ("Gold = Silver |> Silver = Gold".to_string(), [silver[0], gold[1], silver[2], gold[3]], silver),
            (format!("Gold += {delta} |> Silver = Gold"), [gold[0]+delta, gold[1], gold[2]+delta, gold[3]], [gold[0]+delta, silver[1], gold[2]+delta, silver[3]]),
        ] {
            fixture.accepts(&format!("-- seed 0x243f6a88 case {case}\nNord , {effects}\n--! expect Gold = {}\n--! expect Silver = {}", values(&expected_gold), values(&expected_silver)));
        }
    }
}

#[test]
fn effect_sequences_compose_with_batches_guards_rules_and_comprehensions() {
    let fixture = Fixture::new();
    fixture.accepts("Nord , (Silver = Gold ; Gold = Silver) |> Gold += Silver\n--! expect Gold = 11 20 33 40\n--! expect Silver = 10 2 30 4");
    for effects in [
        "Silver = Gold ; Gold += Silver |> Marked = Gold > 20",
        "Gold += Silver |> Marked = Gold > 20 ; Silver = Gold",
    ] {
        fixture.accepts(&format!("Nord , {effects}\n--! expect Gold = 11 20 33 40\n--! expect Silver = 10 2 30 4\n--! expect Marked = 0 0 1 0"));
    }
    fixture.accepts("Nord , mentor = 0 |> Silver = mentor.Gold\n--! expect Silver = 10 2 10 4");
    fixture.accepts("Gold > 20 , Gold = 0 |> Silver = 99\n--! expect Silver = 1 2 99 99");
    fixture.accepts("Nord , Gold += 1 |>\n Silver = Gold |>\n Gold *= 2\n--! expect Gold = 22 20 62 40\n--! expect Silver = 11 2 31 4");
    fixture.accepts("Nord , +Marked |> Silver = Marked + 0\n--! expect Silver = 1 2 1 4");
    fixture.accepts("Nord , ping() |> Silver = Gold\n--! expect Silver = 11 2 31 4");
    fixture.accepts("Nord , Gold += 1\nGold += 1 |> Silver = Gold\n--! expect Silver = 12 2 32 4");
    fixture.accepts("[a & b , Gold += 1 |> Silver = Gold | a <- Nord, b <- Breton]\n--! expect Silver = 11 21 31 41");
    fixture.accepts("def a = Nord => Gold += 1 |> Silver = Gold\ndef b = !Nord => Gold += 2 |> Silver = Gold\n--! expect Silver = 11 22 31 42");
    fixture.refuses("Nord , Silver = Gold |> Gold = Silver = 1", "use '==' for comparison inside an effect");
    fixture.refuses("Nord , Gold = 1 ; Gold = 2 |> Silver = Gold", "no merge law");
    fixture.refuses("Nord , Silver = Gold |>", "expected effect");
}

#[test]
fn sequential_structural_effects_keep_the_subject_and_new_world() {
    let fixture = Fixture::new();
    std::fs::write(fixture.0.join("world.reg"), format!("{WORLD}col Minion bool 0 0 0 0\n")).unwrap();
    fixture.accepts("Nord , spawn Minion |> Gold += #/ Minion\n--! expect Gold = 12 20 32 40 0 0\n--! expect Minion = 0 0 0 0 1 1");
    fixture.accepts("Nord , spawn Minion |> ~\n--! expect Gold = 20 40 0 0\n--! expect Minion = 0 0 1 1");
    fixture.accepts("Nord , ~ |> spawn Minion\n--! expect Gold = 20 40\n--! expect Minion = 0 0");
    fixture.accepts("Nord , spawn Minion |> ~\n, Gold = 99\n--! expect Gold = 20 40 0 0");
    fixture.accepts("Nord , +Marked\n+Marked |> Silver = Gold\n--! expect Silver = 10 2 30 4");
    fixture.accepts("Nord , +Marked\n~ |> spawn Minion\n--! expect Gold = 20 40");

    fixture.accepts("Nord , spawn Minion |> Gold += #/ Minion ; Silver = #/ Minion\n--! expect Gold = 12 20 32 40 0 0\n--! expect Silver = 0 2 0 4 0 0");
}


#[test]
fn sequential_stages_observe_refined_values_and_keep_minted_keys() {
    let fixture = Fixture::new();
    std::fs::write(fixture.0.join("world.reg"), "n 2\ncol Nord bool 1 0\ncol Gold nat 10 20\nrange Gold 0 25\ncol Silver num 1 2\ncol Minion bool 0 0\nunique Key num 10 20\nrole keys Key\n").unwrap();
    fixture.accepts("Nord , Gold += 100 |> Silver = Gold\n--! expect Gold = 25 20\n--! expect Silver = 25 2");
    fixture.accepts("Nord , spawn Minion ; spawn Minion |> Silver = max/ Key @ Minion\n--! expect Silver = 22 2 0 0\n--! expect Key = 10 20 21 22");
    fixture.accepts("Nord , spawn Minion |> spawn Minion |> Silver = max/ Key @ Minion\n--! expect Silver = 22 2 0 0\n--! expect Key = 10 20 21 22");
}


#[test]
fn ordering_keys_and_rule_expansion_keep_their_own_domains() {
    let fixture = Fixture::new();
    std::fs::write(fixture.0.join("world.reg"), format!("{WORLD}col Count num 2 3 4 5\ncol Minion bool 0 0 0 0\n")).unwrap();
    fixture.accepts("--! out 2 0\nNord |> order by Gold desc |> order by 1");
    fixture.accepts("--! out\n(Nord & !Nord) |> order by 1 |> take 1");
    fixture.accepts("--! out 2\nNord |> order by mentor.Gold");
    fixture.accepts("--! out 14\ndef a = Nord |> expand Count => spawn Minion\ndef b = !Nord |> expand Count => spawn Minion\n#/ Minion");
    fixture.accepts("--! out 2\ndef a = Nord |> expand Count |> take 0 => spawn Minion\ndef b = !Nord => spawn Minion\n#/ Minion");
    fixture.accepts("--! out 4\n(Nord |> expand Count) & Gold > 20 , spawn Minion\n#/ Minion");
    fixture.accepts("--! out 8\n(Nord |> expand Count) & Gold > 20 , spawn Minion |> spawn Minion\n#/ Minion");
}
