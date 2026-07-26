// The todo/03 acceptance matrix: folds, scans, Greater/Lesser, and empty results.  Everything
// runs in process through the public boundary (lex -> parse -> emit_with_aliases) with an
// in-memory Registry — no BQN interpreter, no demo fixtures, no sidecar files.  CLAUDE.md rules
// BQN witnesses out as semantic oracles, so the two things under test are the emitted plan and
// the descriptor table itself; the property group holds a Rust reference beside the descriptors.

use steel::reducer::{self, Carrier, Form, OpDesc};
use steel::alias::AliasEnvironment;
use steel::emit::emit_with_aliases;
use steel::lex::lex;
use steel::parse::parse;
use steel::{AliasRow, ColType, Diag, Directives, Expect, Interner, RegEntry, RegEntryKind, Registry};

fn col(name: &str, ty: ColType, nums: Vec<f64>) -> RegEntry {
    RegEntry {
        name: name.to_string(),
        defval: 0.0,
        kind: RegEntryKind::Col { ty, uniq: false, nums, syms: Vec::new(), pres: None, rng: None },
    }
}

// A char column stores its whole glyph run as one sym; the loader spells it as a BQN string.
fn glyphs(name: &str, run: &str) -> RegEntry {
    RegEntry {
        name: name.to_string(),
        defval: 0.0,
        kind: RegEntryKind::Col {
            ty: ColType::Char,
            uniq: false,
            nums: Vec::new(),
            syms: vec![run.to_string()],
            pres: None,
            rng: None,
        },
    }
}

// Four rows: two numeric columns, two masks, one glyph run, one registered reducer, one
// set-valued relation with an empty fiber (row 1), and the two Nihongo spellings the parity
// group needs.
fn registry() -> Registry {
    Registry {
        n: 4,
        ents: vec![
            col("Gold", ColType::Num, vec![1.0, 2.0, 3.0, 4.0]),
            col("Silver", ColType::Num, vec![4.0, 3.0, 2.0, 1.0]),
            col("Burning", ColType::Bool, vec![1.0, 0.0, 1.0, 1.0]),
            col("Path", ColType::Bool, vec![1.0, 1.0, 0.0, 0.0]),
            glyphs("Rune", "gene"),
            RegEntry {
                name: "threat".to_string(),
                defval: 0.0,
                kind: RegEntryKind::Fn { body: Some("{𝕨⌈𝕩}".to_string()) },
            },
            RegEntry {
                name: "near".to_string(),
                defval: 0.0,
                kind: RegEntryKind::SRel {
                    fib: vec![vec![1.0], vec![], vec![0.0, 3.0], vec![2.0]],
                    key_of: None,
                    inv_of: None,
                },
            },
        ],
        aliases: vec![
            AliasRow { from: "金".to_string(), to: "Gold".to_string(), ja: true },
            AliasRow { from: "燃".to_string(), to: "Burning".to_string(), ja: true },
            AliasRow { from: "印".to_string(), to: "Rune".to_string(), ja: true },
        ],
        ..Registry::default()
    }
}

// Inputs: source, the surface flag, the emission directives. Output: the emitted BQN or the
// refusal. One emission per call: a fresh interner and a snapshot frozen at the boundary.
fn plan(src: &str, ja: bool, dirs: &Directives) -> Result<String, Diag> {
    let reg = registry();
    let env = AliasEnvironment::for_registry(&reg);
    let mut it = Interner::new();
    let toks = lex(src.as_bytes(), ja, &mut it)?;
    let prog = parse(&toks, &mut it)?;
    let snap = env.snapshot(&reg)?;
    emit_with_aliases(&prog, &reg, dirs, &it, snap)
}

fn ok(src: &str) -> String {
    match plan(src, false, &Directives::default()) {
        Ok(text) => text,
        Err(d) => panic!("{}: {}", src.trim_end(), d.msg),
    }
}

fn ok_ja(src: &str) -> String {
    match plan(src, true, &Directives::default()) {
        Ok(text) => text,
        Err(d) => panic!("{}: {}", src.trim_end(), d.msg),
    }
}

fn err(src: &str) -> String {
    match plan(src, false, &Directives::default()) {
        Ok(_) => panic!("{}: expected a refusal", src.trim_end()),
        Err(d) => d.msg,
    }
}

fn err_ja(src: &str) -> String {
    match plan(src, true, &Directives::default()) {
        Ok(_) => panic!("{}: expected a refusal", src.trim_end()),
        Err(d) => d.msg,
    }
}

// The statement region of an emission: the fixture prelude and the expectation tail are not
// under test, and the prelude carries declarations whose presence depends on other statements.
fn body(bqn: &str) -> String {
    let start = bqn
        .find("\n# q1\n")
        .or_else(|| bqn.find("\n# s1\n"))
        .unwrap_or_else(|| panic!("no statement region in\n{}", bqn));
    let end = bqn.find("\n# expectations\n").expect("expectations");
    bqn[start..end].to_string()
}

/* ---------- 1. positive emission ---------- */

// The charter's chain (todo/03:159), started on a literal so no leading column fixes the shape.
// Greater over the numeric carrier is q's maximum, lowered through one registered semantic fn.
#[test]
fn greater_chain_lowers_to_the_numeric_maximum() {
    let text = ok("1 | 7 | 9 | 8 | 6 | 1 | 9 | 8 | 99 | 1 | 23 | 4 | 5 | 174 | 1 | 2 | 3\n");
    assert!(text.contains("Fn_AnoSemGreater0 ← {𝕨⌈𝕩}"), "{}", text);
    assert_eq!(text.matches("Fn_AnoSemGreater0¨").count(), 16, "{}", text);
    assert!(text.contains("Fn_AnoSemGreater0¨174)"), "{}", text);
    // one instance selected once: no mask glyph leaks into a numeric chain
    assert!(!body(&text).contains('∨'), "{}", text);
}

