# ano: scan(+) / scan(min) / scan(-) Weight along pathCells 4 2 7 1, beside the row-order masked scans.
# Semantics: `along` gathers the column through a declared traversal order, then runs the one left recurrence over that order — the same descriptor the glyph scan uses over row order.
# Invariant: the along scans read weight[4 2 7 1] = 2 3 4 1; the fold is the last prefix.

weight ← 5‿1‿3‿9‿2‿8‿6‿4
path ← 4‿2‿7‿1
walked ← path⊏weight
! 2‿3‿4‿1 ≡ walked
! 2‿5‿9‿10 ≡ +` walked
! 2‿2‿2‿1 ≡ ⌊` walked
! 2‿¯1‿¯5‿¯6 ≡ -` walked
mask ← 0‿1‿1‿0‿1‿0‿0‿1
! 1‿4‿6‿10 ≡ +` mask/weight
! 10 ≡ ¯1⊑+` mask/weight                   # the fold is the last prefix
