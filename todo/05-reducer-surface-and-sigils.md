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

They never compete: `!` negates a mask, `^` names a thing. `!^cursor` is "not the thing under the cursor": the sigils compose, they do not overlap. The confusion traces to a real seam. Demo 14 pins BOTH `!Whiterun` and `!^Whiterun` for one registered `bind … mask`, and the emitter accepts RK_BIND and RK_ALIAS through either spelling. The grammar text says `^` is deixis; the demo uses it on a static constant. The ruling is deferred by the author (2026-07-11; it stays live in 00-open-rulings) — execute this task without resolving it, both spellings stay legal meanwhile. The options, for when it lands: (a) `^` reserved to host-resolved deictics, binds always bare (tighter, matches the grammar appendix, demo 14 loses one spelling); or (b) `^` as optional constant-reference sugar on any registered constant (looser, both spellings legal forever, the appendix paragraph rewritten). Whichever lands: one spelling per demo unless the demo exists to pin the equivalence.

## Invariants

- Both suites green. Emitted BQN for untouched demos byte-identical.
- The fold contract (§12) unchanged: this task changes spelling, never semantics.
- Registry collision behavior unchanged (names fold, values exact).