// Column against column and column against scalar select the same numeric instance.
#[test]
fn numeric_greater_and_lesser_take_columns_and_scalars() {
    assert!(ok("Gold | 5\n").contains("q1 ← (gold Fn_AnoSemGreater0¨5)"));
    assert!(ok("Gold | Silver\n").contains("q1 ← (gold Fn_AnoSemGreater0¨silver)"));
    assert!(ok("Gold & 5\n").contains("q1 ← (gold Fn_AnoSemLesser0¨5)"));
    assert!(ok("Gold & Silver\n").contains("q1 ← (gold Fn_AnoSemLesser0¨silver)"));
}

// The mask truth tables are untouched by the carrier dispatch: ∧ ∨ ¬ as always.
#[test]
fn mask_truth_tables_are_unchanged() {
    assert_eq!(body(&ok("Burning & Path\n")), "\n# q1\nq1 ← (burning∧path)\n•Show q1\n");
    assert_eq!(body(&ok("Burning | Path\n")), "\n# q1\nq1 ← (burning∨path)\n•Show q1\n");
    assert_eq!(body(&ok("!Burning\n")), "\n# q1\nq1 ← (¬burning)\n•Show q1\n");
}

// Long forms are the same operation as the glyphs, and the ordered non-associative steps lower
// through the exact left recurrence — subtraction and division reverse, never reassociate.
#[test]
fn long_forms_and_ordered_steps_lower_left() {
    assert_eq!(ok("fold(+) Gold\n"), ok("+/ Gold\n"));
    assert_eq!(ok("scan(+) Gold\n"), ok("+\\ Gold\n"));
    assert_eq!(ok("fold(-) Gold\n"), ok("-/ Gold\n"));
    assert_eq!(ok("scan(-) Gold\n"), ok("-\\ Gold\n"));
    assert_eq!(ok("scan(/) Gold\n"), ok("/\\ Gold\n"));

    let minus = ok("fold(-) Gold\n");
    assert!(minus.contains("AnoLeftSubtract ← {-˜´⌽𝕩}"), "{}", minus);
    assert!(minus.contains("t0 ← {0=≠𝕩 ? 0 ; AnoLeftSubtract 𝕩} gold"), "{}", minus);
    let divide = ok("fold(/) Gold\n");
    assert!(divide.contains("AnoLeftDivide ← {÷˜´⌽𝕩}"), "{}", divide);
    // BQN's scan modifier is natively left-to-right, so the scans need no rewriting
    assert_eq!(body(&ok("-\\ Gold\n")), "\n# q1\nq1 ← (-`gold)\n•Show q1\n");
    assert_eq!(body(&ok("/\\ Gold\n")), "\n# q1\nq1 ← (÷`gold)\n•Show q1\n");
    // a registered name folds pairwise left and scans through the same fn
    assert!(ok("fold(threat) Gold\n").contains("Fn_threat˜´⌽𝕩"));
    assert_eq!(body(&ok("threat\\ Gold\n")), "\n# q1\nq1 ← (Fn_threat`gold)\n•Show q1\n");
}

// scan(f) col along ord resolves through one table for every instance, including the machines.
#[test]
fn scan_along_covers_every_instance() {
    let cases = [
        ("scan(+) Gold along Silver\n", "q1 ← (+`(silver)⊏gold)"),
        ("scan(*) Gold along Silver\n", "q1 ← (×`(silver)⊏gold)"),
        ("scan(min) Gold along Silver\n", "q1 ← (⌊`(silver)⊏gold)"),
        ("scan(max) Gold along Silver\n", "q1 ← (⌈`(silver)⊏gold)"),
        ("scan(&) Burning along Silver\n", "q1 ← (∧`(silver)⊏burning)"),
        ("scan(|) Burning along Silver\n", "q1 ← (∨`(silver)⊏burning)"),
        ("scan(threat) Gold along Silver\n", "q1 ← (Fn_threat`(silver)⊏gold)"),
    ];
    for (source, pin) in cases {
        assert!(ok(source).contains(pin), "{}", source);
    }
    // the mean's two prefix sums carry one and the same order, so composing them is one domain
    assert!(ok("scan(avg) Gold along Silver\n")
        .contains("q1 ← ((+`(silver)⊏gold)÷(+`(silver)⊏(¬(¬(1¨gold)))))"));
}

// Count is a prefix machine over selection presence, not a homogeneous reducer over a payload:
// a numeric operand is the all-true membership stream, and a scope scopes the presence too.
#[test]
fn count_and_average_lower_as_prefix_machines() {
    assert_eq!(body(&ok("#\\ Gold\n")), "\n# q1\nq1 ← (+`(¬(¬(1¨gold))))\n•Show q1\n");
    assert_eq!(body(&ok("#\\ Burning\n")), "\n# q1\nq1 ← (+`(¬(¬burning)))\n•Show q1\n");
    assert!(ok("#\\ Gold @ Path\n").contains("q1 ← (+`path/(¬(¬(1¨gold))))"));
    assert!(ok("avg\\ Gold\n").contains("q1 ← ((+`gold)÷(+`(¬(¬(1¨gold)))))"));
    assert!(ok("avg\\ Gold @ Path\n").contains("q1 ← ((+`path/gold)÷(+`path/(¬(¬(1¨gold)))))"));
    // the mean's finish accumulates in the machine's order, so avg/ is the last prefix of avg\
    let mean = ok("avg/ Gold\n");
    assert!(mean.contains("AnoSemAverage ← {(AnoLeftSum 𝕩)÷≠𝕩}"), "{}", mean);
    assert!(mean.contains("AnoLeftSum ← {+˜´⌽(0∾𝕩)}"), "{}", mean);
    assert!(mean.contains("t0 ← {0=≠𝕩 ? 0 ; AnoSemAverage 𝕩} gold"), "{}", mean);
    // the fold direction: cardinality, and the machine's registered empty law needs no guard.
    // A 0/1 mask sums to the same value in either order, so the count's finish carries no
    // reversal — the operand buys that, not the machine.
    assert!(ok("#/ Gold @ Burning\n").contains("q1 ← (+´burning)"));
    assert!(ok("#/ near'\n").contains("q1 ← (≠¨near)"));
}

