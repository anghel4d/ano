# CLAUDE.md

## What this is
Ano (あの). A programming language design. An embeddable ECS scripting layer that addresses entities by description: the selection predicate is the entity reference. FP/LISP/APL lineage, ASCII surface, Lua-class. The name is the Japanese distal demonstrative ("that one over there"). Read ano-language.md for the full spec.

One design document so far, `ano-language.md`. No compiler yet. Work here is design and spec writing.

## The shape
One form governs the static layer:

```
source & predicate , effect
```

Selection on the left, effect on the right, comma between. `source` defaults to the live world. Selection is relational algebra (σ by predicate, ⋈ by the dotted relationship hop, π by component access); column effects are the array calculus (reduce, scan, grade, outer product, replicate, reshape). A column store unifies them.

## Conventions
- Design repo. The spec borrows surface flavor across languages: APL/BQN/J/k, Haskell, Erlang, q/kdb+, SQL, Datalog, Lisp. Keep each example idiomatic to the language it cites.
- Read `ano-language.md` before editing it. Match its part/section numbering and the canonical-task addendum already in place.
- Open design questions live under "Open Questions, Next Steps". Never resolve one silently in prose; surface the tradeoff.
- Bootstrap is open: the bytecode VM and JIT target are fixed, the front-end language (APL, Haskell, or OCaml) is not.
- No heavyweight deps. No frameworks.

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
- For any file search or grep in the current git-indexed directory, use fff tools.
