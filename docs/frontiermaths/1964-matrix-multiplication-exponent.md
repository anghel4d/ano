# 1964 — Matrix-multiplication exponent at most 9/4

openai/math family **107**. Card **1964**.

## What it says

The exponent ω is the infimum of numbers *c* such that two n×n matrices can be multiplied in O(n^{c+ε}) arithmetic operations for every ε > 0.

Over ℂ, ω ≤ 9/4. That is O_ε(n^{9/4+ε}) operations for square multiplication. In characteristic zero, some inner dimension n^a with a > 0.465 permits n^{2+o(1)} rectangular multiplication (the dual exponent α > 0.465). Further bounds: ω < 2.258 outside finitely many positive characteristics, and ω < 2.371054886006746 over every fixed field.

The model counts additions, subtractions, and multiplications in finite division-free arithmetic programs, with arbitrary positive exponent slack. It is arithmetic complexity, not bit complexity and not a measured crossover.

## Manuscripts

- Family entry: [CONTENTS.md §107](https://github.com/openai/math/blob/main/CONTENTS.md)
- [An Upper Bound of 9/4 for the Matrix Multiplication Exponent](https://github.com/openai/math/blob/main/preprints/Matrix-Multiplication-Nine-Fourths-October-2-2026/paper.pdf) — [`preprints/Matrix-Multiplication-Nine-Fourths-October-2-2026/`](https://github.com/openai/math/tree/main/preprints/Matrix-Multiplication-Nine-Fourths-October-2-2026)
- [Complex Matrix Multiplication Below 2.258 and Rectangular Bounds](https://github.com/openai/math/blob/main/preprints/Complex-Matrix-Multiplication-Below-2.258-and-Rectangular-Bounds-September-24-2026/Complex-Matrix-Multiplication-Below-2.258-and-Rectangular-Bounds-September-24-2026.pdf) — secondary writeup, [`…-September-24-2026/`](https://github.com/openai/math/tree/main/preprints/Complex-Matrix-Multiplication-Below-2.258-and-Rectangular-Bounds-September-24-2026)
- [Staggered extraction for exact matrix multiplication over every field](https://github.com/openai/math/blob/main/preprints/Staggered-extraction-for-exact-matrix-multiplication-over-every-field-September-24-2026/Staggered-extraction-for-exact-matrix-multiplication-over-every-field-September-24-2026.pdf) — [`…-September-24-2026/`](https://github.com/openai/math/tree/main/preprints/Staggered-extraction-for-exact-matrix-multiplication-over-every-field-September-24-2026)

## Lean status

Partial. Two comparators: [`lean/docs/107.md`](https://github.com/openai/math/blob/main/lean/docs/107.md).

Formalized: ω(ℂ) ≤ 9/4, α > 0.465, ω(ℂ; 1, 0.709, 1) < 2.092, and the every-field bound ω(F) < 2.371054886006746. No remaining numerical hypotheses. The 2.258 headline is the weaker corollary of the square bound.

## Tie to this repository

Steel is an array language that emits BQN and runs it on CBQN ([ano_jit.md](../ano_jit.md), [emit.rs](../../steel/src/emit.rs)). A future native backend is a design constraint, not an implemented kernel. World data are entity columns and one legacy lattice, not dense n×n GEMM ([ano-ecs.md](../ano-ecs.md)).

`cross` materializes a rank-2 *product domain* with pair lineage; it is not matrix multiplication, and world-column assignment of that product refuses ([ano-language.md](../ano-language.md) §16, [outer_product.rs](../../steel/tests/outer_product.rs)). Numeric folds are exact left accumulation on columns ([reducer.rs](../../steel/src/reducer.rs)).

Practical implication: **do not change lowering**. Schoolbook multiplication, or Strassen at sizes that actually appear, remains the only relevant algorithm if a backend ever multiplies dense blocks. The 9/4 exponent is an existence bound in the arithmetic model. It does not name an implementable routine, a crossover n, or a BQN idiom.

The rectangular dual exponent (α > 0.465) is the only fragment that could matter later: tall-skinny products that are almost linear in the large side. Even that is asymptotic-with-slack, and current worlds are tiny.

## Caveats — galactic constants

ω ≤ 9/4 is still a laser-method / tensor-powering bound. The hidden constants and the n₀ after which n^{9/4+ε} beats n^{2.37} or n^{2.81} are not practical. "Arbitrary positive exponent slack" means the O_ε hides a factor that explodes as ε → 0.

No note in this family gives a bit-complexity, a cache model, or a float64 error bound. Steel's `num` carrier is float64 with refused NaN ([ano-language.md](../ano-language.md), Numeric values). An arithmetic-circuit exponent over ℂ does not authorize reassociating those folds ([spatialmaths.md](../spatialmaths.md); [foundations.md](../../proofs/foundations.md)).

Do not cite 9/4 as a performance claim for CBQN, for a future JIT, or for `cross`.