/* ---------- 2. bridges ---------- */

// `max`/`min` are not lookalikes of the numeric `|`/`&`: one resolved operation, so one program.
#[test]
fn bridge_spellings_emit_one_program() {
    assert_eq!(ok("max/ Gold\n"), ok("|/ Gold\n"));
    assert_eq!(ok("min/ Gold\n"), ok("&/ Gold\n"));
    assert_eq!(ok("max\\ Gold\n"), ok("|\\ Gold\n"));
    assert_eq!(ok("min\\ Gold\n"), ok("&\\ Gold\n"));
    assert_eq!(ok("fold(max) Gold\n"), ok("fold(|) Gold\n"));
    assert_eq!(ok("fold(min) Gold\n"), ok("fold(&) Gold\n"));
    assert_eq!(ok("scan(max) Gold along Silver\n"), ok("scan(|) Gold along Silver\n"));
    assert_eq!(ok("scan(min) Gold along Silver\n"), ok("scan(&) Gold along Silver\n"));
}

// Ano's numeric carrier is finite float64 (todo/03:87): the extrema carry no identity, so no
// seed is prepended and no infinity is ever manufactured.
#[test]
fn extrema_never_seed_and_never_manufacture_infinity() {
    for source in
        ["max/ Gold\n", "min/ Gold\n", "max\\ Gold\n", "min\\ Gold\n", "max/ Gold @ Burning\n"]
    {
        let text = ok(source);
        assert!(!text.contains('∞'), "{}: {}", source, text);
        assert!(!text.contains("⌈˜´⌽("), "{}: {}", source, text);
        assert!(!text.contains("⌊˜´⌽("), "{}: {}", source, text);
    }
    assert!(ok("max/ Gold\n").contains("AnoLeftMaximum ← {⌈˜´⌽𝕩}"));
    assert!(ok("min/ Gold\n").contains("AnoLeftMinimum ← {⌊˜´⌽𝕩}"));
    // and the identity-bearing folds keep the seeds that give them their empty results
    assert!(ok("+/ Gold\n").contains("AnoLeftSum ← {+˜´⌽(0∾𝕩)}"));
    assert!(ok("*/ Gold\n").contains("AnoLeftProduct ← {×˜´⌽(1∾𝕩)}"));
    assert!(ok("&/ Burning\n").contains("AnoLeftAnd ← {∧˜´⌽(1∾𝕩)}"));
    assert!(ok("|/ Burning\n").contains("AnoLeftOr ← {∨˜´⌽(0∾𝕩)}"));
}

/* ---------- 2b. the char carrier ---------- */

// A9 adopts q's Greater and Lesser over the carriers Ano admits, and Ano admits char, so the
// char instances are a derivation and not a new ruling.  BQN's ⌈ and ⌊ refuse characters, so the
// step travels through code points; one declared step serves the dyad, the fold and the scan.
#[test]
fn char_greater_and_lesser_step_through_code_points() {
    let greater = ok("max/ Rune\n");
    assert!(greater.contains("AnoCharGreater ← {@+(𝕨-@)⌈𝕩-@}"), "{}", greater);
    assert!(greater.contains("AnoLeftCharMaximum ← {AnoCharGreater˜´⌽𝕩}"), "{}", greater);
    let lesser = ok("min/ Rune\n");
    assert!(lesser.contains("AnoCharLesser ← {@+(𝕨-@)⌊𝕩-@}"), "{}", lesser);
    assert!(lesser.contains("AnoLeftCharMinimum ← {AnoCharLesser˜´⌽𝕩}"), "{}", lesser);
    // the bridges are the same operation, so they are the same program
    assert_eq!(ok("|/ Rune\n"), greater);
    assert_eq!(ok("&/ Rune\n"), lesser);
    // BQN's scan is already the left recurrence, so a scan owes only the step's declaration
    assert!(body(&ok("|\\ Rune\n")).contains("q1 ← (AnoCharGreater`rune)"), "{}", ok("|\\ Rune\n"));
    assert!(body(&ok("&\\ Rune\n")).contains("q1 ← (AnoCharLesser`rune)"), "{}", ok("&\\ Rune\n"));
    assert_eq!(ok("max\\ Rune\n"), ok("|\\ Rune\n"));
    assert_eq!(ok("min\\ Rune\n"), ok("&\\ Rune\n"));
    // and the direct dyad materializes ONE registry fn carrying that same body
    let direct = ok("Rune | Rune\n");
    assert!(direct.contains("Fn_AnoSemCharGreater0 ← {@+(𝕨-@)⌈𝕩-@}"), "{}", direct);
    assert!(body(&direct).contains("(rune Fn_AnoSemCharGreater0¨rune)"), "{}", direct);
    assert!(ok("Rune & Rune\n").contains("Fn_AnoSemCharLesser0 ← {@+(𝕨-@)⌊𝕩-@}"));
    // a string literal is a glyph run and reads on the same carrier
    assert!(ok("\"sat\" | \"cow\"\n").contains("Fn_AnoSemCharGreater0 ← {@+(𝕨-@)⌈𝕩-@}"));
}

