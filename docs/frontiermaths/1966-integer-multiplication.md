# 1966 — Integer multiplication in O(n (log n)^{1−κ}), κ = 2⁻¹⁸²

openai/math family **109**. Card **1966**.

## What it says

Two n-bit integers can be multiplied exactly, at every input length, in deterministic worst-case time O(n (log n)^{1−κ}) with κ = 2⁻¹⁸², on one fixed finite-alphabet Turing machine with finitely many one-dimensional tapes.

This is strictly below the Schönhage–Strassen n log n barrier in the ordinary multitape bit model. The algorithm is uniform: one machine, every n.

## Manuscripts

- Family entry: [CONTENTS.md §109](https://github.com/openai/math/blob/main/CONTENTS.md)
- [Integer multiplication below n log n](https://github.com/openai/math/blob/main/preprints/Integer-multiplication-below-n-log-n-September-23-2026/paper.pdf) — folder [`preprints/Integer-multiplication-below-n-log-n-September-23-2026/`](https://github.com/openai/math/tree/main/preprints/Integer-multiplication-below-n-log-n-September-23-2026)

## Lean status

None.

## Tie to this repository

Ano's numeric work is float64, not big-integer bit complexity. [num.rs](../../steel/src/num.rs) is C-locale `strtod` and printf-faithful formatting: the choke point every number crosses on load, save, and emit. Carriers are `num` / `nat` / `int` with `nat`/`int` refined to a finite float64 integer range ([lib.rs](../../steel/src/lib.rs) `ColType`; [ano-language.md](../ano-language.md), Numeric values). Unique keys and relationship endpoints live in that same representation ([relationship.rs](../../steel/src/relationship.rs)).

There is no bigint library, no FFT multiplier, and no multitape TM backend. A future JIT still targets the bytecode/VM contract and the accepted language semantics ([ano_jit.md](../ano_jit.md)). Integer multiplication of n-bit strings does not appear in emit, folds, or Kore.

Practical implication: **do not bother**. Do not replace `f64` arithmetic, do not add a Schönhage–Strassen or Harvey–van der Hoeven multiplier, and do not treat this as a reason to change `num.rs`. If a host later needs exact large integers, use an ordinary library; this paper is not that library.

The only honest citation is negative: the n log n conjecture is false in the multitape model, so a complexity remark that "integer multiplication is Θ(n log n)" is outdated. That remark is not one this repository currently makes.

## Caveats — galactic constants

κ = 2⁻¹⁸² is about 2×10⁻⁵⁵. Write the time as O(n log n / (log n)^κ). The saving factor is (log n)^κ = exp(κ ln log n) ≈ 1 + κ ln log n until κ ln log n is no longer tiny.

For κ ln log n to reach even 0.01 (a 1% saving in the log factor), one needs ln log n ≈ 0.01 × 2¹⁸², so log n is exp(Θ(2¹⁸²)) and n is exp(exp(Θ(2¹⁸²))). Every integer that fits in memory, or in the universe, sees (log n)^κ = 1 for all practical purposes.

The model is a multitape Turing machine, not a RAM, not a word machine, and not IEEE arithmetic. Constants hidden by the O(·) are unspecified. The result is a disproof of an optimality conjecture, not a candidate implementation.

Harvey–van der Hoeven n log n multiplication is already unused here and is the realistic "fast integer multiply" citation if one is ever needed. 1966 sits strictly beyond that in theory and strictly behind it in practice.
