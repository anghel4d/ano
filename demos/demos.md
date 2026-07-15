# Demos

Demonstrating and stress-testing the language at different stages of development.

Steel is the current reference compiler and Kore is the current interactive world. `target/release/steel --run demo.ano` is the executable check. CBQN is Steel's present execution backend; it is not a language oracle.

The `.bqn` files are exploratory witnesses from the design process. They may explain an intended array transformation, but matching a one-step BQN post-state does not prove that Steel preserved habitat, lineage, rank, or repeated-tick behavior. The `.ano` directives pin only the states they name.

The spatial corpus in `6-space/`, the counterexample corpus in `7-tiers/`, and the Life corpus in `10-conways/` are active proof and acceptance tests. Some currently expose known Steel failures; that is their job. A passing first step never licenses a second step.

## Layout

- `1-selection/` — 001-015: masks, scopes, aliases, presence, value predicates, hops, named selections (ex1-9); the set-hop family 010-013, three sub-worlds and the 013-set-hop wrap-up (was s05a-d); the keyed hop 015 through a `unique` column (was s60).
- `2-effects/` — 016-027: value, assignment, structural, and sequenced effects; the barrier and pre-state (ex10-13, 25); the soul-gated spawn 025-soul-gate (the loop-safe barrier variant); the despawn-then-hop integrity pair 026/027 (was s58/s59: wrong-entity and out-of-range shapes, resolved through the hidden fixture-row column).
- `3-fold-scan/` — 028-040: reductions, named reducers, scans, along; the fold contract and the Fibonacci stencil pair (ex14-17); the boolean-scan latches 038 (`|\` ever-any) and 039 (`&\` still-all); 040 the reducer-spelling triple (`threat/`, `fold(threat)`, `threat\`) — was s62-s64.
- `4-order/` — 041-045: grade, rank, ordered top-k; the rank-tie write-back (ex18, 19, 26, 31).
- `5-generate/` — 046-054: outer product, replicate, expand, reshape; keys come only from generation (ex20-24); 054 the proto spawn (was s61) — `def Marine …` and the three-layer fill, the unique mint.
- `6-space/` — 055-063: active spatial proofs and acceptance witnesses for named habitats, fixed-rank fields, explicit boundaries, lineage, and repeated ticks.
- `7-tiers/` — 064-075: historical tier examples and counterexamples; the tier hierarchy and its purported proofs are retired.
- `8-gamma/` — 076-081: the grouped fold over relationship fibers (the ex36 farm lines).
- `9-nihongo/` — 082-104: the Japanese surface examples (ex40-49), plus the native-noun witnesses 095-104 (was s50-s56): natively Japanese registries, mixed surfaces, the bilingual ja bridge, and the registry-kind coverage of the emitter's name mangler.
- `10-conways/` — 105-109, Conway's Game of Life: valid evidence for barrier and explicit Moore-relation behavior, not evidence that Steel's current field representation enforces habitat or rank.
- `11-noita/` — 110-128, Noita's wand, card by card (was n1-n6): casts and multicast, modifier order, homing as marker-plus-rule, the trigger, lattice alchemy with the anchored frame and the spread+consume tick, the assembled wand; the wand-construction stress test (was w1-w6) — the draw scan, the wrap, the modifier chain and its two general-join spellings, the Greek letters, the timeless ledger, the eval splice (what stays open, ISSUES.md).
- `12-registry-forms/` — 129-132: one program, many registry forms: `129-beside` loads a `.reg` sitting next to it (bare name), `130-central` runs the same program and world from `../registries/` (the path form every other demo uses), and the derived-tag pair 131/132 (was s57) runs one program over two representations of one tag — stored bits versus an `as`-derived equality mask — with identical pins.
- `13-typed-registries/` — 133-138, the carrier refinements (Steel, 2026-07-12): `bool` enforced by the 0< retraction (133), `nat` — the naturals, floor at 0 with no underflow (134), floor to the integer grid and the 2^53 ceiling (135), `int` its signed twin, ⌊ toward −∞ (136), the declared `range` bounds over the untyped double (137), and constraints stacking — `unique id nat`, `unique slot int`, the mint respecting either carrier (138). Rest data seals at load (the negative battery pins the refusals), committed writes retract at the barrier, the TUI clamps and warns.
- `registries/` — the world fixtures, one `<name>.reg` per twin; a twin loads its own with `--! registry ../registries/<name>.reg`.
- `check.sh` — runs every .bqn under demos/, one ok/FAIL line per file, nonzero exit on any failure.
- `../src/check-ano.sh` — the same contract over every .ano twin, through `steel --run` (build it first: `cargo build --release`).