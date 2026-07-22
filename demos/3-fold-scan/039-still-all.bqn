# ano:  &\ Alive @ MarchOrder
# Semantics: &\ accumulates AND along the scope's declared order (index order here), one running value per selected cell — the still-all latch: 1 while every soldier so far is alive, 0 from the first casualty on.
# Invariant: the scan equals the hand-computed latch, is monotone non-increasing (once off, never on), and has the length of its selection; the &/ fold is the latch's last value.

marchOrder ← 1‿1‿0‿1‿1‿1‿0‿1
alive      ← 1‿1‿0‿1‿0‿1‿0‿1  # rows 2 and 6 are decoy 0s behind !marchOrder

! 1‿1‿1‿0‿0‿0 ≡ ∧` marchOrder/alive

# latch: monotone — a 0 is never followed by a 1
s ← ∧` marchOrder/alive
! s ≡ s ∧ 1»s

# same-length, and the fold is the scan's last element
! (+´marchOrder) ≡ ≠ s
! (∧´ marchOrder/alive) ≡ ¯1⊑s
