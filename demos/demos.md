# Demos

Demonstrating / experimenting with the language at various stages of development. 


We use BQN to work through and verify the semantics of ano language ahead of time. An implementation is verified against the BQN post-states by differential testing.

That implementation exists: Steel (steel/), the Rust reference, launched as `steel`; its C predecessor `anoc` (src/) stays beside it as the frozen differential oracle. Every .bqn demo has an .ano twin beside it — the demo's own `# ano:` statements as a runnable program, a `<name>.reg` registry in `demos/registries/` reproducing the .bqn fixture, and `--!` directives pinning the same post-state as BQN assertions. `target/release/steel --run demo.ano` compiles the program, runs it under CBQN, and exits nonzero on any divergence. Split twins (-a/-b/-c) pin intermediate program points; the two .bqn files with no ano surface (053-keygen, 072-reverse) stay BQN-only witnesses.

## Layout

- `1-selection/` — 001-015: masks, scopes, aliases, presence, value predicates, hops, named selections (ex1-9); the set-hop family 010-013, three sub-worlds and the 013-set-hop wrap-up (was s05a-d); the keyed hop 015 through a `unique` column (was s60).
- `2-effects/` — 016-027: value, assignment, structural, and sequenced effects; the barrier and pre-state (ex10-13, 25); the soul-gated spawn 025-soul-gate (the loop-safe barrier variant); the despawn-then-hop integrity pair 026/027 (was s58/s59: wrong-entity and out-of-range shapes, resolved through the hidden fixture-row column).
- `3-fold-scan/` — 028-040: reductions, named reducers, scans, along; the fold contract and the Fibonacci stencil pair (ex14-17); the boolean-scan latches 038 (`|\` ever-any) and 039 (`&\` still-all); 040 the reducer-spelling triple (`threat/`, `fold(threat)`, `threat\`) — was s62-s64.
- `4-order/` — 041-045: grade, rank, ordered top-k; the rank-tie write-back (ex18, 19, 26, 31).
- `5-generate/` — 046-054: outer product, replicate, expand, reshape; keys come only from generation (ex20-24); 054 the proto spawn (was s61) — `def Marine …` and the three-layer fill, the unique mint.
- `6-space/` — 055-063, Part IV: lattice patterns, computed lines, spatial folds and scans, density fields, derived fields, the board literal (ex27-34).
- `7-tiers/` — 064-075: the two-habitats table and the tier witnesses (ex35, 37-39, the foundations counterexamples).
- `8-gamma/` — 076-081: the grouped fold over relationship fibers (the ex36 farm lines).
- `9-nihongo/` — 082-104: the Japanese surface examples (ex40-49), plus the native-noun witnesses 095-104 (was s50-s56): natively Japanese registries, mixed surfaces, the bilingual ja bridge, and the registry-kind coverage of the emitter's name mangler.
- `10-conways/` — 105-109, Conway's Game of Life (was c1-c3): the synchronous step as one barrier, the glider flipbook, the naru pair under the shared rule barrier — bloom+wither in one tick, the guard-complement certificate.
- `11-noita/` — 110-128, Noita's wand, card by card (was n1-n6): casts and multicast, modifier order, homing as marker-plus-rule, the trigger, lattice alchemy with the anchored frame and the spread+consume tick, the assembled wand; the wand-construction stress test (was w1-w6) — the draw scan, the wrap, the modifier chain and its two general-join spellings, the Greek letters, the timeless ledger, the eval splice (what stays open, ISSUES.md).
- `12-registry-forms/` — 129-132: one program, many registry forms: `129-beside` loads a `.reg` sitting next to it (bare name), `130-central` runs the same program and world from `../registries/` (the path form every other demo uses), and the derived-tag pair 131/132 (was s57) runs one program over two representations of one tag — stored bits versus an `as`-derived equality mask — with identical pins.
- `13-typed-registries/` — 133-138, the carrier refinements (Steel, 2026-07-12): `bool` enforced by the 0< retraction (133), `nat` — the naturals, floor at 0 with no underflow (134), floor to the integer grid and the 2^53 ceiling (135), `int` its signed twin, ⌊ toward −∞ (136), the declared `range` bounds over the untyped double (137), and constraints stacking — `unique id nat`, `unique slot int`, the mint respecting either carrier (138). Rest data seals at load (the negative battery pins the refusals), committed writes retract at the barrier, the TUI clamps and warns.
- `registries/` — the world fixtures, one `<name>.reg` per twin; a twin loads its own with `--! registry ../registries/<name>.reg`.
- `check.sh` — runs every .bqn under demos/, one ok/FAIL line per file, nonzero exit on any failure.
- `../src/check-ano.sh` — the same contract over every .ano twin, through `steel --run` (build it first: `cargo build --release`).