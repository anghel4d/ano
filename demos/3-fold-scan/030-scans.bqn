# ano: +\, *\, min\, avg\, and #\ over declared selections
# Semantics: homogeneous scans emit each left accumulator. Average carries (sum,count) and projects each state; count consumes selection presence and advances only on true rows.

route      ← 1‿0‿1‿1‿0‿1‿0‿1
weight     ← 3‿9‿1‿4‿8‿2‿7‿5
comboChain ← 1‿1‿0‿0‿1‿1‿0‿0
multiplier ← 2‿3‿9‿9‿1.5‿2‿9‿9
ray        ← 0‿1‿1‿1‿1‿1‿0‿0
height     ← 9‿3‿1‿4‿1‿5‿9‿9

sumPrefixes ← +` route/weight
prodPrefixes ← ×` comboChain/multiplier
minPrefixes ← ⌊` ray/height
meanPrefixes ← (+`route/weight)÷+`1¨route/weight
countPrefixes ← +`route

! 3‿4‿8‿10‿15 ≡ sumPrefixes
! 2‿6‿9‿18 ≡ prodPrefixes
! 3‿1‿1‿1‿1 ≡ minPrefixes
! 3‿2‿(8÷3)‿2.5‿3 ≡ meanPrefixes
! 1‿1‿2‿3‿3‿4‿4‿5 ≡ countPrefixes
! (¯1⊑sumPrefixes) ≡ +˜´⌽(0∾route/weight)
