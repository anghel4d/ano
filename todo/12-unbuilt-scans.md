# 12 — the ruled-but-unbuilt scans: the `min\` glyph, running mean, running count

Surfaced 2026-07-13 while writing `docs/ano-keywords.md`. The fold/scan permutation table (task 04, landed verbatim into `ano-language.md`'s Appendix and mirrored in `ano-manual.md`) promises a running value for every reducer, but the emitter builds only a subset, and the two scan spellings build different subsets. Task 04's own closing invariant — "the table must not promise what the code refuses" — is standing violated for three rows. This file is the reconcile: build the missing scans, or footnote the table. Needs the author's call on scope (which of the three, and the running-count spelling); the work items below are gated on it, the sub-questions are not to be resolved silently.

## What the emitter actually builds (verified against both trees)

Two spellings reach the emitter, and their op sets diverge:

- the glyph `f\` under an `@` scope — `N_SCANEXPR` / `emit_scan` — builds `+ * max & |` and named-reducer scans. Rust: `steel/src/emit.rs:1126-1141` (the glyph match, then the registry-fn fallback), refusal `unknown scan op '<op>'` at `:1137`. C oracle: `src/emit.c:653-654`, refusal at `:658`. Same five, both trees.
- the long `scan(f) X along order` — `N_SCANALONG` — builds `+ * max min`. Rust: `steel/src/emit.rs:1526-1532`, refusal `scan(<op>): no registered scan step` at `:1531`. C oracle: `src/emit.c:943-944`, refusal at `:945`. Same four, both trees.

Confirmed by running `target/release/steel --emit`:

- `min\ Height @ Ray` → `emit: line N: unknown scan op 'min'` (exit 2), but `scan(min) X along order` compiles. The running minimum exists, spelled long only.
- `avg\ …` → `emit: line N: unknown scan op 'avg'`. Absent in both spellings.
- `#\ …` → refused one stage earlier, at the lexer: `line N: '#' begins only the fold '#/'` (`steel/src/lex.rs:501-507`, `src/lex.c:265-267`). `#` forms only the fold `#/`, so the running count has no spelling at all.

The fold side is whole: `min/ avg/ #/` all emit and are exercised by the corpus. The gap is scans only.

## The gap, exactly

- `min\` — the glyph is missing though the semantics are built. `scan(min) … along` already emits `⌊\``; the glyph table (`emit_scan`) just has no `min` arm. Cheapest of the three.
- `avg\` — running mean, absent in both spellings. The `avg/` fold already carries the sum-and-count-then-divide (`AnoAvg`, `steel/src/emit.rs` ~`:951`/`:1089`); the scan needs the running form of the same.
- running count — absent, and unspellable: `#\` is lexer-barred. Needs a spelling decision before it can be built.

## Work items (gated on the scope ruling)

- Add the `min\` glyph: one arm in `emit_scan`'s glyph table (`⌊\``), mirrored in `src/emit.c:653`. The along-form proves the codegen; this only opens the second spelling.
- Build `avg\`: the running mean over the scan's scope, off the same sum-and-count `avg/` uses. Decide empty-scope behavior — a scan is length-preserving, so an empty scope is the empty column (no identity consulted), consistent with the named-reducer scan note already in `emit_scan`.
- Build the running count once it has a spelling (see sub-questions).
- Witnesses: `.bqn` twins (`⌊\`, and the mean/count equivalents), pinned `--! out`, nihongo twins, registry fixtures, slotted into the fold-scan demo series (task 03 numbering). Gate on the differential battery: `src/check-ano.sh` byte-compares Rust emit against the C oracle, so the C arms must land in lockstep with the Rust.
- Reconcile the docs. `docs/ano-keywords.md` was corrected this session to state the real behavior (the two spellings, the exact refusals). `ano-language.md`'s Appendix table and `ano-manual.md`'s folds chapter still promise the unbuilt rows unqualified — either they gain the same footnote, or the ops land and the promise becomes true.

## Open sub-questions (surface at execution, do not resolve silently)

- The running-count spelling. `#\` is lexer-barred because `#` is fold-only. Options: (a) relex `#\` as the running-count scan, freeing the glyph on the scan side only; (b) a name — a registered `count` reducer, so `count\` scans and `count/` folds, retiring the `#` special-case pun; (c) leave it unspellable and strike the row from the table. Ties into 00-open-rulings Q13 (fold/scan grammar parity) and the max ruling's named-reducer unification (task 04).
- Scope: are all three wanted, or does `scan(min) … along` already suffice for min, leaving only `avg\`? The `min\` glyph is nearly free but adds a second spelling for one op — a grammar-surface choice, not just a codegen one.
- The glyph-vs-along asymmetry itself. `& |` build under the glyph but not `scan(&)…along`; `min` builds under along but not the glyph. Should the two spellings converge on one op set, or is the split intended (latches are glyph-native, min is order-native)? A prior question the build should not silently answer.

## Invariants

- Both surfaces move together: every Rust arm has its C-oracle twin, emit stays byte-identical across the corpus, dump fixpoints hold. The battery is the gate.
- The table and the emitter agree after landing: no row promises what the code refuses, and no spelling the code accepts is undocumented. Task 04's invariant, finally made true.
- No fold-side change: `min/ avg/ #/` already emit; this task is scans only.
- Empty-scope law preserved: a scan is length-preserving, so the empty scope yields the empty column — no identity, no NaN, consistent with the existing named-reducer scan path.
