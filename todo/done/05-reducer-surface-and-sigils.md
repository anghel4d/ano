# DONE

# 05 — the reducer surface and the sigils: `f/` universalized, `fold()`, `!` vs `^`

Two rulings and one seam, bundled by the author (2026-07-11).

## Ruling: `name/` is the universal reduce spelling

`/` attached to an operator OR a registered reducer name is the reduce op: `threat/ Damage @ Enemies` beside `+/ Gold @ Nord`, one grammar row (precedence level 9 already reads `f/ f\`; let f range over registered fn names). Scans come free: `threat\`. Collision safety is already enforced: the registry refuses fn/col name collisions under the case fold (verified live: `'gold' collides with entry 'gold' under the case fold`, exit 2, the GHC multiple-declarations behavior, ruled good), and a fn name in value position is already its own compile error.

## Ruling: `fold(f)` is the long form

`reduce(f)` becomes `fold(f)`. Fold is the word Haskell, APL culture, and C# LINQ all share, and the parentheses leave room for more inside. Documentation states plainly that this is a fold-reduction. Ruled (author, 2026-07-11): clean cut, `fold()` only, no deprecation cycle for `reduce(`. Demo 15 and the manual update accordingly.

## Implementation notes

Lexer/parser: `name/` and `name\` at fold-prefix level, name resolved against registry fns (and the built-in fold-and-finish set: max, min, avg, #). The §12 contract applies unchanged: associative dyadic, registered identity or fail the empty scope. Demo 15 gains the `threat/` spelling beside the long form (the corpus tradition: two spellings of one meaning forced to agree in public). Spec: §12's "or, spelled for named reducers" block and the Appendix precedence row.

## The `!` vs `^` tabulation (ruled: tabulate; the seam needs a ruling)

| sigil | is | precedence | binds to | resolved |
|---|---|---|---|---|
| `!` | mask NOT, a prefix operator | level 7 | one mask term | at evaluation, pointwise |
| `^` | not an operator: lexically part of the identifier; the alias/deixis sigil (`^cursor`, `^observer`, `^world`), こそあど | atom, level 14 | the name it is glued to | re-resolved per evaluation via the host |

They never compete: `!` negates a mask, `^` selects the dynamic alias overlay. The author ruled the old seam on 2026-07-21. `^Whiterun` reads the live alias named `Whiterun` when one exists and otherwise falls through to bare `Whiterun`. Rebinding or deleting that alias never changes the bare registered column or binding. `!^cursor` is "not the thing under the cursor": the sigils compose, they do not overlap. Steel still treats the sigil as direct registry lookup; the pending overlay, lifecycle, and tests live in `todo/16-dynamic-alias-overlay.md`.

## Invariants

- Both suites green. Emitted BQN for untouched demos byte-identical.
- The fold contract (§12) unchanged: this task changes spelling, never semantics.
- Registry collision behavior unchanged (names fold, values exact).
