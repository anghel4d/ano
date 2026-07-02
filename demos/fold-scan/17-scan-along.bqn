# Ex17 — Explicit scan source: name the ordered selection rather than deriving it inline.
# ano:  scan(+) Weight along pathCells
# Semantics: along supplies the order the scan is carried in — pathCells is an ordered selection, an index sequence in path order, not a mask; the scan gathers Weight in that order and accumulates.
# Invariant: the scan follows path order, not world index order — the prefixes differ from the mask spelling of the same cells, only the associative total agrees.

weight    ← 5‿1‿3‿9‿2‿8‿6‿4
pathCells ← 4‿2‿7‿1          # ordered: visit entity 4, then 2, then 7, then 1

! 2‿5‿9‿10 ≡ +` pathCells⊏weight        # running sum over 2 3 4 1, in path order

# the mask spelling loses the declared order: same cells, index order, different prefixes
mask ← 0‿1‿1‿0‿1‿0‿0‿1
! 1‿4‿6‿10 ≡ +` mask/weight             # running sum over 1 3 2 4, in index order
! ¬ (+` pathCells⊏weight) ≡ +` mask/weight

# the fold's total is order-independent; the scan's prefixes are not
! 10 ≡ +´ pathCells⊏weight
! 10 ≡ +´ mask/weight