// The char order has no greatest or least rune, so it declares no identity: nothing is seeded,
// the empty scope reaches the validity channel, and no code-point zero is ever manufactured.
#[test]
fn char_extrema_declare_no_identity() {
    for source in ["max/ Rune\n", "min/ Rune\n", "max/ Rune @ Path\n", "|\\ Rune\n"] {
        let text = ok(source);
        assert!(!text.contains("⌽(@"), "{}: {}", source, text);
        assert!(!text.contains('∞'), "{}: {}", source, text);
    }
    let guarded = ok("max/ Rune @ Path\n");
    assert!(guarded.contains("{0=≠𝕩 ? 0 ; AnoLeftCharMaximum 𝕩} (path/rune)"), "{}", guarded);
    assert!(guarded.contains("q1v ← (0<(+´path))"), "{}", guarded);
    assert!(guarded.contains("•Show⍟q1v q1"), "{}", guarded);
}

// A9 adopts q's Greater/Lesser CONVENTION over Ano's carriers, not q's promotions: q lifts a
// char to an int in `98 | "a"` and Ano refuses it.  A glyph has no selection reading to fall
// back on, so a char mixture refuses in every position, not only in a value one.
#[test]
fn mixed_char_carriers_refuse() {
    for (source, message) in [
        ("98 | \"a\"\n", "'|' mixes number and char operands; there is no carrier coercion"),
        ("Rune | Gold\n", "'|' mixes number and char operands; there is no carrier coercion"),
        ("Rune & Burning\n", "'&' mixes mask and char operands; there is no carrier coercion"),
        ("Silver = (Gold | Rune)\n", "'|' mixes number and char operands; there is no carrier coercion"),
    ] {
        assert_eq!(err(source), format!("emit: line 1: {}", message), "{}", source);
    }
    // and the mask/number rule is unchanged, named in the same fixed carrier order
    assert_eq!(
        err("Silver = (Gold | Burning)\n"),
        "emit: line 1: '|' mixes mask and number operands; there is no carrier coercion"
    );
    // a predicate is selection, not Greater: the mask reading survives untouched
    assert!(ok("Rune & Burning , +Path\n").contains("burning"));
}

// The char carrier admits Greater, Lesser and their two bridges, and nothing else.
#[test]
fn char_admits_only_greater_and_lesser() {
    for (source, message) in [
        ("+/ Rune\n", "reducer '+' is not defined on Char"),
        ("*/ Rune\n", "reducer '*' is not defined on Char"),
        ("-/ Rune\n", "reducer '-' is not defined on Char"),
        ("avg/ Rune\n", "reducer 'avg' is not defined on Char"),
        ("avg\\ Rune\n", "reducer 'avg' is not defined on Char"),
        ("threat/ Rune\n", "reducer 'threat' is not defined on Char"),
        ("scan(+) Rune along Gold\n", "reducer '+' is not defined on Char"),
    ] {
        assert_eq!(err(source), format!("emit: line 1: {}", message), "{}", source);
    }
    // the canonical char spellings are internal: no surface admits them as written
    assert_eq!(err("charmax/ Rune\n"), "emit: line 1: unknown reducer 'charmax'");
    assert_eq!(err("charmin/ Gold\n"), "emit: line 1: unknown reducer 'charmin'");
    // count consumes presence whatever the payload is, so it counts glyphs
    assert!(body(&ok("#/ Rune\n")).contains("q1 ← (+´(1¨rune))"), "{}", ok("#/ Rune\n"));
}

/* ---------- 3. empty results ---------- */

// Forms with a declared empty result stay real scalars: no guard, no conditional display.
#[test]
fn identity_folds_stay_unguarded_scalars() {
    for source in [
        "+/ Gold @ Burning\n",
        "*/ Gold @ Burning\n",
        "#/ Gold @ Burning\n",
        "&/ Burning @ Path\n",
        "|/ Burning @ Path\n",
    ] {
        let text = ok(source);
        assert!(!text.contains("q1v"), "{}: {}", source, text);
        assert!(text.contains("\n•Show q1\n"), "{}: {}", source, text);
    }
}

// A12 at the only layer that can express runtime skipping.  The guard is staged once and every
// observation runs under it: bare display, the 0x1D label, and the `--! out` comparator.
#[test]
fn identityless_empty_fold_cannot_expose_its_placeholder() {
    let bare = ok("max/ Gold @ Burning\n");
    // the guard counts the rows the scope admits: the emitter's own reduction, in BQN's order
    assert!(bare.contains("q1v ← (0<(+´burning))"), "{}", bare);
    assert!(bare.contains("•Show⍟q1v q1"), "{}", bare);
    assert!(!bare.contains("\n•Show q1\n"), "{}", bare);

    let labelled = plan(
        "max/ Gold @ Burning\n",
        false,
        &Directives { label: true, ..Directives::default() },
    )
    .expect("emit");
    // one conditional carries BOTH the tag line and the value: a false guard emits neither, so
    // Kore receives no QRec and shows no OUTPUTS row
    assert!(labelled.contains("{•Out (@+29)∾\"q1@1\" ⋄ •Show 𝕩}⍟q1v q1"), "{}", labelled);
    assert!(!labelled.contains("\n•Out (@+29)∾\"q1@1\"\n"), "{}", labelled);

    let pinned = plan(
        "max/ Gold @ Burning\n",
        false,
        &Directives {
            expects: vec![Expect::Out { vals: Vec::new() }],
            ..Directives::default()
        },
    )
    .expect("emit");
    // the comparator consumes the guard-compressed ravel, so `--! out` (empty) passes under a
    // false guard and `--! out 0` fails — the placeholder is unpinnable
    assert!(pinned.contains("q1v/⥊q1"), "{}", pinned);
    assert!(!pinned.contains("} ⥊q1"), "{}", pinned);

    // every identityless head reaches the same channel: extrema, mean, a registered step, and
    // the ordered non-associative steps
    for source in
        ["min/ Gold\n", "avg/ Gold\n", "threat/ Gold\n", "-/ Gold\n", "fold(/) Gold\n"]
    {
        let text = ok(source);
        assert!(text.contains("q1v ← "), "{}: {}", source, text);
        assert!(text.contains("•Show⍟q1v q1"), "{}: {}", source, text);
    }
}

