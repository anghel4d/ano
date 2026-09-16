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
