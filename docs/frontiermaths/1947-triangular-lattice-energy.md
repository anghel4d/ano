# 1947 — Triangular-lattice energy and spherical logarithmic energy

openai/math family **090**. Card **1947**.

## What it says

Among locally finite planar point configurations of centered-disk density one, the triangular lattice minimizes the lower limit of energy per particle for every nonnegative completely monotone potential of squared distance. The comparison includes infinite energies.

The same lattice minimizes unit-background renormalized Riesz energies for 0 < s < 2 and Coulomb energy (Sandier–Serfaty). Combined with the known asymptotic, this gives the linear term of optimal ordered-pair logarithmic energy on the unit two-sphere (Brauchart–Hardin–Saff in 2-D). A companion Fourier certificate also attains the optimal planar circle-packing density π/(2√3) and recovers uniqueness of the triangular packing among periodic equality cases.

## Manuscripts

- Family entry: [CONTENTS.md §090](https://github.com/openai/math/blob/main/CONTENTS.md)
- [Universal optimality of the triangular lattice](https://github.com/openai/math/blob/main/preprints/Universal-optimality-of-the-triangular-lattice-September-23-2026/paper.pdf)
- [An atomic certificate for triangular-lattice universal optimality](https://github.com/openai/math/blob/main/preprints/An-atomic-certificate-for-triangular-lattice-universal-optimality-September-26-2026/paper.pdf)
- [A sharp Fourier certificate for planar circle packing](https://github.com/openai/math/blob/main/preprints/A-sharp-Fourier-certificate-for-planar-circle-packing-September-23-2026/paper.pdf)
- [Triangular minimality for planar Coulomb renormalized energy](https://github.com/openai/math/blob/main/preprints/Triangular-minimality-for-planar-Coulomb-renormalized-energy-September-23-2026/paper.pdf)

## Lean status

Partial. Four comparators: [`lean/docs/090.md`](https://github.com/openai/math/blob/main/lean/docs/090.md).

Formalized: universal energy minimality of the density-one triangular lattice for nonnegative completely monotone potentials of squared distance (infinite energies allowed); sharp Gaussian Fourier minorants from the atomic interpolation certificate; and a radial Schwartz certificate for the planar Cohn–Elkies bound π/(2√3). Periodic uniqueness of the packing equality case is outside the selected theorems. The Coulomb / spherical-log linear-term papers are in the family but not in these four comparators.

## Tie to this repository

Task 99 lists five prototype worlds: discrete 2-D and 3-D, continuous 2-D and 3-D, and a sphere ([todo/99](../../todo/99-spatial-lattice.md)). The Lean kernel already has lattice embeddings into affine frames and weighted interpolation support ([Affine.lean](../../proofs/Ano/Affine.lean), [Interpolation.lean](../../proofs/Ano/Interpolation.lean), [spatialmaths.md](../spatialmaths.md)). Foundations still list phyllotaxis distinctness/separation as unsettled concrete geometry ([foundations.md](../../proofs/foundations.md)).

Steel today has one unnamed rectangular `lattice w h` ([ano-ecs.md](../ano-ecs.md)). That is a product grid, not a triangular lattice, and it carries no energy.

Practical implication, possible future work: if a discrete 2-D habitat is ever chosen for packing, spawn clearance, or a Coulomb-like field, the triangular lattice is the energy-minimizing periodic arrangement for the potentials this family covers. The sphere result is the corresponding linear-term fact for logarithmic energy on S², which is the one compact prototype already named in task 99.

Practical implication, don't overread: the theorem does not pick Steel's habitat. Rectangular grids remain legal. Interpolation weights are algebraic (`WeightedSupport` sums to one); they are not Riesz or Coulomb energies. Exact-51 spawn needs support hits and resting poses, not a globally minimal point configuration ([todo/99](../../todo/99-spatial-lattice.md), Exact-51 phase order). Phyllotaxis is still a separate service obligation.

## Caveats

Universal optimality is for completely monotone potentials of *squared* distance at density one in the plane. Other potentials, other densities, and three-dimensional lattices are outside the statement.

The spherical claim is the linear term of the asymptotic, not a finite-n placement algorithm. Constants and n₀ from the asymptotic do not give Kore a spawn pattern.

A Fourier interpolation certificate is not Ano's `WeightedSupport`. Do not treat the packing uniqueness result as a uniqueness proof for registry layouts.
