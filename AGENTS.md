# CLAUDE.md

## What this is
Ano (あの). A programming language design. An embedded ECS query-and-command engine that addresses entities by description: the selection predicate is the entity reference. FP / APL lineage, ASCII surface, SQL/Datalog/production-rules class — the FP/array sibling of Lua, to the game world what q is to kdb+; beside a Lua-class host the split is coroutines versus triggers. The name is the Japanese distal demonstrative ("that one over there"). Read ano-language.md for the full spec.

The spec is `ano-language.md`; satellite working notes orbit it (`ano-ecs.md` the world store, `ano-time.md` time and missions, `ano-sky.md` purity, the Sky Registry, and the machine-as-world hypothesis). The compiler `anoc` lives in `src/`. Work here is design, spec writing, and the differential-tested compiler.

## The shape
One form governs the static layer:

```
source & predicate , effect
```

Selection on the left, effect on the right, comma between. `source` defaults to the live world. Selection is relational algebra (σ by predicate, ⋈ by the dotted relationship hop, π by component access); column effects are the array calculus (reduce, scan, grade, outer product, replicate, reshape). A column store unifies them.

## Conventions
- The Pious Hierarchy: The Mathematics > The Semantics > The Grammar > The Syntax > keywords, pipelining, implementation details.
- The spec borrows surface flavor across languages: APL/BQN/J/k, Haskell, Erlang, q/kdb+, SQL, Datalog, Lisp. Keep each example idiomatic to the language it cites.
- Read `ano-language.md` before editing it. Match its part/section numbering and the canonical-task addendum already in place.
- Open design questions live under "Open Questions, Next Steps". Never resolve one silently in prose; surface the tradeoff.
- Bootstrap is ruled (2026-07-10): the language is drafted and its spec forged in Steel — the Rust reference implementation and standalone launcher; Cano, the embedded C implementation, derives from verified Steel, never the inverse. The bytecode VM and JIT target stay fixed.
- No heavyweight deps. No frameworks.
- Work tracking: TODO.md is a laundry list of current tasks. Crossed off items go under `## Closed` and get a green checkmark emoji or other relevant depending on whether resolved, cancelled, etc.
- PATCHES.md is the ledger of big patches as they get merged to main, in the style of Notch's early Minecraft Patch notes (intellectual property-safe).

## Writing Style
- Comments should follow the existing convention and generally be constrained to the top of functions: Inputs and their types, outputs and their types, invariants.
- Commments inside of functions should be extraordilarily terse and to the point.
- Write markdown the same way: flat, terse prose with no decorative bolding. Bold only academically and selectively for load-bearing terms.
- One long line per paragraph or list item. Let the editor soft-wrap. Do not hard-wrap prose at a column.
- Preserve the author's voice and his own comments verbatim. Tighten, don't rewrite.

## Constraints
- ALWAYS check if a directory you're working in has a .md file. If it does, read it and follow its guidance.
- Do NOT add yourself as a contributor.
- Do not `git commit` or `git push` without explicit per-commit approval. Show the diff and let the author commit.

## Cursor Cloud specific instructions
- The tree is C plus BQN, not Rust: `anoc` (src/, the ano→BQN transpiler), `kore` (kore/, the editor), and `common/` (the shared strings+arena module) are C23; BQN is the verification language every executable claim is checked against. The "Steel"/Rust bootstrap in the notes above is a future ruling, not the current implementation — there is no Cargo project.
- `bqn` is CBQN, the one dependency that is not a system package: it is built from source (github.com/dzaima/CBQN, `make REPLXX=0` to skip the C++ line-editor that needs libstdc++ headers absent here) and installed at `/usr/local/bin/bqn`. The startup update script rebuilds it only if `bqn` is missing, so normally it is already present.
- Build with `cc`, which is clang 18 (supports `-std=c23`). The Makefiles set `CC ?= cc`; do not override `CC=gcc`, since gcc 13 here rejects `-std=c23` (it only knows `-std=c2x`).
- Run/build/test, all fast (seconds): `make -C src` builds `anoc`; `make -C src test` runs the full differential corpus (`check-ano.sh`: every `.ano` twin through `anoc --run`, the `-nihongo` emit-equality pairs, and the `refusals/` negative battery via `check-refusals.sh`); `bash demos/check.sh` runs every `.bqn` demo; `make -C common test` builds and runs the strings smoke test; `make -C kore` builds the editor. See `demos/demos.md` for the corpus layout.
- Run a program directly: `src/anoc --run file.ano` transpiles to BQN, executes under CBQN, and exits nonzero if a `--!` pin (e.g. `--! expect col = ...`) diverges; `src/anoc --emit file.ano` prints the emitted BQN (runtime prelude from `src/rt.bqn` + fixture + program + assertions). A demo's world fixture is a `.reg` loaded via `--! registry <path>` or `--registry`.
- `flake.nix` describes the intended shell (cbqn, gcc, the demo languages, Lean4), but nix is not installed here and is not needed for the C+BQN core; the demo languages (Erlang/Haskell/OCaml/APL) and Lean4 are only for the satellite experiments, not the `anoc`/demo/common/kore build-and-test loop above.
