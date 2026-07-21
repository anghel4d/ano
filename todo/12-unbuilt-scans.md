# 12 — ruled fold and scan parity

Standing verified 2026-07-21. `emit_scan`'s glyph table still builds `+ * max & |` only (`steel/src/emit.rs:1126-1131`), the along-form still builds `+ * max min` (`:1530`), and `#\` stays lexer-barred. The long-form head in `fold(f)`, `scan(f)`, and `scan2(f)` is an accumulator operation. Every semantically admitted operator or registered reducer belongs there. This is general parity, not a special case for `fold(+)`. Steel's `fold(f)` parser still accepts names only while `scan(f)` and `scan2(f)` use a separate operator-or-name parser.

Haskell folds and scans and LINQ Aggregate establish the useful policy: the parenthesized argument denotes the accumulator function, not a privileged identifier class. Their sequential forms can apply arbitrary functions because traversal order is fixed. Ano keeps its stricter semantic license. An unordered fold still requires associativity, and unordered parallel reassociation still requires commutativity. Syntax admits the common callable head, then the semantic checker accepts or refuses the operation for that fold or scan.

The fold/scan permutation table promises a running value for every admitted reducer, but Steel builds only a subset and its spellings maintain different whitelists. The ruled work is to converge the grammar and emitter on one semantic operation set. Only the running-count spelling remains an open surface decision.

## What Steel actually builds

Two spellings reach the emitter, and their op sets diverge:

- the glyph `f\` under an `@` scope — `N_SCANEXPR` / `emit_scan` — builds `+ * max & |` and named-reducer scans. `steel/src/emit.rs:1126-1141` holds the glyph match, registry-function fallback, and `unknown scan op '<op>'` refusal.
- the long `scan(f) X along order` — `N_SCANALONG` — builds `+ * max min`. `steel/src/emit.rs:1526-1532` holds the whitelist and the `scan(<op>): no registered scan step` refusal.

Confirmed by running `target/release/steel --emit`:

- `min\ Height @ Ray` → `emit: line N: unknown scan op 'min'` (exit 2), but `scan(min) X along order` compiles. The running minimum exists, spelled long only.
- `avg\ …` → `emit: line N: unknown scan op 'avg'`. Absent in both spellings.
- `#\ …` → refused one stage earlier, at the lexer: `line N: '#' begins only the fold '#/'` (`steel/src/lex.rs:501-507`). `#` forms only the fold `#/`, so the running count has no spelling at all.

The glyph fold side is whole: `min/ avg/ #/` all emit and are exercised by the corpus. The remaining fold gap is the long-form parser's refusal of operator heads. The scan gaps are the missing rows and divergent whitelists above.

## The gap, exactly

- `min\` — the glyph is missing though the semantics are built. `scan(min) … along` already emits `⌊\``; the glyph table (`emit_scan`) just has no `min` arm. Cheapest of the three.
- `avg\` — running mean, absent in both spellings. The `avg/` fold already carries the sum-and-count-then-divide (`AnoAvg`, `steel/src/emit.rs` ~`:951`/`:1089`); the scan needs the running form of the same.
- running count — absent, and unspellable: `#\` is lexer-barred. Needs a spelling decision before it can be built.

## Work items

- Give `fold(f)`, `scan(f)`, and `scan2(f)` one operator-or-registered-reducer head parser. Apply semantic admission after parsing instead of maintaining syntax-specific whitelists. This admits `+`, `*`, `&`, `|`, the named bridges, and registered reducers wherever their fold or scan instance exists. It does not admit subtraction or division into unordered folds.
- Add the `min\` bridge to `emit_scan`'s glyph table. The along-form already proves the codegen.
- Build `avg\`, the running mean over the scan's scope, from the same sum-and-count state as `avg/`. Empty input yields the empty column without consulting an identity.
- Build the running count once it has a spelling (see the open sub-question).
- Add Ano and Nihongo acceptance and refusal tests for every long-form head. Add BQN explanatory witnesses for the emitted operations. Exercise the result through Steel and Kore.
- Reconcile the liveness notes after implementation so the table and emitter agree.

## Open sub-question

- The running-count spelling. `#\` is lexer-barred because `#` is fold-only. Option (a): relex `#\` as the running-count scan and free the glyph on the scan side only. Option (b): use a registered `count` reducer, so `count\` scans and `count/` folds, retiring the `#` special-case pun. Option (c): leave it unspellable and strike the row from the table. This ties into fold/scan grammar parity. The Greater/Lesser carrier overload is separate and lives in `17-greater-lesser.md`.

## Invariants

- Steel is the reference compiler. The archived C predecessor is not an oracle. BQN files explain and witness the denotation but do not constrain it independently.
- The table and emitter agree after landing: no row promises what Steel refuses, and no accepted spelling is undocumented.
- Long forms share one callable-head policy. The operation's registered laws and the fold or scan's order determine semantic admission.
- Empty-scope law preserved: a scan is length-preserving, so the empty scope yields the empty column — no identity, no NaN, consistent with the existing named-reducer scan path.
