# Demos

Demonstrating / experimenting with the language at various stages of development. 

Currently, this means working through the language examples in BQN, Erlang, Haskell, or OCaml to hash out everything we can about the semantics before committing to a compiler. 

A temporary transpiler through BQN or OCaml might be considered at a later point.

We use BQN to work through and verify the semantics of ano language ahead of time. An implementation in src/ is verified against the BQN post-states by differential testing.

That implementation exists: `anoc` (src/), an ano-to-BQN transpiler in C. Every .bqn demo has an .ano twin beside it — the demo's own `# ano:` statements as a runnable program, a `<name>.reg` registry in `demos/registries/` reproducing the .bqn fixture, and `--!` directives pinning the same post-state as BQN assertions. `src/anoc --run demo.ano` compiles the program, runs it under CBQN, and exits nonzero on any divergence. Split twins (-a/-b/-c) pin intermediate program points; the two .bqn files with no ano surface (s16-keygen, t1-reverse) stay BQN-only witnesses.

## Layout

- `1-selection/` — masks, scopes, aliases, presence, value predicates, hops, named selections (ex1-9, the set-hop family s05a-d split by sub-world with the s05d wrap-up, the s60 keyed hop through a `unique` column).
- `2-effects/` — value, assignment, structural, and sequenced effects; the barrier and pre-state (ex10-13, 25); the soul-gated spawn s10-soul-gate (the loop-safe barrier variant); the despawn-then-hop integrity pair s58/s59 (wrong-entity and out-of-range shapes, resolved through the hidden fixture-row column).
- `3-fold-scan/` — reductions, named reducers, scans, along; the fold contract and the Fibonacci stencil pair (ex14-17); the boolean-scan latches s62 (`|\` ever-any) and s63 (`&\` still-all); s64 the reducer-spelling triple (`threat/`, `fold(threat)`, `threat\`).
- `4-order/` — grade, rank, ordered top-k; the rank-tie write-back (ex18, 19, 26, 31).
- `5-generate/` — outer product, replicate, expand, reshape; keys come only from generation (ex20-24); s61 the proto spawn — `def Marine …` and the three-layer fill, the unique mint.
- `6-space/` — Part IV: lattice patterns, computed lines, spatial folds and scans, density fields, derived fields, the board literal (ex27-34).
- `7-tiers/` — the two-habitats table and the tier witnesses (ex35, 37-39, the foundations counterexamples).
- `8-gamma/` — the grouped fold over relationship fibers (the ex36 farm lines).
- `9-nihongo/` — the Japanese surface examples (ex40-49), plus the native-noun witnesses s50-s53: natively Japanese registries, mixed surfaces, the bilingual ja bridge, and the registry-kind coverage of the emitter's name mangler.
- `10-conways/` — Conway's Game of Life: the synchronous step as one barrier, the glider flipbook, the naru pair under the shared rule barrier — bloom+wither in one tick, the guard-complement certificate (c1-c3, c3-b).
- `11-noita/` — Noita's wand, card by card: casts and multicast, modifier order, homing as marker-plus-rule, the trigger, lattice alchemy with the anchored frame and the spread+consume tick, the assembled wand (n1-n6, n5-b/-c); the wand-construction stress test — the draw scan, the wrap, the modifier chain and its two general-join spellings, the Greek letters, the timeless ledger, the eval splice (w1-w6; what stays open, ISSUES.md).
- `12-registry-forms/` — one program, many registry forms: `r1-beside` loads a `.reg` sitting next to it (bare name), `r2-central` runs the same program and world from `../registries/` (the path form every other demo uses), and the `s57-derived-tag-{a,b}` pairs run one program over two representations of one tag — stored bits versus an `as`-derived equality mask — with identical pins.
- `registries/` — the world fixtures, one `<name>.reg` per twin; a twin loads its own with `--! registry ../registries/<name>.reg`.
- `check.sh` — runs every .bqn under demos/, one ok/FAIL line per file, nonzero exit on any failure.
- `../src/check-ano.sh` — the same contract over every .ano twin, through `anoc --run` (build anoc first: `make -C src`).