# 2167 — Grothendieck homotopy hypothesis via elementary expansions

openai/math family **312**. Card **2167**.

## What it says

Grothendieck's homotopy hypothesis asks whether algebraic ∞-groupoids present the same homotopy theory as topological spaces (or simplicial sets). The result answers yes for the weak globular ∞-groupoids associated with every Grothendieck coherator in the Ara–Henry convention.

The method is elementary expansions. Attaching an (n+1)-disk along its source n-face is a weak equivalence, for every such coherator and every boundary-cellular model. Henry's pushout conjecture follows: those expansions preserve path components and all homotopy groups of cellular ∞-groupoids.

In plain terms: the algebraic gadgets Grothendieck proposed as "higher groupoids" really do model spaces. Adding a cell along a boundary does not change homotopy type.

## Manuscripts

- Family entry: [CONTENTS.md §312](https://github.com/openai/math/blob/main/CONTENTS.md)
- [The Grothendieck homotopy hypothesis via elementary expansions](https://github.com/openai/math/blob/main/preprints/The-Grothendieck-homotopy-hypothesis-via-elementary-expansions-September-24-2026/paper.pdf) — folder [`preprints/The-Grothendieck-homotopy-hypothesis-via-elementary-expansions-September-24-2026/`](https://github.com/openai/math/tree/main/preprints/The-Grothendieck-homotopy-hypothesis-via-elementary-expansions-September-24-2026)

## Lean status

Partial. One comparator: [`lean/docs/312.md`](https://github.com/openai/math/blob/main/lean/docs/312.md), statement in `GrothendieckElementaryExpansion.lean`.

Lean covers the elementary-expansion weak-equivalence theorem used in the approach. The later semi-model structure and the full comparison with the homotopy theory of spaces are **not** formalized.

## Tie to this repository

This is the types-as-spaces tradition. A type is a space, a term is a point, an identification is a path. That is the opposite of Ano's [types-as-predicates](types-as-predicates.md) reading and of the current Lean kernel.

`proofs/Ano/` builds explicit 1-categorical structures: finite layouts and reindexing ([Field.lean](../../proofs/Ano/Field.lean)), affine frames with a simply transitive point action ([Affine.lean](../../proofs/Ano/Affine.lean)), weighted interpolation fibers with algebraic normalization ([Interpolation.lean](../../proofs/Ano/Interpolation.lean)), and lineage for query provenance ([Lineage.lean](../../proofs/Ano/Lineage.lean)). Points, vectors, cells, and entity keys remain distinct. Equal cardinality is not an identification ([spatialmaths.md](../spatialmaths.md)).

2167 does not license collapsing those distinctions. An elementary expansion is not a Steel rewrite, not a spatial plan, and not a Kore view update. Task 99's five prototype worlds (discrete and continuous 2-D/3-D, and a sphere) are habitats and frames, not coherators ([todo/99](../../todo/99-spatial-lattice.md)).

Practical implication: **do not import this**. Do not add HoTT path types, globular ∞-groupoids, or coherator combinators to the registry, the emitter, or the spatial kernel. If a future Sky language needs higher structure, that is a new design question; 2167 does not settle it and does not weaken the predicate reading.

The one honest use is contrast. When someone proposes "types are spaces, so lineage is a path," the answer is: 2167 makes that precise *in homotopy theory*, and this repository's contracts reject that translation.

## Caveats

The Lean formalization stops at elementary expansions. Citing the full homotopy-hypothesis comparison as machine-checked is false.

The result is existence of a Quillen-style equivalence of homotopy theories. It gives no algorithm, no complexity bound, and no geometry service. It does not speak to float64 interpolation, phyllotaxis, raycasts, or Kore hit policy ([foundations.md](../../proofs/foundations.md), Outstanding mathematical scope).
