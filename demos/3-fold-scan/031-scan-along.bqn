# ano: scan(+) / scan(-) / scan(min) / scan(avg) Weight along pathCells
# Semantics: the vector binding is an ordered view, not a mask. Every head consumes 4,2,7,1 in that order; subtraction demonstrates why Steel may not regroup it.

weight    ← 5‿1‿3‿9‿2‿8‿6‿4
pathCells ← 4‿2‿7‿1
x ← pathCells⊏weight

! 2‿5‿9‿10 ≡ +`x
! 2‿¯1‿¯5‿¯6 ≡ -`x
! 2‿2‿2‿1 ≡ ⌊`x
! 2‿2.5‿3‿2.5 ≡ (+`x)÷+`1¨x
! ¯6 ≡ ¯1⊑-`x
! 2 ≡ -´x
! ¬ (¯1⊑-`x) ≡ -´x

mask ← 0‿1‿1‿0‿1‿0‿0‿1
! 1‿4‿6‿10 ≡ +`mask/weight
! 10 ≡ +˜´⌽(0∾mask/weight)
! ¬ (+`x) ≡ +`mask/weight