// A per-row guard is a per-row result: an empty fiber produces no result row, mirroring the
// assignment path, which already drops those rows from the scatter mask.
#[test]
fn grouped_query_compresses_empty_fibers() {
    for source in ["max/ near'.Gold\n", "min/ near'.Gold\n", "avg/ near'.Gold\n"] {
        let text = ok(source);
        assert!(text.contains("q1 ← ((0<≠¨near))/t0"), "{}: {}", source, text);
        assert!(text.contains("\n•Show q1\n"), "{}: {}", source, text);
    }
}

// The per-row fold over a fiber column owes the same channel: it applies its helper once per
// row, so without a guard an empty fiber aborts the whole program on BQN's missing identity
// rather than dropping one row.  A12 rules the answer is no result row.
#[test]
fn row_fold_without_an_identity_drops_its_empty_rows() {
    for (source, helper) in
        [("max/ near@row\n", "AnoLeftMaximum"), ("min/ near@row\n", "AnoLeftMinimum")]
    {
        let text = ok(source);
        assert!(text.contains(&format!("{{0=≠𝕩 ? 0 ; {} 𝕩}}¨near", helper)), "{}: {}", source, text);
        assert!(text.contains("q1 ← ((0<≠¨near))/t0"), "{}: {}", source, text);
        assert!(text.contains("\n•Show q1\n"), "{}: {}", source, text);
    }
    // an identity answers the empty fiber itself, so that form stays unguarded and total
    let seeded = ok("+/ near@row\n");
    assert!(seeded.contains("q1 ← ({AnoLeftSum 𝕩}¨near)"), "{}", seeded);
    assert!(!seeded.contains("0=≠𝕩 ?"), "{}", seeded);
    // and the assignment path drops the same rows from the scatter mask
    let written = ok("Burning , Gold = max/ near@row\n");
    assert!(written.contains("(0<≠¨near)"), "{}", written);
}

// Count consumes selection presence, never a numeric payload: `#/ rel'.Comp` is the cardinality
// of the fiber's admitted elements, as q's count and Haskell's length are.  The normalizer wraps
// the hopped component in the presence reading, exactly as it does for the scan forms.
#[test]
fn count_over_a_fiber_hop_counts_rather_than_sums() {
    let counted = body(&ok("#/ near'.Silver\n"));
    assert!(counted.contains("t0 ← {+´𝕩⊏(¬(¬(1¨silver)))}¨near"), "{}", counted);
    // the payload itself never reaches the reduction
    assert!(!counted.contains("+´𝕩⊏silver"), "{}", counted);
    // a bare fiber counts its members, and the hop through a total column agrees with it
    assert!(body(&ok("#/ near'\n")).contains("(≠¨"), "{}", ok("#/ near'\n"));
    // over a mask the machine advances on the true rows, which is the same presence stream
    assert!(body(&ok("#/ near'.Burning\n")).contains("{+´𝕩⊏(¬(¬burning))}¨near"));
    // the sum is still the sum: only count was ever meant to consume presence
    assert!(body(&ok("+/ near'.Silver\n")).contains("{AnoLeftSum 𝕩⊏silver}¨near"));
}

// Assignment: the guard refines the selection before the gather, so a false guard writes
// nothing.  The emit_effect probe stages the ∧-guard and the scatter mask reads it.
#[test]
fn guarded_assignment_refines_the_scatter_mask() {
    let text = ok("Burning , Gold = max/ Silver @ Path\n");
    assert!(text.contains("t1 ← s1m∧(0<(+´path))"), "{}", text);
    assert!(text.contains("t3 ← t1‿((+´t1)⥊t2) AnoScat gold"), "{}", text);
    // an unguarded right-hand side stages no probe conjunction at all
    let plain = ok("Burning , Gold = +/ Silver @ Path\n");
    assert!(!plain.contains("s1m∧"), "{}", plain);
}

// An empty unseeded scan is an empty column: length preservation does the work, so no instance
// stages a validity guard and none seeds an extra row.
#[test]
fn empty_scans_are_empty_columns_for_every_instance() {
    let cases = [
        ("+\\ Gold @ Path\n", "q1 ← (+`path/gold)"),
        ("*\\ Gold @ Path\n", "q1 ← (×`path/gold)"),
        ("min\\ Gold @ Path\n", "q1 ← (⌊`path/gold)"),
        ("max\\ Gold @ Path\n", "q1 ← (⌈`path/gold)"),
        ("&\\ Burning @ Path\n", "q1 ← (∧`path/burning)"),
        ("|\\ Burning @ Path\n", "q1 ← (∨`path/burning)"),
        ("#\\ Gold @ Path\n", "q1 ← (+`path/(¬(¬(1¨gold))))"),
        ("threat\\ Gold @ Path\n", "q1 ← (Fn_threat`path/gold)"),
    ];
    for (source, pin) in cases {
        let text = ok(source);
        assert!(text.contains(pin), "{}: {}", source, text);
        assert!(!text.contains("q1v"), "{}: {}", source, text);
        assert!(text.contains("\n•Show q1\n"), "{}: {}", source, text);
    }
    assert!(ok("avg\\ Gold @ Path\n").contains("q1 ← ((+`path/gold)÷(+`path/(¬(¬(1¨gold))))"));
    assert!(!ok("avg\\ Gold @ Path\n").contains("q1v"));
}

/* ---------- 4. refusals ---------- */

