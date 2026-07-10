# 04 — fold/scan formalization: the permutation table, `&\` and `|\` demos, the max ruling

Ruled (author, 2026-07-11): the permutation table goes into the documentation or the grammar, verbatim. `&\` and `|\` "most definitely do deserve demos". One full table shows all of `+/ */ …` AND `+\ *\ …` together.

## The table (land verbatim; extend as ops land)

There is no grammar.md. The grammar lives in ano-language.md's Appendix. Put the table there (a new "fold and scan permutations" subsection beside Precedence) and mirror it in ano-manual.md's folds chapter. Lineage note to carry: in k, `&` IS min and `|` IS max over numerics. ano's boolean reading is the k reading restricted to masks.

| f | `f/` fold | `f\` scan | empty-scope identity |
|---|---|---|---|
| `+` | sum | running sum | 0 |
| `*` | product | running product | 1 |
| `&` | ALL | still-all: a latch that trips off at the first false and stays off | 1 (vacuous truth) |
| `\|` | ANY | ever-any: a latch that trips on at the first true and stays on | 0 |
| `#` | count | running count | 0 |
| `max` | maximum | running peak (occlusion, high-water) | none → row drops |
| `min` | minimum | running floor | none → row drops |
| `avg` | fold-and-finish mean | running mean | none → row drops |
| `-` | rejected: not associative | — | — |
| `/` (divide) | rejected: not associative; `//` additionally unlexable (`/` is fold-marker and replicate) | — | — |

The identity column restates the §12/§13 law: a fold with a registered identity yields it on the empty scope, and a reducer without one fails the row. That is left-join-null extended to the empty fiber. The γ column-form (`f/ rel'.Comp`) inherits the same identities per fiber.

## New demos

- `|\` — ever-any: e.g. `|\ Burning @ path`, "has the fire reached each point yet". One latch, flavor over a route or sightline.
- `&\` — still-all: e.g. `&\ Alive @ marchOrder`, "the column is intact up to here".
- Both with .bqn witnesses (`∨\`, `∧\`), pinned outs, nihongo twins, registry fixtures. Slot them into the fold-scan series under task 03's numbering.

## The max ruling (recorded verdict)

`max/` and `max\` stay named forms. The `>/` idea was examined and rejected: `>` is a comparison returning bool, folding it is non-associative nonsense, and making `>/` mean max requires reinterpreting `>` as "the greater-of" in fold position, the same pun refused for `-/` (APL's alternating sum). k earns `|/` as max-fold because k's `|` IS max natively, not a fold-position reinterpretation. Under task 05's unification, max/min/avg become registered reducers like any other name, so no glyph is needed. Author may overrule. Record the verdict in the table's notes either way.

## Fibonacci, for the record

Across ticks the recurrence is already expressible: the tick loop is the scan, and the s19 stencil twins iterate it (one shift-add per barrier). The within-statement recurrence stays the spec's open question (host fn canonical; non-associative `scan(f) along` and the generator subclause are the listed options). Task 03 carries the demo-comment framing.

## Invariants

- Docs only plus new demos: no emitter changes in this task. Both suites green. The table matches what the emitter actually registers: verify the identity claims against emit.c's fold registrations before landing. The table must not promise what the code refuses.
