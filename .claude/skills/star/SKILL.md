---
name: star
description: Use automatically whenever enumerating two or more concrete, code-anchored issues at once — bugs, defects, gaps, missing checks, stale or contradictory spec, deferred fixes, surfaced tradeoffs — each of which maps to a place in the code or spec. Lays them out in STAR format (Situation, Task, Action, Resolution) plus (a) where the issue IS and (b) where it AFFECTS.
argument-hint: "[the problems, or a file/area to enumerate them from]"
---
Report every problem in $ARGUMENTS using this template. One entry per problem, numbered.

#N - Issue Name
- S: (situation contents)
- T: (task contents)
- A: (action/steps taken contents)
- R (resolution contents)
- (a) WHERE in code the issue IS (what spec item, src file, etc exactly)
- (b) WHERE in code the feature AFFECTS (example)

EXAMPLE:
#2 — Stale ↕ in the space-operations open-question
- S: ano-language.md's "Space operations" bullet names the generator with the retired glyph ↕. Step 8 respelled the surface to til.
- T: Is that ↕ a retired surface token (respell) or a conceptual BQN glyph label (leave)?
- A: If surface, change the one ↕ to til. Didn't: it sits parallel to ⥊ as a glyph label in your prose (provenance rule).
- R: No stale surface spelling in the spec prose, if you read it as surface.
- (a) Issue IS: ano-language.md:879, the ↕ in `generate (↕, a bare numeric shape…)`, under `## Open Questions, Next Steps`.
- (b) Affects: the surface generator, now spelled til: added at src/lex.c:74 (the kwkind table, {"til",T_IOTA}), the old ↕ branch deleted from lex_ascii, U+2195 moved to the blacklist ublack (src/lex.c:27). Live: demos/7-tiers/35-two-habitats-a.ano:13 is now til 5, demos/6-space/30-space-reductions.ano:17 is max\ Height @ (Eye + til n * north).

