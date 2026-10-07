# 1874 — Ramanujan–Arthur decompositions and generic Ramanujan

openai/math family **014**. Card **1874**.

## What it says

This family is geometric and automorphic Langlands over function fields, not graph theory.

Restricted geometric Langlands: for connected reductive groups on smooth projective connected curves over an algebraic closure of a finite field, the restricted equivalence holds under four stated Lie-theoretic characteristic hypotheses. Over a general algebraically closed field of characteristic p > 0, the same conclusion needs p very good for the group and p not dividing |W_G|.

Ramanujan–Arthur: at every full finite level, cuspidal automorphic functions for split semisimple groups over global function fields decompose (rationally and ℓ-adically) into subspaces indexed by nilpotent orbits of the dual group. Assuming that decomposition, occurring cuspidal excursion parameters admit a global Arthur enhancement: one algebraic SL₂ and a commuting Weil centralizer recover the parameter by diagonal specialization on the whole Weil group, including inertia. The enhancement does not claim ellipticity, packet classification, or a multiplicity formula.

Generic Ramanujan: a globally generic cuspidal representation of a split adjoint absolutely simple exceptional group is tempered at every place, with no restriction on characteristic or ramification depth. For split adjoint absolutely simple groups more broadly, one generic unramified local component implies temperedness at every unramified place.

"Ramanujan" here means local components are tempered (the generalized Ramanujan conjecture for these automorphic representations). It does **not** mean a Ramanujan expander graph.

## Manuscripts

- Family entry: [CONTENTS.md §014](https://github.com/openai/math/blob/main/CONTENTS.md)
- [Ramanujan-Arthur Decompositions of Cuspidal Functions at Full Finite Level](https://github.com/openai/math/blob/main/preprints/Ramanujan-Arthur-Decompositions-of-Cuspidal-Functions-at-Full-Finite-Level-September-24-2026/paper.pdf)
- [Global Arthur Enhancements of Cuspidal Excursion Parameters](https://github.com/openai/math/blob/main/preprints/Global-Arthur-Enhancements-of-Cuspidal-Excursion-Parameters-October-5-2026/manuscript.pdf)
- [Temperedness at ramified places for globally generic exceptional groups](https://github.com/openai/math/blob/main/preprints/Temperedness-at-ramified-places-for-globally-generic-exceptional-groups-October-5-2026/ramified-ramanujan.pdf)
- [The Restricted Geometric Langlands Equivalence in Positive Characteristic](https://github.com/openai/math/blob/main/preprints/The-Restricted-Geometric-Langlands-Equivalence-in-Positive-Characteristic-September-24-2026/paper.pdf)
- [Rationality of the Canonical Unramified Arthur Filtration](https://github.com/openai/math/blob/main/preprints/Rationality-of-the-Canonical-Unramified-Arthur-Filtration-September-24-2026/paper.pdf)
- plus the tame Hecke eigensheaf papers in the same CONTENTS block (constructible, several marked points, Frobenius structures)

Folders live under [`preprints/`](https://github.com/openai/math/tree/main/preprints) with the names above.

## Lean status

None.

## Tie to this repository

Ano has no automorphic forms, Galois representations, excursion operators, or Hecke eigensheaves. Steel's relationships are stored functional keys and nested set-valued fibers ([ano-ecs.md](../ano-ecs.md), [relationship.rs](../../steel/src/relationship.rs)). Prime is relational converse ([ano-language.md](../ano-language.md) §5). Neighborhoods are a legacy clamp path or explicit `srel` edges ([ano-ecs.md](../ano-ecs.md), Spatial limit).

The name collision is the risk. Family **178** (card 2033) constructs deterministic nonbipartite Ramanujan *graphs* (every nonconstant adjacency eigenvalue inside (−2√(d−1), 2√(d−1))). That is the expander-theoretic "Ramanujan." 1874 is the automorphic one. They share a bound-on-spectrum metaphor and nothing else.

Practical implication: **do not bother**, and do not cite 1874 when discussing neighbor graphs, mixing on relationship fibers, or task-99 lattices. If a later host wants expander-quality adjacency, look at family 178 — and even that is unused: current worlds declare their edges, they do not construct Ramanujan graphs.

The special relational algebra remains an open review ([ano-language.md](../ano-language.md) §5 TODO). 1874 does not inform carriers, composition, grouping, or converse.

## Caveats

Several theorems are conditional on the finite-level Ramanujan–Arthur decomposition or on Lie-theoretic characteristic hypotheses. The Arthur enhancement explicitly refuses packet classification and multiplicities.

There is no complexity bound, no algorithm for Steel, and no Lean comparator. Importing this family into `proofs/Ano/` would be a new formalization project with no implementation consumer.
