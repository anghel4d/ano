# 00 — open rulings: the residue, Q/A

This file is the decision ledger. Entries remain after they are answered so later implementation work can recover the question, the tradeoff, the ruling, and whether code still owes anything. The author may clear closed entries manually.

The answered rulings from the 2026-07-11 pass are folded into their consuming task files (A1/A2/A3 → 02, A5/A8 → 03, A7 → 04, A6 → 05). Every later ruling below is now resolved; entries whose code still owes the contract say so explicitly.

Q4. The `^` seam. Demo 14 pins both `!Whiterun` and `!^Whiterun` for one registered bind. (a) `^` reserved to host-resolved deictics, binds always bare: matches the grammar appendix, demo 14 loses one spelling. (b) `^` as optional constant-reference sugar on any registered constant: both spellings legal forever, the appendix paragraph rewritten. → 05.
A4. Defer decision. Keep this 00-open-rulings item here.

Resolution (2026-07-21). `!` is mask negation. `^` selects a dynamic alias. `^name` resolves the live alias first and falls through to bare `name` when no alias exists. Installing, rebinding, or deleting `^Whiterun` never mutates the bare `Whiterun` column or binding.

Implementation. Steel and Kore do not yet implement the live overlay, lifecycle, and fallback. The pending work is `todo/16-dynamic-alias-overlay.md`.

Q9. Spun off A7's deferral: whether to adopt k's `|` as native max — earning `|/` as an honest max-fold — and rename boolean OR to `||` to free the glyph. The author notes he is not aware of any language where OR is bare `|`. → 04, if ever ruled.
A9. Deferred (2026-07-11).

