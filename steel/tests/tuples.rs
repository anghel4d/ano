// Tuple effects are checked through the CLI, backend, and persisted world boundary.
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const WORLD: &str = "n 4\ncol Nord bool 1 0 1 0\ncol Gold num 10 20 30 40\ncol Silver num 1 2 3 4\ncol Copper num 5 6 7 8\ncol Marked bool 0 0 0 0\ncol Race sym Nord Breton Nord Breton\nrel mentor -1 0 0 2\nbind cursor entity 1\nfn show\nas Money Gold\n";
struct Fixture(PathBuf);
impl Fixture {
    fn new(world: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!("ano-tuples-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("world.reg"), world).unwrap();
        Self(path)
    }
    fn run(&self, source: &str, flags: &[&str]) -> Output {
        let input = self.0.join("input.ano");
        std::fs::write(&input, format!("--! registry world.reg\n{source}\n")).unwrap();
        Command::new(env!("CARGO_BIN_EXE_steel")).args(flags).arg(input).output().unwrap()
    }
    fn accepts(&self, source: &str) {
        let output = self.run(source, &["--run"]);
        assert!(output.status.success(), "{source}\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    }
    fn refuses(&self, source: &str, cause: &str) {
        let output = self.run(source, &["--emit"]);
        assert_eq!(output.status.code(), Some(2), "{source}\n{}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stderr).contains(cause), "{}", String::from_utf8_lossy(&output.stderr));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}

#[test]
fn three_columns_accept_independent_constants_and_simultaneous_rotations() {
    let fixture = Fixture::new(WORLD);
    fixture.accepts("Nord , (Gold, Silver, Copper) = (4, 51, 13)\n--! expect Gold = 4 20 4 40\n--! expect Silver = 51 2 51 4\n--! expect Copper = 13 6 13 8");
    fixture.accepts("Nord , (Gold, Silver, Copper) = (Silver, Copper, Gold)\n--! expect Gold = 1 20 3 40\n--! expect Silver = 5 2 7 4\n--! expect Copper = 10 6 30 8");
    fixture.accepts("Nord , (Gold, Silver, Copper) += (Silver, Copper, Gold)\n--! expect Gold = 11 20 33 40\n--! expect Silver = 6 2 10 4\n--! expect Copper = 15 6 37 8");
    fixture.accepts("Nord , (Gold, Silver, Copper) *= (2, 3, 4)\n--! expect Gold = 20 20 60 40\n--! expect Silver = 3 2 9 4\n--! expect Copper = 20 6 28 8");
}

#[test]
fn tuple_arity_is_general_and_each_slot_reads_the_incoming_state() {
    let mut seed = 0x6a09e667u32;
    for arity in [1, 2, 3, 4, 7, 16, 65] {
        let mut world = "n 3\ncol Selected bool 1 0 1\n".to_string();
        let mut values = Vec::new();
        for slot in 0..arity {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let value = (seed % 1001) as i32 - 500;
            values.push(value);
            world.push_str(&format!("col C{slot} num {value} 999 {}\n", value + 1));
        }
        let fixture = Fixture::new(&world);
        let mut targets = (0..arity).map(|i| format!("C{i}")).collect::<Vec<_>>().join(", ");
        let mut reads = (0..arity).map(|i| format!("C{}", (i + 1) % arity)).collect::<Vec<_>>().join(", ");
        if arity == 1 { targets.push(','); reads.push(','); }
        let source = format!("-- seed 0x6a09e667 arity {arity}\nSelected , ({targets}) = ({reads})\n");
        let saved = fixture.0.join("after.reg");
        let output = fixture.run(&source, &["--run", "--save", saved.to_str().unwrap()]);
        assert!(output.status.success(), "{source}\n{}", String::from_utf8_lossy(&output.stderr));
        let reg = steel::registry::reg_load(saved.to_str().unwrap()).unwrap();
        for slot in 0..arity {
            let value = values[(slot + 1) % arity] as f64;
            let column = reg.ents.iter().find(|entry| entry.name == format!("C{slot}")).unwrap();
            let steel::RegEntryKind::Col { nums, .. } = &column.kind else { panic!("numeric column required") };
            assert_eq!(nums, &[value, 999.0, value + 1.0], "seed 0x6a09e667 arity {arity} slot {slot}");
        }
    }
}

#[test]
fn tuple_effects_compose_with_batches_pipelines_rules_and_existing_comprehensions() {
    let fixture = Fixture::new(WORLD);
    fixture.accepts("Nord , (Gold, Silver, Copper) = (Silver, Copper, Gold) |> Gold += Silver\n--! expect Gold = 6 20 10 40");
    fixture.accepts("Nord , (Gold, Silver, Copper) = (Silver, Copper, Gold) ; Marked = Gold > 20\n--! expect Marked = 0 0 1 0");
    fixture.accepts("Nord , ((Gold, Silver, Copper) = (4, 51, 13); +Marked) |> Silver = Gold\n--! expect Silver = 4 2 4 4\n--! expect Marked = 1 0 1 0");
    fixture.accepts("Nord , +Marked\n, (Gold, Silver, Copper) = (4, 51, 13)\n--! expect Copper = 13 6 13 8");
    fixture.accepts("(Gold, Silver, Copper) += (4, 51, 13)\n--! expect Gold = 10 24 30 40\n--! expect Silver = 1 53 3 4\n--! expect Copper = 5 19 7 8");
    fixture.accepts("def rotate = Nord => (Gold, Silver, Copper) = (Silver, Copper, Gold)\n--! expect Gold = 1 20 3 40\n--! expect Copper = 10 6 30 8");
    fixture.accepts("[a & b , (Gold, Silver, Copper) = (4, 51, 13) | a <- Nord, b <- !Nord]\n--! expect Gold = 4 4 4 4\n--! expect Copper = 13 13 13 13");
}

#[test]
fn nested_and_mixed_carrier_tuples_preserve_each_destination_contract() {
    let fixture = Fixture::new(WORLD);
    fixture.accepts("Nord , (Gold, (Marked, Race), Copper) = (Silver + 1, (Gold > 20, :Rich), 13)\n--! expect Gold = 2 20 4 40\n--! expect Marked = 0 0 1 0\n--! expect Race = Rich Breton Rich Breton\n--! expect Copper = 13 6 13 8");
    fixture.accepts("Nord , (\n Gold, Silver, Copper,\n) = (\n4, 51, 13,\n)\n--! expect Copper = 13 6 13 8");
    fixture.refuses("Nord , (Gold, Silver, Copper) = (4, 51)", "arity mismatch");
    fixture.refuses("Nord , (Gold, (Silver, Copper)) = ((4, 51), 13)", "matching tuple shapes");
    fixture.refuses("Nord , (Gold, Silver, Copper) = (4, :Rich, 13)", "cannot write sym result to number");
    fixture.refuses("Nord , (Gold, Money, Copper) = (4, 51, 13)", "repeats or overlaps target");
    fixture.refuses("Nord , (Gold + 1, Silver) = (4, 51)", "targets must name columns");
    fixture.refuses("Nord , (Gold, Silver) = (4, Copper = 13)", "use '==' for comparison inside an effect");
    fixture.refuses("Nord , (Gold, Silver) = (+\\Silver @ Nord, Gold)", "recurrence footprints must be disjoint");
}

#[test]
fn missing_tuple_members_skip_all_slots_for_that_row() {
    let fixture = Fixture::new(WORLD);
    fixture.accepts("Nord , (Gold, Silver, Copper) = (mentor.Gold, 51, 13) ; +Marked\n--! expect Gold = 10 20 10 40\n--! expect Silver = 1 2 51 4\n--! expect Copper = 5 6 13 8\n--! expect Marked = 1 0 1 0");
    fixture.accepts("Nord , (Gold, Silver, Copper) = (4, max/Gold @ (Nord & !Nord), 13)\n--! expect Gold = 10 20 30 40\n--! expect Silver = 1 2 3 4\n--! expect Copper = 5 6 7 8");
    let presence = Fixture::new(&format!("{WORLD}pres Silver 0 1 1 1\n"));
    presence.accepts("Nord , (Gold, Silver, Copper) = (Silver, Copper, Gold)\n--! expect Gold = 10 20 3 40\n--! expect Silver = 1 2 7 4\n--! expect Copper = 5 6 30 8");
}
