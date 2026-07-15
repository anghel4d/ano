# CLAUDE.md

## What this is
Ano (あの). A programming language design. An embedded ECS query-and-command engine that addresses entities by description: the selection predicate is the entity reference. FP / APL lineage, ASCII surface, SQL/Datalog/production-rules class — the FP/array sibling of Lua, to the game world what q is to kdb+; beside a Lua-class host the split is coroutines versus triggers. The name is the Japanese distal demonstrative ("that one over there"). Read ano-language.md for the full spec.

The spec is `docs/ano-language.md`; satellite working notes orbit it (`docs/ano-ecs.md` the world store, `docs/ano-time.md` time and missions, `docs/ano-sky.md` purity, the Sky Registry, and the machine-as-world hypothesis). Steel, the Rust reference compiler and standalone launcher, lives in `steel/`; Kore, the Rust interactive world, lives in `kore/`. `src/` is an archived C predecessor and is neither an implementation target nor an oracle.

## The shape
One form governs the static layer:

```
source & predicate , effect
```

Selection on the left, effect on the right, comma between. `source` defaults to the live world. Selection is relational algebra (σ by predicate, ⋈ by the dotted relationship hop, π by component access); column effects are the array calculus (reduce, scan, grade, outer product, replicate, reshape). A column store unifies them.

## Conventions
- The Pious Hierarchy: The Mathematics > The Denotation > the domain-and-lineage IR > The Grammar > The Surface > lowering and backend details.
- The spec borrows surface flavor across languages: APL/BQN/J/k, Haskell, Erlang, q/kdb+, SQL, Datalog, Lisp. Keep each example idiomatic to the language it cites.
- Read `ano-language.md` before editing it. Match its part/section numbering and the canonical-task addendum already in place.
- Open design questions live under "Open Questions, Next Steps". Never resolve one silently in prose; surface the tradeoff.
- Bootstrap is ruled (2026-07-10, superseded in part 2026-07-15): Steel is the reference compiler and Kore is the current world. The archived C predecessor and BQN twins are not semantic or differential oracles. The bytecode VM and JIT target stay fixed.
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