// One carrier gate for every form, one message family.  These spellings have no mask instance
// (todo/03:76-97), so fold, scan, and scan-along refuse alike.
#[test]
fn carrier_gate_is_one_message_family_across_forms() {
    let heads = [("max", "max"), ("min", "min"), ("avg", "avg"), ("threat", "threat")];
    for (spelling, named) in heads {
        let expected = format!("reducer '{}' is not defined on Mask", named);
        for source in [
            format!("{}/ Burning\n", spelling),
            format!("{}\\ Burning\n", spelling),
            format!("scan({}) Burning along Silver\n", spelling),
            format!("fold({}) Burning\n", spelling),
        ] {
            let message = err(&source);
            assert!(message.contains(&expected), "{}: {}", source.trim_end(), message);
        }
    }
    // the arithmetic reducers converge with them: `+\ mask` now refuses like `+/ mask`
    for source in ["+/ Burning\n", "+\\ Burning\n", "*/ Burning\n", "*\\ Burning\n"] {
        let message = err(source);
        assert!(message.contains("is not defined on Mask"), "{}: {}", source, message);
    }
}

// A name is admitted because the registry answers it with a fn, never because it is a name;
// the refusal is the same spelling in every form.
#[test]
fn unknown_reducer_refuses_identically_in_every_form() {
    for source in [
        "nope/ Gold\n",
        "nope\\ Gold\n",
        "fold(nope) Gold\n",
        "scan(nope) Gold\n",
        "scan(nope) Gold along Silver\n",
    ] {
        assert_eq!(err(source), "emit: line 1: unknown reducer 'nope'", "{}", source);
    }
}

// Greater/Lesser is carrier-directed with no coercion, so a mixture has no reading.  Only a
// genuine value position refuses: a bare column in selection position keeps presence semantics.
#[test]
fn mixed_direct_carriers_refuse() {
    assert!(err("Silver = (Gold | Burning)\n")
        .contains("'|' mixes mask and number operands; there is no carrier coercion"));
    assert!(err("Silver = (Gold & Burning)\n")
        .contains("'&' mixes mask and number operands; there is no carrier coercion"));
    assert!(ok("Burning & Gold , +Path\n").contains("burning∧"));
}

// Surfaces that were never added and one that was deleted.  `scan2` is an ordinary identifier
// now, so the refusal is the downstream generic parse error, not a keyword diagnostic.
#[test]
fn absent_surfaces_refuse() {
    let scan2 = err("scan2(+) Gold\n");
    assert!(scan2.contains("unexpected token"), "{}", scan2);
    assert!(!scan2.contains("scan2"), "{}", scan2);
    // `>` stays comparison; `>/` is not a reducer spelling
    assert!(err(">/ Gold\n").contains("unexpected token"));
    // no long count spelling: bare '#' keeps its lex refusal
    for source in ["fold(#) Gold\n", "scan(#) Gold\n", "scan(#) Gold along Silver\n"] {
        assert!(err(source).contains("'#' begins only '#/' or '#\\'"), "{}", source);
    }
    // a per-fiber scan needs a ragged result representation that does not exist yet
    assert!(err("+\\ near'.Gold\n").contains("scan over fibers is not yet supported"));
}

/* ---------- 5. properties against the descriptor semantics ---------- */

// A hand-rolled xorshift64: deterministic, dependency-free, and seeded per property.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    // A small-magnitude value grid: exact in float64, so the reference laws hold bit for bit.
    fn number(&mut self) -> f64 {
        ((self.next() % 33) as f64) - 16.0
    }

    // Division needs a divisor: 0÷0 is NaN, which is a data question and not a fold law.
    fn nonzero(&mut self) -> f64 {
        let v = ((self.next() % 31) as f64) - 15.0;
        if v == 0.0 { 1.0 } else { v }
    }

    fn bit(&mut self) -> f64 {
        (self.next() & 1) as f64
    }

    fn len(&mut self) -> usize {
        1 + (self.next() % 12) as usize
    }
}

// Inputs: a descriptor step glyph. Output: the Rust reference for that step.  This is the whole
// reference semantics: the descriptor names the operation, this names what it computes.
fn reference_step(glyph: &str) -> fn(f64, f64) -> f64 {
    match glyph {
        "+" => |a, b| a + b,
        "-" => |a, b| a - b,
        "×" => |a, b| a * b,
        "÷" => |a, b| a / b,
        "⌈" => |a, b| if b > a { b } else { a },
        "⌊" => |a, b| if b < a { b } else { a },
        "∧" => |a: f64, b: f64| ((a != 0.0) && (b != 0.0)) as i32 as f64,
        "∨" => |a: f64, b: f64| ((a != 0.0) || (b != 0.0)) as i32 as f64,
        other => panic!("no reference for step '{}'", other),
    }
}

// The unseeded left recurrence of todo/03:9-14.  a0 = x0, a(k+1) = f(ak, x(k+1)).
fn scan_reference(step: fn(f64, f64) -> f64, xs: &[f64]) -> Vec<f64> {
    let mut out = Vec::with_capacity(xs.len());
    let mut acc = 0.0;
    for (i, x) in xs.iter().enumerate() {
        acc = if i == 0 { *x } else { step(acc, *x) };
        out.push(acc);
    }
    out
}

fn fold_reference(step: fn(f64, f64) -> f64, xs: &[f64]) -> Option<f64> {
    scan_reference(step, xs).last().copied()
}

fn descriptor(spelling: &str, carrier: Carrier) -> OpDesc {
    reducer::resolve_head(spelling, Form::Fold, carrier, false).expect(spelling)
}

