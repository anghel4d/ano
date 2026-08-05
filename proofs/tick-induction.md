# Tick induction over the demo corpus

Whether "the demo ticks at step n and n+1" extends to every step is a per-demo theorem, not a harness output. This file states the tick map, proves the extension where an invariant carries it, and records the exact finite horizons where none does. Proofs here are hand-checked and witnessed by execution; they are not Lean-checked and claim no kernel status. Measurements: steel/kore at `05646cf`, CBQN backend, 87 unique active worlds (a `-nihongo` twin shares its ASCII twin's dynamics; quarantined numbers excluded).

## The tick map

Kore's `n` is `T(W) = save(run(P, W))` with `P` the staged program: the demo verbatim, `--! registry` retargeted to the play scratch, `--! out`/`expect`/`expect-n` pins stripped. `P` is fixed across presses, none of the active demos consults a host service, the alias sidecar is frozen, and emission is deterministic (twin emission is byte-identical; the trace battery pins run determinism), so `T` is a deterministic function on saved worlds. Every claim below is about iterating `T`.

## Theorems

1. Fixed point. If `T(W*) = W*` byte-for-byte, then `T^k(W_0) = W*` for every `k` past the tick that produced `W*`, and each later tick re-executes a verified computation. Immediate from determinism.
2. Verified orbit. If the trajectory revisits a state, every reachable future state lies on the observed cycle and every future transition was already executed successfully. Same argument, applied to the closed orbit.
3. Additive absorption. Iterating `g ← fl(g + c)` in float64 round-to-nearest, for finite nonzero `c`, is strictly monotone until the first representable `g*` with `|c| < ulp(g*)/2`, which absorbs; the representables between `g_0` and `g*` are finite, so the orbit reaches `g*` in finitely many steps (or saturates at `±∞` first, which `num` admits and which absorbs). A demo whose mutations are `+=`/`-=` by constants under selections that are constant, monotone-stable, or self-extinguishing therefore reaches a world fixed point componentwise. Executable witness: a world with gold at `2^63` (ulp `2048 > 2·1000`) is frozen by `Gold += 1000` from tick one, while gold at `2^62` climbs by the rounded step `1024` per tick.
4. Geometric saturation. `*= c` with `|c| > 1` grows strictly until overflow rounds to `±∞`; `∞` absorbs under the demos' further `+ c` and `· c`; no active demo forms `∞·0`, `∞−∞`, or `∞/∞`, so no NaN arises and publication never refuses. `/= 2` descends through the subnormals to exact `0`, which absorbs. Verified by running: `021` fixes at tick 1018 with merchant gold `inf` and bystanders untouched, `002` at 1022, `007` at 1079 with speed exactly `0`; the saturated worlds save and reload through the admitted-infinity fixtures.
5. Monotone tag lattice. `+Tag` under any selection only sets bits over a fixed row set: the world ascends a finite lattice and fixes in at most rows×columns ticks. Despawn exhaustion is the dual: population is non-increasing and bounded, so a `~` whose selection cannot refill empties in finitely many ticks.

## The corpus, classified

- Fixed by tick 2 (theorem 1, verified): 34 worlds.
- Delayed fixed point (theorem 1, verified): `011`, `134`, `136` at tick 3; `135` at tick 53.
- Verified cycle (theorem 2): `024` period 2; `137` period 4 entered at tick 3 — the clamp bounces `hp` around the `-= 200` retraction rather than pinning it.
- Saturation verified by running (theorems 3–4): `002`, `007`, `021`.
- Proved convergent, fixed point beyond running (theorems 3–5): `001 003 004 005 006 008 009 010 013 015 016 019 082 083 084 085 086 092 095 096 097 098 100 103 111 112 113 122 123 124 125 127 128 129 130 131 132`. Additive lanes absorb near `|c|·2^52` (a `+= 1000` purse freezes near `4.5×10^18` after ~`10^15` ticks); `111`/`112` already hold `Damage = inf` at tick 1200 while `Speed -= 2` still descends its absorption lane. The convergence is the theorem; the horizon is unreachable wall-clock and is claimed only as proved.
- No induction, provable finite horizon: `023` keeps `n = 6` while `spawn`'s key mint (`1 + max`) raises `unique keys` by 2 per tick — every state is new, the orbit never closes, and the tick is total exactly until the minted key would leave `nat` at `k ≈ 4.5×10^15`, where the carrier refuses. The induction that holds is bounded: total for all `k` below the wall, refused at it.
- No induction, measured implementation ceiling: `022`, `054`, `138` grow population geometrically (their spawned rows re-enter the spawning selection) and die at ticks 16–17 with `n ≈ 65540` — CBQN segfaults parsing the emitted multi-megabyte world literals. Semantically the tick is total for every `k` (spawn is total, the mint injective, every statement length-generic); the ceiling is the fixed backend's, recorded here rather than worked around. `048` (+9 rows/tick), `049` (+5), `126` (+7) grow linearly with spawned rows outside the spawning set, so the same semantic totality holds and the same ceiling projects to `k ≈ 7.3×10^3`, `1.3×10^4`, `9.4×10^3`; verified to tick 64.

## The executable form

The classification is a standing pin, not a report: `src/tick-classes.txt` records every active demo's class and `src/check-tick.sh` re-derives each class on every run — fixed points must still fix at their exact tick, cycles must still close with their exact period, evolving demos must still evolve, and the absorption, infinity, and zero witness worlds re-prove theorems 3–4's absorbing semantics each time. `check-ano.sh` chains it, so the standard battery fails on any drift. `ANO_TICK_SATURATE=1` additionally runs the three measured saturation fixed points (`002`@1022, `007`@1079, `021`@1018) to their exact ticks. A class mismatch always means one of three things — the test was never good, a grammar intentionally changed, or the implementation broke — and the mismatch is the signal to decide which.

## What the step case assumes

Determinism of the backend, no host input, canonical serialization, and a single writer. These are the hypotheses under which theorems 1–2 are trivial and 3–5 are IEEE-754 facts; a future demo that reads a service, randomizes, or writes the world concurrently re-opens the question for itself. A new demo joins a class by exhibiting its invariant, not by ticking twice.
