
weight    ← 5‿1‿3‿9‿2‿8‿6‿4
pathCells ← 4‿2‿7‿1

! 2‿5‿9‿10 ≡ +` pathCells⊏weight

mask ← 0‿1‿1‿0‿1‿0‿0‿1
! 1‿4‿6‿10 ≡ +` mask/weight
! ¬ (+` pathCells⊏weight) ≡ +` mask/weight

! 10 ≡ +´ pathCells⊏weight
! 10 ≡ +´ mask/weight
