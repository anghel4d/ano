# 00 — historical rulings: Q/A from the previous review round.

DO NOT, UNDER ANY CIRCUMSTANCES, ALTER ANY PART OF THIS FILE.

THIS IS A Q&A CONCERNING *FINISHED AND DONE RULINGS*. ALL A ITEMS ARE BY THE USER AND SHOULD BE CONSIDERED DEFINITIVE AND AUTHORITATIVE.

<<READONLY>>

This file is the decision ledger. Entries remain after they are answered so later implementation work can recover the question, the ruling, and whether code still owes anything. The author may clear closed entries manually.

The answered rulings from the 2026-07-11 pass are folded into their consuming task files (A1/A2/A3 → 02, A5/A8 → 03, A7 → 04, A6 → 05). Every later ruling below is resolved; answers whose code still owes the contract say so explicitly.

Q4. The `^` seam. Demo 14 pins both `!Whiterun` and `!^Whiterun` for one registered bind. (a) `^` reserved to host-resolved deictics, binds always bare: matches the grammar appendix, demo 14 loses one spelling. (b) `^` as optional constant-reference sugar on any registered constant: both spellings legal forever, the appendix paragraph rewritten. → 05.

A4. `!` means not: on a mask, it is mask negation. `^` names a dynamic alias whose target may change between statement steps. Bare `Whiterun` always refers to the `Whiterun` column or binding, and `!Whiterun` is its negation. `^Whiterun` resolves an installed alias first and otherwise falls through to bare `Whiterun`. Installing, rebinding, or deleting an alias named `Whiterun` changes only `^Whiterun`; it never destroys or mutates bare `Whiterun`, because Ano variables are column names while aliases have their own lifecycle. Steel and Kore still owe the live overlay, lifecycle, fallback, and tests in `todo/16-dynamic-alias-overlay.md`.

Q9. Spun off A7's deferral: whether to adopt k's `|` as native max — earning `|/` as an honest max-fold — and rename boolean OR to `||` to free the glyph. The author notes he is not aware of any language where OR is bare `|`. → 04, if ever ruled.

