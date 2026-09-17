# src/trace — the relationship-diagnostic fixture set

Small worlds, one crossing class each, read by `src/check-trace.sh`. Every fixture pins its post-state with `--! expect` so the parity leg can prove that `--trace` changed no value, only the 0x1F stream beside it.

The machine grammar is deliberately presentation-free:

| record | semantic phase | domain | rule |
|---|---|---|---|
| `RELATION c origin -> sink IS DEAD ! USE id PREDICATE SOURCE` | predicate | source `X` | every evaluated valid non-`-1` target that fails foundness |
| `RELATION c origin -> sink IS DEAD ! USE id EFFECT SELECTED` | effect | selected `S` | the same test only after selection restriction |
| `FIBER c origin IS EMPTY ! USE id PREDICATE SOURCE` | predicate | source `X` | empty identityless fiber use in a predicate |
| `FIBER c origin IS EMPTY ! USE id EFFECT SELECTED` | effect | selected `S` | empty identityless fiber use in an effect |
| `TRACE-USE id site:line phase domain c` | either | as declared | stable source-use declaration, one per crossing |

`-1` is the sole silent functional no-link sentinel. Any other carrier-valid target is ordinary data and reports when dead; carrier-invalid storage refuses before execution. Events are ordered by statement, textual use, then stable source identity. Distinct crossings stay distinct. Steel prefixes each line with 0x1F; Kore removes only that byte and places the exact remainder in history. Query tags use 0x1D, saved world records use 0x1E, and no trace line may enter either channel. Presentation may aggregate only downstream, must expose a count, and may change neither exit status nor world state; Kore currently preserves the unaggregated machine stream.

The naming is `<case>.ano` beside `<case>-world.reg`, matching the `src/refusals/` convention.

| fixture | what it witnesses |
|---|---|
| `silent` | `-1` in a predicate, an effect and a query: zero `RELATION` lines |
| `dead-key` | never-existed keys: predicate reports over `X`, effect over `S` |
| `despawned` | a formerly-live key: silent before the despawn, `DEAD` after it |
| `unkeyed-oob` | an in-carrier row index past the world: bounded gather, row drops, `DEAD` |
| `excluded-effect` | the dead link sits on a `p`-excluded row and is crossed only in an effect: no event |
| `excluded-predicate` | the same world crossed in the predicate instead: the event appears |
| `fiber` | a dead fiber member scoped to `S`, `FIBER ... IS EMPTY`, and a gamma in effect position |
| `rules` | two standing rules crossing one relationship in one tick |
| `parity` | hop, fiber, gamma, spawn, rule and query in one program — the parity and determinism subject |

Fixture line numbers are load-bearing: `check-trace.sh` compares whole `TRACE-USE` blocks byte-exactly, and a record carries `<site>:<line>`. Editing a fixture means re-pinning its expected block in the script.
