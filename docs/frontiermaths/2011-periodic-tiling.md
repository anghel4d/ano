# 2011 — A translational tile with no periodic tiling in dimension three

openai/math family **155**. Card **2011**.

## What it says

There exists a finite subset of ℤ³ that tiles ℤ³ by translations and admits no fully periodic tiling. Thickening each point to a unit cube gives the same failure in ℝ³, even if arbitrary real translation vectors are allowed.

This disproves the periodic tiling conjecture in the smallest dimension where a counterexample can exist.

## Manuscripts

- Family entry: [CONTENTS.md §155](https://github.com/openai/math/blob/main/CONTENTS.md)
- [A translational tile with no fully periodic tiling in dimension three](https://github.com/openai/math/blob/main/preprints/A-translational-tile-with-no-fully-periodic-tiling-in-dimension-three-September-23-2026/paper.pdf) — folder [`preprints/A-translational-tile-with-no-fully-periodic-tiling-in-dimension-three-September-23-2026/`](https://github.com/openai/math/tree/main/preprints/A-translational-tile-with-no-fully-periodic-tiling-in-dimension-three-September-23-2026)

## Lean status

Partial. One comparator: [`lean/docs/155.md`](https://github.com/openai/math/blob/main/lean/docs/155.md), statement in `PeriodicTilingThree.lean`.

Formalized: a finite tile in ℤ³ that tiles but has no complement invariant under a finite-index subgroup; the unit-cube thickening tiles ℝ³ almost everywhere with no fully periodic tiling, even under real translations; dimension three is the least lattice dimension where this occurs.

## Tie to this repository

Task 99 contemplates a discrete 3-D habitat and declared lattice embeddings ([todo/99](../../todo/99-spatial-lattice.md), [Space.lean](../../proofs/Ano/Space.lean)). Quarantined space demos must be rewritten from declared habitats, layouts, frames, and boundaries ([demos/6-space](../../demos/6-space/README.md)). Conway-style fields need a declared habitat, Moore relation, and boundary policy ([demos/10-conways](../../demos/10-conways/README.md)).

Practical implication: **don't assume tileability implies a periodic board**. A 3-D pattern that covers the lattice by translations need not have a repeating fundamental domain. A Kore view, a wrap boundary, or a `til` generation that presupposes a period can be incomplete even when a tiling exists.

This does not block rectangular or cubic product lattices, which remain periodic by construction. It does not change the current singleton 2-D `lattice w h` ([ano-ecs.md](../ano-ecs.md)). It is a refusal to treat "this prototype tiles" as "this prototype has a period we can store as width × height × depth."

## Caveats

The counterexample is existential and specific. Most tiles people write by hand are periodic. The result is a don't-bother on a general theorem, not a requirement to support aperiodic 3-D storage in Steel.

Boundary policies, wrap, and clamp are still declared services ([todo/99](../../todo/99-spatial-lattice.md)). This paper does not define them.
