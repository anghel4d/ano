// Relational converse through Steel's public CLI and executed world results.
mod support;
use support::Fixture;

fn numbers(values: impl IntoIterator<Item = usize>) -> String {
    values.into_iter().map(|n| n.to_string()).collect::<Vec<_>>().join(" ")
}
const WORLD: &str = "n 5
col All bool 1 1 1 1 1
col Selected bool 1 0 0 0 0
col Gold num 2 4 8 16 32
col Out num 99 99 99 99 99
col Marked bool 0 0 0 0 0
rel parent -1 0 0 1 1
inv children parent
srel links 2 1 1 99 | 3 | 4 | | 0
as Rel links
ja 辺 links
fn echo {𝕩}
";

#[test]
fn live_edge_sets_reverse_and_restore_for_seeded_relations() {
    let mut seed = 0x243f6a88u32;
    for n in [1, 2, 5, 9] {
        for keyed in [false, true] {
            let keys: Vec<_> = (0..n).map(|i| if keyed { 10 + 7 * i } else { i }).collect();
            let mut edges = vec![vec![false; n]; n];
            for row in &mut edges {
                for edge in row {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    *edge = seed % 3 == 0;
                }
            }
            let groups = edges.iter().map(|row| {
                let mut members: Vec<_> = (0..n).rev().filter(|&j| row[j]).map(|j| keys[j]).collect();
                if let Some(first) = members.first().copied() { members.push(first); }
                members.push(999); // admissible, but no live endpoint
                numbers(members)
            }).collect::<Vec<_>>().join(" | ");
            let mut world = format!("n {n}
col All bool {}
col Gold num {}
col Out num {}
", numbers(vec![1;n]), numbers((0..n).map(|j| 1 << j)), numbers(vec![0;n]));
            if keyed { world.push_str(&format!("unique key num {}
srel key links {groups}
", numbers(keys))); }
            else { world.push_str(&format!("srel links {groups}
")); }
            let fixture = Fixture::new(&world);
            for suffix in ["", "'", "''", "'''", "''''"] {
                let reverse = suffix.len() % 2 == 1;
                let expected = numbers((0..n).map(|i| (0..n).filter(|&j| if reverse { edges[j][i] } else { edges[i][j] }).map(|j| 1 << j).sum()));
                fixture.accepts(&format!("-- seed 0x243f6a88 n={n} keyed={keyed} suffix={suffix}
All , Out = +/ links{suffix}.Gold
--! expect Out = {expected}"));
            }
            // Observe each adjacency row through an image, independently of grouped folds.
            for i in 0..n {
                let expected = numbers((0..n).map(|j| usize::from(edges[j][i])));
                fixture.accepts(&format!("Gold == {} , Out = 0
(Gold == {}).links' , Out = 1
--! out {expected}
(Gold == {}).links'", 1 << i, 1 << i, 1 << i));
            }
        }
    }
}

#[test]
fn direct_queries_images_and_functional_converse_share_edges() {
    let fixture = Fixture::new(WORLD);
    assert_eq!(fixture.accepts("parent'").trim(), "⟨ ⟨ 1 2 ⟩ ⟨ 3 4 ⟩ ⟨⟩ ⟨⟩ ⟨⟩ ⟩");
    assert_eq!(fixture.accepts("links").trim(), "⟨ ⟨ 1 2 ⟩ ⟨ 3 ⟩ ⟨ 4 ⟩ ⟨⟩ ⟨ 0 ⟩ ⟩");
    assert_eq!(fixture.accepts("links''"), fixture.accepts("links"));
    assert_eq!(fixture.accepts("parent''"), fixture.accepts("parent"));
    assert_eq!(fixture.accepts("parent'"), fixture.accepts("children"));
    fixture.accepts("Selected.links , +Marked
--! expect Marked = 0 1 1 0 0");
    fixture.accepts("Selected.links' , +Marked
--! expect Marked = 0 0 0 0 1");
    fixture.accepts("(Gold == 8).parent , +Marked
--! expect Marked = 1 0 0 0 0");
    fixture.accepts("--! out 2 1 1 0 1
#/ links");
    fixture.accepts("--! out 1 1 1 1 1
#/ links'");
    fixture.accepts("--! out 0 1 1 0 0
Selected.links");
}

#[test]
fn composition_prime_precedence_aliases_and_definitions() {
    let fixture = Fixture::new(WORLD);
    for expression in ["(links.links)'", "links'.links'"] {
        fixture.accepts(&format!("All , Out = +/ ({expression}).Gold
--! expect Out = 8 32 32 2 2"));
    }
    assert_eq!(fixture.accepts("(links.links')'"), fixture.accepts("links.links'"));
    for expression in ["Rel'", "^Rel'", "def reverse = links'
reverse", "def forward = links''
forward'", "--! ja
辺'", "--! ja
辺 '", "--! ja
^辺'"] {
        assert_eq!(fixture.accepts(expression), fixture.accepts("links'"), "{expression}");
    }
    for expression in ["--! ja
辺''", "--! ja
( 辺 )''", "--! ja
辺 ' '", "(links)''"] {
        assert_eq!(fixture.accepts(expression), fixture.accepts("links"), "{expression}");
    }
}

#[test]
fn group_reductions_filter_missing_values_and_guard_empty_results() {
    let fixture = Fixture::new(&format!("{WORLD}pres Gold 1 0 1 0 1
"));
    fixture.accepts("All , Out = +/ links.Gold
--! expect Out = 8 0 32 0 2");
    fixture.accepts("All , Out = #/ links.Gold
--! expect Out = 1 0 1 0 1");
    fixture.accepts("All , Out = max/ links.Gold
--! expect Out = 8 99 32 99 2");
    fixture.accepts("All , Out = avg/ links.Gold
--! expect Out = 8 99 32 99 2");
    fixture.accepts("--! out 8 32 2
max/ links.Gold");
    fixture.accepts("All , Out = #/ (links & (Gold > 10))
--! expect Out = 0 0 1 0 0");
    fixture.accepts("--! out 0 0 0 1 0
&/ links.Marked");
    fixture.accepts("--! out 0 0 0 0 0
|/ links.Marked");
}

#[test]
fn numeric_key_columns_obey_presence_and_canonical_identity() {
    let fixture = Fixture::new("n 4
unique id num 40 10 30 20
role id id
col Bind num 10 40 10 -1
pres Bind 1 0 1 1
col Gold num 2 4 8 16
col All bool 1 1 1 1
col Out num 0 0 0 0
");
    fixture.accepts("--! out 0 2 0 0
#/ Bind'");
    fixture.accepts("All , Out = +/ Bind'.Gold
--! expect Out = 0 10 0 0");
    assert_eq!(fixture.accepts("Bind''"), fixture.accepts("Bind"));
}

#[test]
fn relation_reads_follow_stage_boundaries_and_live_row_identity() {
    let fixture = Fixture::new("n 4
unique id num 40 10 30 20
role id id
col All bool 1 1 1 1
col Delete bool 0 1 0 0
col Out num 0 0 0 0
col Gold num 2 4 8 16
rel parent -1 0 1 2
srel links 1 2 | 2 | 3 | 0
srel id keyed 10 30 | 30 | 20 | 40
");
    fixture.accepts("Delete , ~
All , Out = +/ links.Gold
--! expect Out = 8 16 2");
    fixture.accepts("Delete , ~
All , Out = +/ links'.Gold
--! expect Out = 16 2 8");
    fixture.accepts("Delete , ~
All , Out = +/ keyed'.Gold
--! expect Out = 16 2 8");
    fixture.accepts("Delete , ~
All , Out = +/ parent'.Gold
--! expect Out = 0 16 0");
    fixture.accepts("All , parent = 0 |> Out = #/ parent'
--! expect Out = 4 0 0 0");
    fixture.accepts("All , parent = 0 ; Out = #/ parent'
--! expect Out = 1 1 1 0");
}

#[test]
fn invalid_operands_and_group_scatter_refuse_before_execution() {
    let fixture = Fixture::new(WORLD);
    for source in ["All'", "1'", "(Gold + 1)'", "(links.Gold)'", "(Gold, Gold)'", "(1)''"] {
        fixture.refuses(source, "prime requires");
    }
    for source in ["All , Out = links", "All , Out = links'", "All , Out = links.Gold", "def grouped = links'.Gold
All , Out = grouped"] {
        fixture.refuses(source, "relational groups");
    }
    fixture.refuses("All , Out = echo(links.Gold)", "relational groups");
    fixture.refuses("All , Out = links.Gold + 1", "relational groups");
    fixture.refuses("links.Gold > 1", "relational groups");
    fixture.refuses(&format!("(links){}", "'".repeat(100)), "deeply");
    fixture.refuses(&format!("links{}", "'".repeat(100)), "deeply");
}

#[test]
fn inverses_are_live_reads_and_positional_edges_survive_save_and_reopen() {
    let fixture = Fixture::new(WORLD);
    fixture.accepts("All , parent = 0 |> Out = #/ children
--! expect Out = 5 0 0 0 0");
    fixture.accepts("All , parent = 0 ; Out = #/ children
--! expect Out = 2 2 0 0 0");
    // Save after deleting original row 1, with no intervening relation read.
    let saved = fixture.0.join("after.reg");
    let output = fixture.run("Gold == 4 , ~", &["--run", "--save", saved.to_str().unwrap()]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let reopened = Fixture::new(&std::fs::read_to_string(&saved).unwrap());
    reopened.accepts("All , Out = +/ links.Gold
--! expect Out = 8 32 0 2");
    reopened.accepts("All , Out = +/ links'.Gold
--! expect Out = 32 2 0 8");
    reopened.accepts("All , Out = #/ parent'
--! expect Out = 1 0 0 0");
    assert_eq!(reopened.accepts("children"), reopened.accepts("parent'"));
}

#[test]
fn empty_worlds_and_spawned_sources_have_well_formed_converse() {
    let fixture = Fixture::new("n 0
col Gold num
rel parent
inv children parent
");
    fixture.accepts("--! out
#/ parent'");
    assert_eq!(fixture.accepts("parent'").trim(), "⟨⟩");
    let fixture = Fixture::new(WORLD);
    fixture.accepts("Selected , spawn Selected
All , Out = #/ parent'
--! expect Out = 2 2 0 0 0 0");
    fixture.accepts("Selected , spawn Selected
All , Out = #/ children
--! expect Out = 2 2 0 0 0 0");
}

#[test]
fn converse_traces_the_source_domain_and_respects_silent_absence() {
    let fixture = Fixture::new("n 4
col Selected bool 1 0 0 0
col Out num 0 0 0 0
rel parent -1 99 0 0
");
    let output = fixture.run("Selected , Out = #/ parent'", &["--run", "--trace"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(text.contains("parent 1 -> 99 IS DEAD"), "{text}");
    assert!(text.contains("EFFECT SOURCE"), "{text}");
    assert!(!text.contains("-> -1 IS DEAD"), "{text}");
}