// fold f = last (scan f), and scan preserves length.  The two forms are one recurrence.
#[test]
fn fold_is_the_last_prefix_of_the_scan() {
    let mut rng = Rng(0x2545F4914F6CDD1D);
    let numeric = ["+", "*", "max", "min", "-", "/"];
    let masked = ["&", "|"];
    for _ in 0..100 {
        let n = rng.len();
        let xs: Vec<f64> = (0..n).map(|_| rng.number()).collect();
        let nz: Vec<f64> = (0..n).map(|_| rng.nonzero()).collect();
        let bits: Vec<f64> = (0..n).map(|_| rng.bit()).collect();
        for spelling in numeric {
            let desc = descriptor(spelling, Carrier::Number);
            let step = reference_step(desc.reducer().unwrap().step);
            let sample: &[f64] = if spelling == "/" { &nz } else { &xs };
            let scan = scan_reference(step, sample);
            assert_eq!(scan.len(), sample.len(), "{} preserves length", spelling);
            assert_eq!(fold_reference(step, sample), scan.last().copied(), "{}", spelling);
        }
        for spelling in masked {
            let desc = descriptor(spelling, Carrier::Mask);
            let step = reference_step(desc.reducer().unwrap().step);
            let scan = scan_reference(step, &bits);
            assert_eq!(scan.len(), bits.len(), "{} preserves length", spelling);
            assert_eq!(fold_reference(step, &bits), scan.last().copied(), "{}", spelling);
            assert!(scan.iter().all(|v| *v == 0.0 || *v == 1.0), "{} stays on the carrier", spelling);
        }
    }
}

// The declared identities are the ones the emitter seeds (AnoLeftSum and friends), so seeding a
// nonempty fold must not move its answer.  A wrong identity would corrupt nonempty folds too.
#[test]
fn declared_identities_are_neutral_for_the_seeded_lowering() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let cases =
        [("+", Carrier::Number), ("*", Carrier::Number), ("&", Carrier::Mask), ("|", Carrier::Mask)];
    for _ in 0..100 {
        let n = rng.len();
        for (spelling, carrier) in cases {
            let desc = descriptor(spelling, carrier);
            let reducer = desc.reducer().unwrap();
            let step = reference_step(reducer.step);
            let identity: f64 =
                reducer.identity.expect(spelling).bqn.parse().expect("identity literal");
            let xs: Vec<f64> = (0..n)
                .map(|_| if carrier == Carrier::Mask { rng.bit() } else { rng.number() })
                .collect();
            let mut seeded = vec![identity];
            seeded.extend_from_slice(&xs);
            assert_eq!(fold_reference(step, &seeded), fold_reference(step, &xs), "{}", spelling);
            // and on empty input the seeded fold IS the declared empty result
            assert_eq!(fold_reference(step, &[identity]), Some(identity), "{}", spelling);
        }
    }
    // the extrema declare none, so nothing may be seeded for them
    assert!(descriptor("max", Carrier::Number).reducer().unwrap().identity.is_none());
    assert!(descriptor("min", Carrier::Number).reducer().unwrap().identity.is_none());
}

// The count machine: state starts at 0, advances by 1 for each admitted row, emits per prefix.
fn count_machine(mask: &[f64]) -> Vec<f64> {
    let mut state = 0.0;
    mask.iter()
        .map(|m| {
            if *m != 0.0 {
                state += 1.0;
            }
            state
        })
        .collect()
}

#[test]
fn count_machine_emits_prefix_cardinalities() {
    // the charter's own example (todo/03:58)
    assert_eq!(count_machine(&[1.0, 0.0, 1.0, 1.0]), vec![1.0, 1.0, 2.0, 3.0]);
    // an implicit all-true membership stream yields 1..n
    let all_true = vec![1.0; 9];
    assert_eq!(
        count_machine(&all_true),
        (1..=9).map(|k| k as f64).collect::<Vec<f64>>()
    );
    assert!(count_machine(&[]).is_empty());

    let mut rng = Rng(0xD1B54A32D192ED03);
    for _ in 0..100 {
        let n = rng.len();
        let mask: Vec<f64> = (0..n).map(|_| rng.bit()).collect();
        let emits = count_machine(&mask);
        assert_eq!(emits.len(), mask.len());
        let finish: f64 = mask.iter().filter(|m| **m != 0.0).count() as f64;
        assert_eq!(*emits.last().unwrap(), finish, "last emit is the finish");
        // every prefix is the cardinality of that prefix, and the state never decreases
        for k in 0..n {
            let expected = mask[..=k].iter().filter(|m| **m != 0.0).count() as f64;
            assert_eq!(emits[k], expected);
            if k > 0 {
                assert!(emits[k] >= emits[k - 1]);
            }
        }
    }
    // the registered empty-fold law is 0, and it belongs to the machine, not to a binary step
    let desc = reducer::resolve_head("#", Form::Fold, Carrier::Presence, false).unwrap();
    assert_eq!(desc.empty_identity(), Some("0"));
    assert!(desc.fold_glyph().is_none());
}

// The average machine: state (sum,count), projected sum/count at each prefix.
fn average_machine(xs: &[f64]) -> Vec<f64> {
    let mut sum = 0.0;
    let mut count = 0.0;
    xs.iter()
        .map(|x| {
            sum += *x;
            count += 1.0;
            sum / count
        })
        .collect()
}

#[test]
fn average_machine_finishes_on_its_last_prefix() {
    let mut rng = Rng(0xA24BAED4963EE407);
    for _ in 0..100 {
        let n = rng.len();
        let xs: Vec<f64> = (0..n).map(|_| rng.number()).collect();
        let emits = average_machine(&xs);
        assert_eq!(emits.len(), xs.len());
        let finish = xs.iter().sum::<f64>() / xs.len() as f64;
        assert_eq!(*emits.last().unwrap(), finish, "last emit is the finish");
    }
    assert!(average_machine(&[]).is_empty());
    // mean has no result row on empty input; count's 0 is a law, not an identity mean shares
    let desc = reducer::resolve_head("avg", Form::Fold, Carrier::Number, false).unwrap();
    assert_eq!(desc.empty_identity(), None);
}

