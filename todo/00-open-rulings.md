# 00 — open rulings: the residue, Q/A

The answered rulings from the 2026-07-11 pass are folded into their consuming task files (A1/A2/A3 → 02, A5/A8 → 03, A7 → 04, A6 → 05). What remains here is deliberately deferred.

Q4. The `^` seam. Demo 14 pins both `!Whiterun` and `!^Whiterun` for one registered bind. (a) `^` reserved to host-resolved deictics, binds always bare: matches the grammar appendix, demo 14 loses one spelling. (b) `^` as optional constant-reference sugar on any registered constant: both spellings legal forever, the appendix paragraph rewritten. → 05.
A4. Defer decision. Keep this 00-open-rulings item here.

Q9. Spun off A7's deferral: whether to adopt k's `|` as native max — earning `|/` as an honest max-fold — and rename boolean OR to `||` to free the glyph. The author notes he is not aware of any language where OR is bare `|`. → 04, if ever ruled.
A9. Deferred (2026-07-11).

Q10. Surfaced by 02's landing: what does a bare rel in mask position mean for a KEYED rel? Today's `0≤rel` reads declaredness (a target was assigned); the landed keyed hop reads found-ness (the target is alive), so a keyed rel with a dead key now holds the mask while every hop through it fails. The two readings coincide for idx-keyed worlds before any despawn. (a) mask = declaredness, hop = found-ness (cheap, but the mask can select rows the hop then drops); (b) mask = found-ness everywhere (one meaning, costs an index-of per bare-rel mask). Landed as (b) for keyed rels; needs the ruling.
A10. What?

Q11. Surfaced by 02's landing: negative values in a `unique` column. When the column keys a rel, the loader refuses them (¯1 must stay the unambiguous dangling sentinel). Should `unique` itself refuse negatives even when nothing keys through it, so a later `rel <key> <name>` can never be invalidated by standing data? Cheap either way; a load-order question more than a semantics one.
A11. No. Unique means unique. It does not refuse negatives. It can stack with unsigned. unique is its own type attribute.

Q12. Surfaced by 05's landing: the §12 fail-the-row law is unobservable at bare-query level. `threat/ Damage @ none` (likewise `max/`, `avg/`) as a bare query prints 0 — emitQuery drops the row-failure guard, since a query has no row to fail. Should a no-identity fold over the empty scope print nothing, print a none marker, or fail the program? The assignment path already behaves (the row drops).
A12. What? Wtf are you saying? Speak fucking normally.

Q13. Surfaced by 05's landing: `fold(threat)` takes reducer names only while `scan(+)` also takes operator spellings — exact parity with the old `reduce(`, but under "one grammar row" should `fold(+)` be admitted? Cosmetic; `+/` already spells it.
A13. Bring to parity with scan().

Q14. Surfaced by 08's spatial audit: the pair-order straddle. `to h w` pours pairs ⟨row, col⟩ (axis-0 first), spawn's convention writes (col, row) = (x, y), and kore reads pos[0] as x — so 24-reshape-positions-b's 4×16 formation renders transposed, invisible on every square fixture. 27's own registry comment ("x is the ROW coordinate") shows the corpus already straddles it. Which pair order is the contract, and who converts?
A14. Explain what you mean by this.

Q15. Surfaced by 08's audit: is pos integer-by-contract? kore paints an entity at truncated (int)x but edit-targets by exact double equality, so a fractional pos is visible yet never editable. The corpus is integral today.
A15. No, positions are not integers only.

Q16. Surfaced by 10's landing: RELATION-vs-FIBER scoping. The FIBER report scopes to the statement's selection; the RELATION report does not — a row excluded by a sibling predicate conjunct still reports its dead crossing, and a hop spelled in both predicate and effect reports the same link twice (one line per crossing). Defensible under the columnar model (the hop genuinely crosses the whole column); should RELATION scope like FIBER?
A16. Please elaborate on this in plain english.

Q17. Surfaced by 10's landing: `-1` stays silent while a set-but-never-existed key screams DEAD. Licensed by the task wording ("a link whose target no longer exists" — -1 is no link at all), and 02's guard treats both alike semantically ("not-found is dangling is dead"). Should the diagnostic distinguish never-linked from died, or is the current split (sentinel silent, everything else loud) the contract?
A17. What?
