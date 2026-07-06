# src/

The source code of the ano lang's infrastructure:
- Compiler code.
- Parser / Interpreter.
- Integration infrastructure.

## anoc

`anoc` is the first working implementation: an ano-to-BQN transpiler in C23 with CBQN as the evaluation engine. A statement compiles to one gather-effect-scatter barrier — selection mask against pre-state, effects staged as temps reading only pre-state, one commit — so the barrier, saved-mask continuation, `;`-batching, and pre-state laws hold by construction in the emitted code. `GRAMMAR.md` is the surface contract (tokens, precedence, statement forms, directives, the Japanese skin); where it and ano-language.md disagree, ano-language.md wins.

- `ano.h` — the one contract: token columns, AST, registry, directives, module APIs; header-only arena, string buffer, and intern pool (one canonical copy per spelling, FNV-1a open addressing).
- `lex.c` — tokenizer, ASCII surface plus the Japanese skin (registry ja aliases, particle table, kanji numerals, ex40's TGT-drop and postfix re-root normalization). The stream is struct-of-arrays — kind/name/num/line columns grown together — and token text is interned, no per-token buffers. Source is validated as strict UTF-8 once at the ano_lex boundary (overlongs, encoded surrogates, out-of-range rejected); unknown characters report by codepoint.
- `parse.c` — Pratt parser over the spec appendix's 14 precedence levels.
- `registry.c` — registry loader; the `.reg` file format is specified at `reg_load` in ano.h. A registry stands in for the host: columns, presence, relationships (functional, set-valued, inverse reads), aliases, bindings, and callables carrying their own BQN — the eventual host ECS replaces exactly this file.
- `emit.c` — AST to BQN codegen against the registry; carries the semantic conventions (sym columns as strings, pair columns, id-space fiber membership, fold identities and row-drop, Tier-1 field inscription vs entity spawn).
- `fs.c` — path values and file reading, ported and miniaturized from the anoptic-engine filesystem module: AnoPath (a checked value type — truncation is a detectable len 0, never a silently clamped open target), exe-dir discovery, dirname, checked join, lexical ./.. normalization, and the one fortified reader (rejects directories, detects short reads, preserves errno) both main.c and registry.c go through.
- `rt.bqn` — the runtime prelude (Ano-prefixed helpers: dense rank, scatter-under-mask, fiber image, inverse fibers, clamp shift).
- `main.c` — driver: `anoc [--tokens] [--emit] [--run] [--rt <path>] [--registry <path-or-name>] file.ano`; `--!` directives name the registry and pin expectations, compiled into BQN assertions, so `--run`'s exit code is the differential-test verdict.
- world fixtures — a `<name>.reg` in `demos/registries/` reproduces each demo's initial world exactly; the twin's `--! registry ../registries/<name>.reg` loads it. A bare `--! registry <name>` still resolves beside the `.ano`, so a fixture may sit next to its twin instead.
- `check-ano.sh` — runs every demos/**/*.ano through `anoc --run`, ok/FAIL per file (mirrors demos/check.sh).

Build and test (inside the dev shell for CBQN): `make -C src && ./src/check-ano.sh`, or `make -C src test`.

Known gaps, deliberate for now: spawned rows take registry defaults, not source-row copies; an effect verb cannot share its target column's name (reg lookup is kind-blind); `--! expect` targets registry columns only, so def-derived columns pin indirectly; numeric expectations compare within 1e-9 (float pins come from sibling BQN evaluations). The generation notes under the demo twins record each omission per file.