// Bridge equality at the descriptor layer AND behaviorally: one object, one reference step.
#[test]
fn bridge_descriptors_are_one_object() {
    let mut rng = Rng(0x14057B7EF767814F);
    for form in [Form::Fold, Form::Scan, Form::ScanAlong] {
        let greater = reducer::resolve_head("|", form, Carrier::Number, false).unwrap();
        let maximum = reducer::resolve_head("max", form, Carrier::Number, false).unwrap();
        assert_eq!(greater, maximum);
        let lesser = reducer::resolve_head("&", form, Carrier::Number, false).unwrap();
        let minimum = reducer::resolve_head("min", form, Carrier::Number, false).unwrap();
        assert_eq!(lesser, minimum);
    }
    let greater = reference_step(descriptor("|", Carrier::Number).reducer().unwrap().step);
    let maximum = reference_step(descriptor("max", Carrier::Number).reducer().unwrap().step);
    for _ in 0..100 {
        let xs: Vec<f64> = (0..rng.len()).map(|_| rng.number()).collect();
        assert_eq!(fold_reference(greater, &xs), fold_reference(maximum, &xs));
        assert_eq!(scan_reference(greater, &xs), scan_reference(maximum, &xs));
    }
}

/* ---------- 6. Nihongo parity ---------- */

// Every Japanese fold and scan word is its ASCII twin, over both carriers where the instance
// exists.  Byte-identical emission is the assertion: the surfaces differ, the operation does not.
#[test]
fn nihongo_words_emit_their_ascii_twins() {
    let numeric = [
        ("総和 金\n", "+/ Gold\n"),
        ("総積 金\n", "*/ Gold\n"),
        ("総数 金\n", "#/ Gold\n"),
        ("最大 金\n", "max/ Gold\n"),
        ("最小 金\n", "min/ Gold\n"),
        ("平均 金\n", "avg/ Gold\n"),
        ("皆 金\n", "&/ Gold\n"),
        ("或 金\n", "|/ Gold\n"),
        ("累和 金\n", "+\\ Gold\n"),
        ("累積 金\n", "*\\ Gold\n"),
        ("累数 金\n", "#\\ Gold\n"),
        ("累小 金\n", "min\\ Gold\n"),
        ("累平均 金\n", "avg\\ Gold\n"),
        ("累大 金\n", "max\\ Gold\n"),
        ("累皆 金\n", "&\\ Gold\n"),
        ("累或 金\n", "|\\ Gold\n"),
    ];
    assert_eq!(numeric.len(), 16, "every word in the fold/scan block of JATAB");
    for (ja, ascii) in numeric {
        assert_eq!(ok_ja(ja), ok(ascii), "{} vs {}", ja.trim_end(), ascii.trim_end());
    }
    // the mask carrier, for the instances that have one
    let masked = [
        ("総数 燃\n", "#/ Burning\n"),
        ("皆 燃\n", "&/ Burning\n"),
        ("或 燃\n", "|/ Burning\n"),
        ("累数 燃\n", "#\\ Burning\n"),
        ("累皆 燃\n", "&\\ Burning\n"),
        ("累或 燃\n", "|\\ Burning\n"),
    ];
    for (ja, ascii) in masked {
        assert_eq!(ok_ja(ja), ok(ascii), "{} vs {}", ja.trim_end(), ascii.trim_end());
    }
    // and the char carrier, which admits Greater, Lesser and their two bridges
    let runes = [
        ("最大 印\n", "max/ Rune\n"),
        ("最小 印\n", "min/ Rune\n"),
        ("或 印\n", "|/ Rune\n"),
        ("皆 印\n", "&/ Rune\n"),
        ("累大 印\n", "max\\ Rune\n"),
        ("累小 印\n", "min\\ Rune\n"),
        ("累或 印\n", "|\\ Rune\n"),
        ("累皆 印\n", "&\\ Rune\n"),
        ("印 か 印\n", "Rune | Rune\n"),
        ("印 と 印\n", "Rune & Rune\n"),
    ];
    for (ja, ascii) in runes {
        assert_eq!(ok_ja(ja), ok(ascii), "{} vs {}", ja.trim_end(), ascii.trim_end());
    }
}

// Parity is refusal parity too: the same carrier gate, spelled identically on both surfaces.
#[test]
fn nihongo_refusals_match_their_ascii_twins() {
    let pairs = [
        ("最大 燃\n", "max/ Burning\n"),
        ("最小 燃\n", "min/ Burning\n"),
        ("平均 燃\n", "avg/ Burning\n"),
        ("累大 燃\n", "max\\ Burning\n"),
        ("累小 燃\n", "min\\ Burning\n"),
        ("累平均 燃\n", "avg\\ Burning\n"),
        ("総和 燃\n", "+/ Burning\n"),
        ("総積 燃\n", "*/ Burning\n"),
        ("累和 燃\n", "+\\ Burning\n"),
        ("累積 燃\n", "*\\ Burning\n"),
    ];
    for (ja, ascii) in pairs {
        assert_eq!(err_ja(ja), err(ascii), "{} vs {}", ja.trim_end(), ascii.trim_end());
    }
    assert_eq!(err_ja("累大 燃\n"), "emit: line 1: reducer 'max' is not defined on Mask");
    // the char gate and the carrier mixture refuse identically on both surfaces
    for (ja, ascii) in
        [("総和 印\n", "+/ Rune\n"), ("平均 印\n", "avg/ Rune\n"), ("印 か 金\n", "Rune | Gold\n")]
    {
        assert_eq!(err_ja(ja), err(ascii), "{} vs {}", ja.trim_end(), ascii.trim_end());
    }
    assert_eq!(err_ja("累平均 印\n"), "emit: line 1: reducer 'avg' is not defined on Char");
}