A9. Ano adopts q/kdb+'s Greater/Lesser convention over the carriers Ano admits; this is not a claim of general q compatibility. `|` is OR on masks and maximum on numbers. `&` is AND on masks and minimum on numbers. Their folds and scans are the corresponding reductions and running reductions: `|/` and `|\` are any/running-any on masks and maximum/running-maximum on numbers; `&/` and `&\` are all/running-all on masks and minimum/running-minimum on numbers. Boolean OR remains `|`; there is no `||`. `max/`, `max\`, `min/`, and `min\` remain numeric bridges. See KX's definitions of [Greater](https://code.kx.com/q/ref/greater/), [Lesser](https://code.kx.com/q/ref/lesser/), [max](https://code.kx.com/q/ref/max/), and [min](https://code.kx.com/q/ref/min/). Steel and Kore still owe carrier-directed dyads, folds, scans, and tests in `todo/17-greater-lesser.md`.

Q10. Surfaced by 02's landing: what does a bare rel in mask position mean for a KEYED rel? Today's `0≤rel` reads declaredness (a target was assigned); the landed keyed hop reads found-ness (the target is alive), so a keyed rel with a dead key now holds the mask while every hop through it fails. The two readings coincide for idx-keyed worlds before any despawn. (a) mask = declaredness, hop = found-ness (cheap, but the mask can select rows the hop then drops); (b) mask = found-ness everywhere (one meaning, costs an index-of per bare-rel mask). Landed as (b) for keyed rels; needs the ruling.

A10. Keep foundness. A bare relationship is true exactly when its stored target resolves in the current world. It uses the same found-guard as a hop, so a dead target cannot satisfy bare `Owner` while failing `Owner.Gold`. Steel already implements this; no core implementation work remains.

Q11. Surfaced by 02's landing: negative values in a `unique` column. When the column keys a rel, the loader refuses them (¯1 must stay the unambiguous dangling sentinel). Should `unique` itself refuse negatives even when nothing keys through it, so a later `rel <key> <name>` can never be invalidated by standing data? Cheap either way; a load-order question more than a semantics one.

A11. `unique` means injective and nothing else. It does not imply nonnegativity. A carrier such as `nat` supplies the nonnegative restriction independently, so `unique id nat` is injectivity over naturals while a unique integer column may contain negatives. Steel implements this; no core implementation work remains.

Q12. Surfaced by 05's landing: the §12 fail-the-row law is unobservable at bare-query level. `threat/ Damage @ none` (likewise `max/`, `avg/`) as a bare query prints 0 — emitQuery drops the row-failure guard, since a query has no row to fail. Should a no-identity fold over the empty scope print nothing, print a none marker, or fail the program? The assignment path already behaves (the row drops).

A12. An identityless fold over nothing produces nothing: no result row, exactly like a predicate with no matches. An assignment writes nothing and a bare query prints nothing. Ano introduces no none marker, error, NaN, infinity, or placeholder scalar. Identity-bearing folds retain their registered identities. Steel and Kore still owe the bare-query validity guard and tests in `todo/18-empty-result-output.md`.

Q13. Surfaced by 05's landing: `fold(threat)` takes reducer names only while `scan(+)` also takes operator spellings — exact parity with the old `reduce(`, but under "one grammar row" should `fold(+)` be admitted? Cosmetic; `+/` already spells it.

A13. `fold(f)` and `scan(f)` admit every semantically compatible arithmetic or fold/scan operator and every compatible registered reducer; `fold(+)` is valid. The long forms use the LINQ unseeded-accumulator model: on nonempty ordered input, the first value starts the accumulator and the rest apply left-to-right; fold returns the final state and scan returns every successive state. Ano improves on LINQ because the registry can prove which execution strategies are legal: ordered fold admits any compatible registered accumulator; unordered regrouping requires associativity; parallel or unordered execution requires associativity and commutativity; empty fold returns the registered identity or nothing when none exists; scan uses the same accumulator and returns every successive state; empty scan is an empty column and needs no identity unless a seeded form explicitly emits the seed. Haskell supplies the fold/scan vocabulary, while LINQ is the closer operational precedent for registered dispatch. Existing symbolic forms such as `|/`, `+\`, and every other correct fold or scan keep their established behavior; the long forms expose the same semantics rather than replacing them. Steel still owes general parser/emitter parity and the remaining ruled scans in `todo/12-unbuilt-scans.md`.

Q14. Surfaced by 08's spatial audit: the pair-order straddle. `to h w` pours pairs ⟨row, col⟩ (axis-0 first), spawn's convention writes (col, row) = (x, y), and kore reads pos[0] as x — so 24-reshape-positions-b's 4×16 formation renders transposed, invisible on every square fixture. 27's own registry comment ("x is the ROW coordinate") shows the corpus already straddles it. Which pair order is the contract, and who converts?

A14. A raw pair has no universal spatial-axis interpretation. Declaring the position carrier and its ordered spatial axes is a registry-side obligation. A two-coordinate `to` result is valid whenever the destination carrier has at least two spatial axes; it does not require a declared lattice placement. Kore and every other consumer must render and edit through the registry-declared axes instead of assuming `pos[0]` is universally x or row. Steel/Kore spatial formalization and the view repair remain pending in `todo/12-spatial-formalization.md` and the spatial-view TODO item.

Q15. Surfaced by 08's audit: is pos integer-by-contract? kore paints an entity at truncated (int)x but edit-targets by exact double equality, so a fractional pos is visible yet never editable. The corpus is integral today.

A15. Positions are not integer-only. Position coordinates may be fractional, and integral fixtures do not narrow that carrier contract. Kore's truncated-paint/exact-edit mismatch is a Kore bug covered by the spatial-view task; the language ruling itself is closed.

Q16. Surfaced by 10's landing: RELATION-vs-FIBER scoping. The FIBER report scopes to the statement's selection; the RELATION report does not — a row excluded by a sibling predicate conjunct still reports its dead crossing, and a hop spelled in both predicate and effect reports the same link twice (one line per crossing). Defensible under the columnar model (the hop genuinely crosses the whole column); should RELATION scope like FIBER?

A16. Diagnostic scope follows semantic domain. Let `X` be the source domain, `p : X → Bool` the final predicate mask, `S = {x ∈ X | p(x)}`, and `i : S ↪ X` the inclusion. Predicate expressions are columns on `X`. In `Enemy & Owner.Gold > 10`, both conjuncts are masks on `X`; `&` is pointwise and commutative, not sequential short-circuiting, so the `Owner` hop is evaluated over all of `X`. Predicate-side `RELATION` and `FIBER` reports therefore range over `X`. After the predicate, effects live on `S`: an effect-side `Owner.Power` is the corresponding column restricted along `i`, so effect-side `RELATION` and `FIBER` reports range over `S`. Distinct source crossings remain distinct trace events; spelling the same hop in predicate and effect may correctly produce two reports because they are two uses in different semantic positions. Presentation may aggregate them without changing the trace contract. Steel's `FIBER` path already follows this phase rule; effect-side `RELATION` still owes restriction to `S` and trace tests.

Q17. Surfaced by 10's landing: `-1` stays silent while a set-but-never-existed key screams DEAD. Licensed by the task wording ("a link whose target no longer exists" — -1 is no link at all), and 02's guard treats both alike semantically ("not-found is dangling is dead"). Should the diagnostic distinguish never-linked from died, or is the current split (sentinel silent, everything else loud) the contract?

A17. Keep it simple. `-1` means no link and stays silent. Any actual link whose target fails the found-guard is DEAD. MISSING and NOT FOUND add states that mean the same thing. Steel already implements the silent sentinel and DEAD diagnostic; no core implementation work remains.


<<READONLY>>