Resolution (2026-07-21). Ano's Greater/Lesser family is exactly q/kdb+'s convention over the carriers that Ano admits; this is not a claim of general q compatibility. `|` is OR on masks and maximum on numbers. `&` is AND on masks and minimum on numbers. Their folds and scans are the corresponding reductions and scans. Boolean OR remains `|`; there is no `||`. `max/`, `max\`, `min/`, and `min\` remain numeric bridges. See KX's definitions of [Greater](https://code.kx.com/q/ref/greater/), [Lesser](https://code.kx.com/q/ref/lesser/), [max](https://code.kx.com/q/ref/max/), and [min](https://code.kx.com/q/ref/min/).

Implementation. Steel and Kore still treat `|` and `&` as mask-only operations. Carrier-directed direct, fold, and scan behavior and its tests are pending in `todo/17-greater-lesser.md`.

Q10. Surfaced by 02's landing: what does a bare rel in mask position mean for a KEYED rel? Today's `0≤rel` reads declaredness (a target was assigned); the landed keyed hop reads found-ness (the target is alive), so a keyed rel with a dead key now holds the mask while every hop through it fails. The two readings coincide for idx-keyed worlds before any despawn. (a) mask = declaredness, hop = found-ness (cheap, but the mask can select rows the hop then drops); (b) mask = found-ness everywhere (one meaning, costs an index-of per bare-rel mask). Landed as (b) for keyed rels; needs the ruling.
A10. What?

Q10 restated (2026-07-19). A rel column is used two ways: as a mask in a predicate (`Owner &` — "does this row have an owner?") and as a hop (`Owner.Gold` — "follow to the owner"). After a despawn these can disagree on a keyed rel: the row still holds an owner key, but the owner is dead. Should the bare mask mean "an owner was assigned" (cheap, but the predicate can select rows whose hop then finds nobody) or "the owner is alive right now" (one meaning everywhere, costs one lookup per bare-rel mask)? The code currently does the second. Rule: keep it, or switch?

Resolution (2026-07-21). Keep foundness. A bare relationship is true exactly when its stored target resolves in the current world. It uses the same found-guard as a hop, so a dead target cannot satisfy bare `Owner` while failing `Owner.Gold`.

Implementation. Steel already implements foundness. No core implementation work remains; the ruling is now explicit in the spec and manual.

Q11. Surfaced by 02's landing: negative values in a `unique` column. When the column keys a rel, the loader refuses them (¯1 must stay the unambiguous dangling sentinel). Should `unique` itself refuse negatives even when nothing keys through it, so a later `rel <key> <name>` can never be invalidated by standing data? Cheap either way; a load-order question more than a semantics one.
A11. No. Unique means unique. It does not refuse negatives. It can stack with unsigned. unique is its own type attribute.

Q11 executed (verified 2026-07-19). Commit 170667d landed the ruling: `unique` takes a kind word (`unique id nat` is injectivity ∩ ℕ, `unique slot int` keys negatives), `steel/src/registry.rs:365`. Closed.

Resolution restated (2026-07-21). `unique` means injective and nothing else. A carrier such as `nat` supplies nonnegativity independently.

Q12. Surfaced by 05's landing: the §12 fail-the-row law is unobservable at bare-query level. `threat/ Damage @ none` (likewise `max/`, `avg/`) as a bare query prints 0 — emitQuery drops the row-failure guard, since a query has no row to fail. Should a no-identity fold over the empty scope print nothing, print a none marker, or fail the program? The assignment path already behaves (the row drops).
A12. What? Wtf are you saying? Speak fucking normally.

Q12 restated (2026-07-19). Some folds have a natural answer for "nothing selected": `+/` over an empty selection is 0, honestly. Others don't: `max/`, `avg/`, and named reducers like `threat/` have no honest answer for an empty selection. When such a fold feeds an assignment, the ruled law already handles it: the row silently drops. But as a bare query — just printing the number — there is no row to drop, and today it prints 0, a made-up value. Rule: should `threat/ Damage @ none` as a bare query print nothing, print a none marker, or refuse to run?

Resolution (2026-07-21). Maximum of nothing and every other identityless fold over nothing produce nothing: no result row, exactly like a predicate with no matches. An assignment writes nothing and a bare query prints nothing. No none marker, program error, NaN, infinity, or placeholder scalar is introduced. Identity-bearing folds retain their identities.

Implementation. Fold emission already carries the validity guard, but bare-query emission discards it and leaks a backend `0` placeholder. Steel/Kore implementation and tests are pending in `todo/18-empty-result-output.md`.

Q13. Surfaced by 05's landing: `fold(threat)` takes reducer names only while `scan(+)` also takes operator spellings — exact parity with the old `reduce(`, but under "one grammar row" should `fold(+)` be admitted? Cosmetic; `+/` already spells it.
A13. Bring to parity with scan().

Q13 ruled, not built (verified 2026-07-19). `fold(+)` is still refused — `steel/src/parse.rs:387` accepts reducer names only after `fold(`. The work rides with `12-unbuilt-scans.md`, which touches the same emitter pass.

Resolution expanded (2026-07-21). This is general callable-head parity, not a special case for `fold(+)`. `fold(f)`, `scan(f)`, and `scan2(f)` use the LINQ unseeded-accumulator model: `f` may be an operator or registered reducer and resolves through Ano's registry. On nonempty ordered input the first value starts the accumulator and the rest apply left-to-right; fold returns the final state and scan returns every successive state. Ano improves on LINQ because the registry proves the legal execution strategy: ordered fold admits any compatible registered accumulator; unordered regrouping requires associativity; parallel/unordered execution requires associativity and commutativity; empty fold uses the registered identity or yields nothing; scan returns every successive accumulator state; empty scan is empty and needs no identity unless a seeded form explicitly emits the seed. Ano has no seeded form today. Haskell supplies the fold/scan vocabulary, but LINQ is the closer operational precedent for registered dispatch.

Implementation. Steel's long-form parsers and scan whitelists remain inconsistent. General parser/emitter parity, the `min\` bridge, running mean, and the still-unruled running-count spelling live in `todo/12-unbuilt-scans.md`.

Q14. Surfaced by 08's spatial audit: the pair-order straddle. `to h w` pours pairs ⟨row, col⟩ (axis-0 first), spawn's convention writes (col, row) = (x, y), and kore reads pos[0] as x — so 24-reshape-positions-b's 4×16 formation renders transposed, invisible on every square fixture. 27's own registry comment ("x is the ROW coordinate") shows the corpus already straddles it. Which pair order is the contract, and who converts?
A14. Explain what you mean by this.

Q14 restated (2026-07-19). Two conventions collide over what a pair ⟨a,b⟩ means as a 2D position. The reshape pour `to h w` fills row-first, so it produces pairs whose FIRST number is the row (vertical). Spawn's convention and kore's map read the FIRST number as x (horizontal). Result: demo 24's 4×16 formation renders transposed — 16 wide where it should be 4 wide — and the bug is invisible on any square world. Demo 27's registry comment ("x is the ROW coordinate") shows the corpus already straddles both readings. Rule: is a pair (x, y) or (row, col)? And whichever wins, which layer converts the other — the pour, or the readers?

Resolution (2026-07-21). A raw pair has no universal spatial-axis interpretation. The destination's registry declaration supplies the position carrier and ordered spatial axes. A two-coordinate `to` result is valid whenever that carrier has at least two axes; no lattice placement is required. Kore and every other consumer must render and edit through the declared axes instead of assuming `pos[0]` is universally x or row.

Implementation. The ruling is recorded, but Steel/Kore spatial formalization and the view repair remain pending in `todo/12-spatial-formalization.md` and TODO.md's spatial-view item.

Q15. Surfaced by 08's audit: is pos integer-by-contract? kore paints an entity at truncated (int)x but edit-targets by exact double equality, so a fractional pos is visible yet never editable. The corpus is integral today.
A15. No, positions are not integers only.

Q15 answered (2026-07-19 note). Positions are fractional by contract; kore's paint-at-truncated-x but edit-by-exact-double mismatch is therefore a kore bug, consumed by the spatial-views task in TODO.md. Closed here.

Resolution restated (2026-07-21). Position coordinates may be fractional. Integer-only fixtures do not narrow the carrier contract.

Q16. Surfaced by 10's landing: RELATION-vs-FIBER scoping. The FIBER report scopes to the statement's selection; the RELATION report does not — a row excluded by a sibling predicate conjunct still reports its dead crossing, and a hop spelled in both predicate and effect reports the same link twice (one line per crossing). Defensible under the columnar model (the hop genuinely crosses the whole column); should RELATION scope like FIBER?
A16. Please elaborate on this in plain english.

Q16 restated (2026-07-19). The debug channel from task 10 emits two report kinds. FIBER reports respect the statement's selection: only rows the predicate kept get reported. RELATION reports do not: they scan the whole column. Two consequences: a row your predicate excluded still reports its dead link, and a hop spelled in both the predicate and the effect reports the same dead link twice. This is defensible — the hop really does cross the whole column — but it is noisy. Rule: should RELATION reports filter to the selected rows, the way FIBER already does?

Resolution (2026-07-21). Diagnostic scope follows semantic domain. Let `X` be the source domain, `p : X → Bool` the final predicate mask, `S = {x ∈ X | p(x)}`, and `i : S ↪ X` the inclusion. Predicate masks and their `RELATION` and `FIBER` diagnostics range over `X`; pointwise `&` never short-circuits one conjunct through another. Effect values restrict along `i` and effect diagnostics range over `S`. Distinct source crossings remain distinct trace events; presentation may aggregate them without changing the trace contract.

Implementation. Steel's `FIBER` path already follows this phase rule. Its `RELATION` hook masks only earlier relationship-leg guards and does not apply the statement selection in effect position, so effect-side reports still leak whole-column lowering. Restrict effect-side `RELATION` reports to `S`, retain whole-domain predicate reporting and per-crossing multiplicity, and add trace tests. Implementation pending.

Q17. Surfaced by 10's landing: `-1` stays silent while a set-but-never-existed key screams DEAD. Licensed by the task wording ("a link whose target no longer exists" — -1 is no link at all), and 02's guard treats both alike semantically ("not-found is dangling is dead"). Should the diagnostic distinguish never-linked from died, or is the current split (sentinel silent, everything else loud) the contract?
A17. What?

Q17 restated (2026-07-19). Same debug channel, one asymmetry: a rel holding `-1` (this row never had a link) stays silent; a rel holding a key whose target died reports DEAD. So "never had an owner" is quiet while "owner died" is loud. That split follows the task wording — `-1` is no link at all, not a broken one. Rule: is silent-sentinel/loud-corpse the contract, or should the diagnostic name both cases distinctly?

Resolution (2026-07-21). Keep it simple. `-1` means no link and stays silent. Every nonnegative target that fails the found-guard is a DEAD link. Never-existed, despawned, and mistyped are not separate diagnostic states, and MISSING or NOT FOUND add no useful distinction.

Implementation. Steel already implements the silent sentinel and the DEAD diagnostic. No core implementation work remains.
