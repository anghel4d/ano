# Demos

Demonstrating / experimenting with the language at various stages of development. 

Currently, this means working through the language examples in BQN, Erlang, Haskell, or OCaml to hash out everything we can about the semantics before committing to a compiler. 

A temporary transpiler through BQN or OCaml might be considered at a later point.

We use BQN to work through and verify the semantics of ano language ahead of time. An implementation in src/ is verified against the BQN post-states by differential testing.

## Layout

- `selection/` — masks, scopes, aliases, presence, value predicates, hops, named selections (ex1-9, the set hop).
- `effects/` — value, assignment, structural, and sequenced effects; the barrier and pre-state (ex10-13, 25).
- `fold-scan/` — reductions, named reducers, scans, along; the fold contract and the Fibonacci stencil pair (ex14-17).
- `order/` — grade, rank, ordered top-k; the rank-tie write-back (ex18, 19, 26, 31).
- `generate/` — outer product, replicate, expand, reshape; keys come only from generation (ex20-24).
- `gamma/` — the grouped fold over relationship fibers (the ex36 farm lines).
- `space/` — Part IV: lattice patterns, computed lines, spatial folds and scans, density fields, derived fields, the board literal (ex27-34).
- `tiers/` — the two-habitats table and the tier witnesses (ex35, 37-39, the foundations counterexamples).
- `nihongo/` — the Japanese surface examples (ex40-49).
- `check.sh` — runs every .bqn under demos/, one ok/FAIL line per file, nonzero exit on any failure.