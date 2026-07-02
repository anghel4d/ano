# Ex16 — Scans: accumulate along the current ordered view and return a same-length column.
# ano:  +\ Weight @ (↕steps |> route A B)
#       *\ Multiplier @ comboChain
#       max\ Height @ (Eye + ↕n * fwd)
# Semantics: scan folds along the scope's declared order — these views are generated lines, so their order is index order — and yields one column aligned to the scope, one running value per selected cell.
# Invariant: each scan equals its hand-computed prefix column and has the length of its selection.

route      ← 1‿0‿1‿1‿0‿1‿0‿1
weight     ← 3‿9‿1‿4‿8‿2‿7‿5
comboChain ← 1‿1‿0‿0‿1‿1‿0‿0
multiplier ← 2‿3‿9‿9‿1.5‿2‿9‿9
ray        ← 0‿1‿1‿1‿1‿1‿0‿0
height     ← 9‿3‿1‿4‿1‿5‿9‿9

! 3‿4‿8‿10‿15 ≡ +` route/weight         # running sum over 3 1 4 2 5
! 2‿6‿9‿18    ≡ ×` comboChain/multiplier # running product over 2 3 1.5 2
! 3‿3‿4‿4‿5   ≡ ⌈` ray/height           # running max over 3 1 4 1 5

# same-length: one running value per selected cell
! (+´route) ≡ ≠ +` route/weight
! (+´comboChain) ≡ ≠ ×` comboChain/multiplier
! (+´ray) ≡ ≠ ⌈` ray/height
