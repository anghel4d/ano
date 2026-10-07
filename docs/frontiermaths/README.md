# Frontier mathematics notes

These notes record which results from [openai/math](https://github.com/openai/math) (372 families, 722 manuscripts; [CONTENTS.md](https://github.com/openai/math/blob/main/CONTENTS.md)) have a concrete, honest connection to this repository.

The source treats the published results as holding. The notes do not re-prove them. They say what each result claims, where the manuscripts live, what Lean currently covers, and what that does or does not authorize in Steel, Kore, or the Lean spatial kernel.

Card numbers are the family IDs from the published card index (1861–2232). Family *k* in CONTENTS.md is not always card `1860+k`; each note gives both.

Most of the 372 families have no plausible tie to this code: number theory, algebraic geometry, von Neumann algebras, chromatic homotopy, and similar work stay out. Quality over quantity. Owner-priority families get dedicated notes even when the honest implication is "do not import this machinery."

## Notes

| Note | Why it is here |
|---|---|
| [Types as predicates](types-as-predicates.md) | Ano's carriers, ranges, presence, and selection masks already classify by property, not by Curry–Howard proofs; this is the design frame for 2100 and 2167. |
| [2100: weak ⇒ strong normalization in PTS](2100-weak-to-strong-normalization.md) | The only frontier result that speaks directly to a future Sky rewrite language. |
| [2167: Grothendieck homotopy hypothesis](2167-grothendieck-homotopy-hypothesis.md) | Types-as-spaces, the opposite tradition; a negative for importing ∞-groupoids into Ano. |
| [1964: matrix-multiplication exponent ≤ 9/4](1964-matrix-multiplication-exponent.md) | Array-backend complexity; galactic, do not change lowering. |
| [1966: integer multiplication below n log n](1966-integer-multiplication.md) | Bit-complexity of multiplication; κ = 2⁻¹⁸² is unusable. |
| [1874: Ramanujan–Arthur and generic Ramanujan](1874-ramanujan-arthur.md) | Automorphic temperedness, not expander graphs; do not confuse with neighborhood spectra. |
| [1947: triangular-lattice energy](1947-triangular-lattice-energy.md) | The only energy/packing theorem that names the lattices and sphere task 99 already lists. |
| [2011: nonperiodic tiles in dimension three](2011-periodic-tiling.md) | A don't-bother bound on assuming every 3-D lattice pattern is periodic. |
