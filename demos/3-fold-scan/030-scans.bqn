
route      ← 1‿0‿1‿1‿0‿1‿0‿1
weight     ← 3‿9‿1‿4‿8‿2‿7‿5
comboChain ← 1‿1‿0‿0‿1‿1‿0‿0
multiplier ← 2‿3‿9‿9‿1.5‿2‿9‿9
ray        ← 0‿1‿1‿1‿1‿1‿0‿0
height     ← 9‿3‿1‿4‿1‿5‿9‿9

! 3‿4‿8‿10‿15 ≡ +` route/weight
! 2‿6‿9‿18    ≡ ×` comboChain/multiplier # running product over 2 3 1.5 2
! 3‿3‿4‿4‿5   ≡ ⌈` ray/height

! (+´route) ≡ ≠ +` route/weight
! (+´comboChain) ≡ ≠ ×` comboChain/multiplier
! (+´ray) ≡ ≠ ⌈` ray/height
