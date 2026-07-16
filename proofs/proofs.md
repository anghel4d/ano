# Proofs

Visible mathematical obligations and executable witnesses for Ano. These files are active tests of the specification even where Steel does not yet enforce them.

- `foundations.md` — schema, habitats, layouts, query lineage, reindexing, relationships, aggregation, effects, structural change, lattice placement, boundaries, and columnar lowering.
- `lean.md` — the machine-checked first milestone, theorem map, build command, and trust boundary.
- `Ano/*.lean` — the dependency-free Lean semantic kernel and guarded negative witnesses.
- `../docs/spatialmaths.md` — the complete spatial derivation, including the rejected preliminary definitions and corrected laws.
- `../demos/6-space/*.bqn` — concrete spatial array laws.
- `../demos/7-tiers/*.bqn` — counterexamples to the retired tier arguments and local symmetry/order witnesses.
- `../demos/10-conways/*.bqn` — repeated-step barrier and explicit-neighborhood witnesses.

A BQN witness proves only the equation it asserts. It does not make BQN an oracle for Ano. A Lean theorem proves its stated semantic kernel law; it becomes an implemented language guarantee only when the specification states it and Steel/Kore satisfy the corresponding positive and refusal tests.

The active obligations are deliberately ordinary visible files. Do not hide proof work in dotfiles or temporary audit artifacts.
