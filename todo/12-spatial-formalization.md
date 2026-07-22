# 12 — Spatial formalization

Status: specified, not implemented. This file defines the observable result required from the future Steel and Kore update. It deliberately does not prescribe data structures, compiler passes, registry syntax, or algorithms.

## Authority

`docs/ano-language.md` is the language contract. `docs/spatialmaths.md` is the mathematical account. `proofs/foundations.md` lists the proof obligations. Steel is the reference compiler and Kore is the interactive world. BQN siblings are executable mathematical witnesses, not semantic or differential oracles.

## Required language outcome

A stored field belongs to exactly one named habitat. Its habitat identity, rank, and shape survive every ordinary value update and every successful tick.

Two equal-length columns on different habitats are foreign. Steel accepts a joint operation only when the query has placed both on one current row habitat through explicit lineage.

An ECS component remains partial on the live entity habitat. A spatial field remains total on its declared field habitat unless its schema explicitly declares sparsity. Spawning or despawning entities cannot resize a field.

A query has one current row habitat. Every non-scalar value consumed together is aligned on it. Selection, relationship traversal, grade, replicate, reshape, and generation retain enough lineage for a later effect to reach a legal destination.

A shape-changing read creates a derived value. It does not mutate the habitat, rank, or shape of a stored source or target. Exact reshape, cycling reshape, and `pos = to shape` have the three distinct meanings in `docs/spatialmaths.md`.

The destination's registry declaration supplies the position carrier and spatial axes. A two-coordinate `to` result is valid when that carrier has at least two axes. No lattice placement is required.

A plain assignment has at most one value for each destination. A colliding effect succeeds only when its declared merge makes the result independent of evaluation order. Otherwise the statement refuses before commit.

Every neighborhood has an explicit boundary meaning. Shrink, constant, clamp, reflect, and wrap remain observably distinct. No spelling silently chooses clamp for all habitats.

A lattice may exist without world placement. World placement is meaningful only when an ambient affine space, axes or coordinate chart, units, origin point, and basis map make it meaningful. `origin 100 0 200` is rejected or incomplete without that context.

A successful statement leaves a world satisfying the same schema and suitable for immediate consumption by the next tick. A refusal leaves the world byte-for-byte unchanged.

## Required Steel outcome

Steel accepts at least two named field habitats at once, including two with equal cardinality and different rank or shape, without cross-aligning them.

Steel distinguishes stored fields, ECS components, relation edges, anonymous generated habitats, and derived query habitats in observable diagnostics and refusals.

Steel preserves field habitat, rank, shape, and values not selected by a mask across save and reload.

Steel reports a domain error before CBQN execution when an expression combines foreign habitats without an explicit alignment.

Steel reports a destination or collision error before commit when an effect cannot lawfully scatter to its target.

Steel's emitted program may continue to use CBQN, but the result must be determined by Ano's domain and lineage semantics rather than BQN length agreement.

## Required Kore outcome

Kore displays enough field metadata to distinguish habitat, rank, and shape. Equal-length foreign fields cannot appear as one anonymous ground.

Pressing `n` twice is a required spatial operation, not an exploratory stress case. The second tick consumes the first tick's saved world without changing any registered field's habitat, rank, or shape.

Kore's `r`, `n`, `u`, save, reload, and direct field edit preserve the same habitat metadata.

A refused spatial tick leaves the play world and undo history in the same atomic state as any other refused tick.

A map or bitmap view uses declared spatial and placement metadata. It does not infer a 2D ground from a buffer length.

Kore renders and targets through the registry-declared position role and spatial axes.




## Required negative witnesses

Foreign equal-length field and component columns refuse without an explicit map.

Two equal-cardinality field habitats refuse accidental cross-write.

A rank-changing derived value refuses assignment into a fixed-rank field without a legal destination map.

A lattice field refuses an undeclared boundary operation.

A placed operation refuses an origin whose ambient dimension or units do not match.

Plain assignment refuses duplicate destinations.

A failed second tick commits nothing.

Spawn from a cell view does not fill an entity `parent` from the cell index unless an entity lineage is explicitly present.

## Completion test

The work is complete only when all affected Ano tests, their Nihongo siblings, and the BQN mathematical witnesses agree on the laws above; every required positive case passes in Steel and through two Kore ticks; every required negative case refuses before commit; save and reload preserve habitat metadata; and no active test or document names C or BQN as an oracle.

## Follow-up script

```text
Read AGENTS.md, docs/ano-language.md, docs/spatialmaths.md, docs/ano-ecs.md, proofs/foundations.md, and todo/12-spatial-formalization.md completely. Treat Steel and Kore as the only active implementations.

Your task is to make every observable outcome in todo/12-spatial-formalization.md true. The file specifies results, not an implementation plan. Choose the smallest coherent Steel and Kore changes that satisfy the contract without weakening, deleting, skipping, or converting any positive or negative witness.

Begin by running the affected Ano, Nihongo, BQN, refusal, save/reload, and repeated-tick cases and recording the exact failures. Preserve unrelated worktree changes. Keep the language surface open where the spec says it is open; do not freeze registry spelling merely to make a test convenient.

A field must keep its named habitat, rank, and shape through masks, spawns, saves, reloads, and repeated ticks. Equal length is never alignment. Every effect must reach its target through lawful lineage. Every boundary and placement assumption must be explicit.

Finish only when the required demo outcomes and negative witnesses pass, two consecutive Kore ticks consume one another's state, the complete Steel/Kore suite is green, and the diff contains no C-oracle fallback. Report the semantic outcomes, refusals, tests, and remaining open surface questions. Do not commit without the author's explicit approval for that future commit.
```
