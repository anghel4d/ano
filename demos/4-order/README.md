# order

Grade, rank, and ordered top-k: examples 18, 19, 26, 31 and the rank-tie question from spec §15 against the Tier-2 write-back rule (the old s13- prefix, kept as a (was …) header marker, keys to the pre-renumbering spec section).

- `041-grade-and-rank.bqn` — ex18: dense value-only rank of Initiative written back under the Unit mask, top-5 threat by grade-desc membership; checks value-only ties (the tied pair shares a rank) and mask-or.
- `042-pipeline-topk.bqn` — ex19: top-5 enemies by threat; demos the doc bug (filtered-space grade indices or-ed raw into a world column) against the fix (map through /enemy, then membership), asserts they differ.
- `043-fold-ordered-topk.bqn` — ex26 Version B: sum threat over the ten highest-dps enemies; demos the doc bug (grade of filtered dps indexing unfiltered threat) against the fix (filtered indexes filtered), asserts they differ.
- `044-spatial-topk.bqn` — ex31 at 8x8: grade a per-cell column descending, spawn into the top 8 / mask the top 5; checks stable tie order on the lattice.
- `045-rank-ties.bqn` — stable ordinal rank (⍋⍋, the retired "ties are stable" reading) vs dense value-only rank ((∧⍷v)⊐v) on gold 5 5 3, the blessed write-back Unit , Rank = rank(Gold) under both, and the Sym(I)-equivariance check (§15) that ordinal rank fails on ties and dense rank passes.
