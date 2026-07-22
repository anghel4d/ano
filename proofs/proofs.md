# Proofs

Visible mathematical obligations and executable witnesses for Ano. These files are active tests of the specification even where Steel does not yet enforce them.

- `foundations.md` — schema, habitats, layouts, query lineage, reindexing, relationships, aggregation, effects, structural change, boxes, frames, localization, support projection, boundaries, and columnar lowering, with checked and pending obligations separated.
- `lean.md` — all three machine-checked milestones, their theorem maps, build commands, axiom accounting, and exact trust boundary.
- `Ano/*.lean` — the dependency-free Lean semantic kernels and guarded negative witnesses.
- `Ano/{Space,SpatialRegistry,Lineage,SpatialQuery,SpatialPlan,SpatialSpawn,SpatialWorld,ColumnBundle,Affine,Interpolation,SpatialNegative,SpatialAdvancedNegative}.lean` — the spatial contract: nominal domains and frames, sealed location and interpolation lineage, affine torsors and registered linear embeddings, weighted support, deterministic support choice, actual partial live `Position` writes, heterogeneous aligned inputs, exact-51 planning, dependent repeated ticks, and guarded rejection witnesses.
- `../docs/spatialmaths.md` — the complete spatial derivation.

A BQN witness proves only the equation it asserts. It does not make BQN an oracle for Ano. A Lean theorem proves its stated semantic kernel law; it becomes an implemented language guarantee only when the specification states it and Steel/Kore satisfy the corresponding positive and refusal tests.

The active obligations are deliberately ordinary visible files. Do not hide proof work in dotfiles or temporary audit artifacts.
