# Tick checks

[src/check-tick.sh](../src/check-tick.sh) runs a finite number of ticks for the cases in [tick-classes.txt](../src/tick-classes.txt). It applies the initial program pins once and strips them for later ticks. The harness checks its declared finite horizon.

A repeated output is not necessarily a repeated world. An induction requires equality of the complete relevant state and a deterministic transition with the same inputs. A verified fixed point then stays fixed; a verified cycle repeats under those assumptions.

An additive or saturating example needs its own invariant, including numeric publication, allocation limits, live rules, aliases, and external input. A bounded successful run does not establish total execution for every future tick.

The optional `ANO_TICK_SATURATE=1` checks extend selected finite runs. They remain experiments, not unbounded proofs. Do not promote their observed horizon into a language guarantee.

General spatial tick preservation is conditional on the structures in [SpatialWorld.lean](Ano/SpatialWorld.lean). Current Steel/Kore behavior needs separate agreement tests.

## Manifest meanings

| Class | Finite check |
|---|---|
| `fixed K` | Saved registry hashes at ticks K and K−1 agree. For K ≥ 3, K−1 must differ from K−2. |
| `cycle A B` | Saved registry hashes at A and B agree, with distinct hashes inside A through B−1. |
| `evolving` | Ticks 1 through 4 succeed and have distinct saved registry hashes. |
| `saturate=K` | With the optional saturation check enabled, the evolving case additionally reaches the declared adjacent equality at K. |

The harness compares truncated SHA-256 hashes of serialized `world.reg`. It does not compare the full host state, all sidecars, callable implementations, or external service inputs. Its class names describe these observations.

## From observation to induction

Write a deterministic transition as F over the complete relevant state. If F(s) = s, induction gives Fⁿ(s) = s. If Fᵖ(s) = s, the same argument gives a repeating orbit. The proof needs the same transition and inputs at each step, not just similar console output.

A fixture can supply evidence for this argument when its registry captures all varying state and its functions are deterministic. A general host-driven program may not meet those assumptions. Changing aliases, input services, allocation availability, or callable behavior can break the conclusion while a previous finite trace remains correct.

When a class check fails, inspect the first differing tick and the fixture's intended behavior. Update a pin only after deciding whether the behavior change is intended. Do not increase a horizon or relabel a case merely to make the harness pass.
