# ano:  |\ Burning @ Path
# Semantics: |\ accumulates OR along the scope's declared order (index order here), one running value per selected cell — the ever-any latch: 0 until the first burning cell, 1 from it on.
# Invariant: the scan equals the hand-computed latch, is monotone non-decreasing (once on, never off), and has the length of its selection; the |/ fold is the latch's last value.

path    ← 1‿1‿1‿1‿0‿1‿1‿0
burning ← 0‿0‿1‿0‿1‿0‿0‿1     # rows 4 and 7 are decoy 1s behind !path

! 0‿0‿1‿1‿1‿1 ≡ ∨` path/burning

# latch: monotone — a 1 is never followed by a 0
s ← ∨` path/burning
! s ≡ s ∨ »s

# same-length, and the fold is the scan's last element
! (+´path) ≡ ≠ s
! (∨´ path/burning) ≡ ¯1⊑s
