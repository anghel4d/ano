# src/trace — the relationship-diagnostic fixture set

Small worlds, one crossing class each, read by `src/check-trace.sh`. Every fixture pins its post-state with `--! expect` so the parity leg can prove that `--trace` changed no value, only the 0x1F stream beside it.

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
