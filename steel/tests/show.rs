// Display is observed through the real CLI and backend, including world-state expectations.
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(PathBuf);
impl Fixture {
    fn new(world: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!("ano-show-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("world.reg"), world).unwrap();
        Self(path)
    }
    fn tutorial() -> Self {
        Self::new(include_str!("../../demos/registries/144-show.reg"))
    }
    fn run(&self, source: &str, flags: &[&str]) -> Output {
        let file = self.0.join("input.ano");
        std::fs::write(&file, format!("--! registry world.reg\n{source}\n")).unwrap();
        Command::new(env!("CARGO_BIN_EXE_steel")).arg("--run").args(flags).arg(file).output().unwrap()
    }
    fn output(&self, source: &str) -> String {
        let result = self.run(source, &[]);
        assert!(result.status.success(), "{source}\n{}\n{}", String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
        String::from_utf8(result.stdout).unwrap()
    }
    fn refuses(&self, source: &str, message: &str) {
        let result = self.run(source, &[]);
        assert_eq!(result.status.code(), Some(2), "{source}\n{}", String::from_utf8_lossy(&result.stderr));
        assert!(String::from_utf8_lossy(&result.stderr).contains(message), "{}", String::from_utf8_lossy(&result.stderr));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}

fn cells(table: &str) -> Vec<Vec<&str>> {
    table.lines().map(|line| line.split_whitespace().collect()).collect()
}

#[test]
fn tutorial_rows_and_column_projection_are_displayed_without_writes() {
    let fixture = Fixture::tutorial();
    let table = fixture.output("IsHostile , show()\n--! expect Race = Nord Breton Khajiit Nord Imperial Redguard Argonian Nord Breton Khajiit\n--! expect IsHostile = 1 1 0 0 0 1 0 1 0 1\n--! expect TwoHanded = 80 55 70 55 90 60 45 72 40 88\n--! expect Archery = 40 70 85 30 50 75 60 55 65 90\n--! expect Gold = 100 200 300 400 500 600 150 350 90 520\n--! expect-n 10");
    assert_eq!(cells(&table), vec![
        vec!["row", "Race", "IsHostile", "TwoHanded", "Archery", "Gold"],
        vec!["0", "Nord", "1", "80", "40", "100"],
        vec!["1", "Breton", "1", "55", "70", "200"],
        vec!["5", "Redguard", "1", "60", "75", "600"],
        vec!["7", "Nord", "1", "72", "55", "350"],
        vec!["9", "Khajiit", "1", "88", "90", "520"],
    ]);
    let table = fixture.output("Race = :Nord , show(Gold, Race, TwoHanded)");
    assert_eq!(cells(&table), vec![vec!["row", "Gold", "Race", "TwoHanded"], vec!["0", "100", "Nord", "80"], vec!["3", "400", "Nord", "55"], vec!["7", "350", "Nord", "72"]]);
    let table = fixture.output("IsHostile & TwoHanded > 60 & Gold >= 300 & Race = :Nord , show()");
    assert_eq!(cells(&table)[1], vec!["7", "Nord", "1", "72", "55", "350"]);
}

#[test]
fn display_observes_the_effect_stage_and_supports_implicit_calls() {
    let fixture = Fixture::new("n 2\ncol Selected bool 1 0\ncol Gold num 10 20\nbind cursor entity 1\nfn show\n");
    let before = fixture.output("Selected , Gold += 5 ; show(Gold)\n--! expect Gold = 15 20");
    let after = fixture.output("Selected , Gold += 5 |> show(Gold)\n--! expect Gold = 15 20");
    assert_eq!(cells(&before), vec![vec!["row", "Gold"], vec!["0", "10"]]);
    assert_eq!(cells(&after), vec![vec!["row", "Gold"], vec!["0", "15"]]);
    assert_eq!(cells(&fixture.output("show(Gold)")), vec![vec!["row", "Gold"], vec!["1", "20"]]);
    assert_eq!(cells(&fixture.output("Selected , Gold += 5\nshow(Gold)")), vec![vec!["row", "Gold"], vec!["0", "15"]]);
    assert_eq!(cells(&fixture.output("Selected , ~ |> show(Gold)")), vec![vec!["row", "Gold"]]);
    let result = fixture.run("Selected , show(Gold)\n--! out 10 20\nGold", &["--label"]);
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    let text = String::from_utf8(result.stdout).unwrap();
    assert_eq!(text.lines().filter(|line| line.starts_with('\u{1d}')).count(), 2);
    assert!(text.contains("row  Gold\n0    10\n"), "{text}");
}

#[test]
fn display_handles_empty_selections_absence_and_unicode_cells() {
    let fixture = Fixture::new("n 2\ncol Selected bool 1 1\ncol 名前 sym 雪 火\ncol Glyph char ab\ncol Gold num -1 inf\npres Gold 1 0\nfn show\n");
    assert_eq!(fixture.output("Selected , show(名前, Glyph, Gold)"), "row  名前  Glyph  Gold\n0    雪    a      -1\n1    火    b      _\n");
    assert_eq!(fixture.output("Selected & !Selected , show(名前, Gold)"), "row  名前  Gold\n");
    let empty = Fixture::new("n 0\ncol Gold num\nfn show\n");
    assert_eq!(empty.output("Gold == Gold , show()"), "row  Gold\n");
}

#[test]
fn display_requires_registration_and_entity_column_arguments() {
    let fixture = Fixture::new("n 1\ncol Selected bool 1\ncol Gold num 10\nfn show\nas display show\n");
    assert_eq!(cells(&fixture.output("Selected , display(Gold)")), vec![vec!["row", "Gold"], vec!["0", "10"]]);
    fixture.refuses("Selected , show(Gold + 1)", "show arguments must name entity columns");
    fixture.refuses("Selected , show(Missing)", "unknown column");
    fixture.refuses("Selected , show(show)", "not an entity column");
    fixture.refuses("Selected , Gold = show()", "show is an output effect");
    fixture.refuses("Selected |> show()", "show is an output effect");
    Fixture::new("n 1\ncol Selected bool 1\n").refuses("Selected , show()", "needs a registered fn");
}

#[test]
fn display_escapes_control_characters_in_cells() {
    let fixture = Fixture::new("n 1\ncol Selected bool 1\ncol Label sym initial\nfn controls Label {s 𝕊 cs: ⟨\"a\"∾(@+29)∾\"b\"⟩}\nfn show\n");
    assert_eq!(fixture.output("Selected , controls() |> show(Label)"), "row  Label\n0    a\\x1db\n");
}


#[test]
fn showing_declared_columns_preserves_selection_and_projection_for_varied_rows() {
    let mut seed = 0x9e3779b9u32;
    for count in 0..12 {
        let mut flags = Vec::new();
        let mut gold = Vec::new();
        for _ in 0..count {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            flags.push(((seed >> 8) & 1).to_string());
            gold.push(((seed % 101) as i32 - 50).to_string());
        }
        let fixture = Fixture::new(&format!("n {count}\ncol Selected bool {}\ncol Gold num {}\nfn show\n", flags.join(" "), gold.join(" ")));
        let output = fixture.output(&format!("Selected , show(Gold, Selected)\n--! expect Gold = {}", gold.join(" ")));
        let expected = std::iter::once(vec!["row".to_string(), "Gold".to_string(), "Selected".to_string()])
            .chain((0..count).filter(|&i| flags[i] == "1").map(|i| vec![i.to_string(), gold[i].clone(), "1".to_string()]))
            .collect::<Vec<_>>();
        assert_eq!(cells(&output), expected, "seed 0x9e3779b9 rows {count}");
    }
}
