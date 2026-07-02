# 35 - Two habitats table: the same calculus over ordered entity vectors (rank 1) and spatial lattices (rank 2)
# ano: generator   (↕n)                          (↕ w‿h)
# ano: reduce      +/ Gold @ Nord                +/ Elevation @ 64 64
# ano: scan        +\ Damage @ graded            +\ Cost @ 64 64
# ano: grade       top 5 (grade Threat)          top 8 (grade Safety @ 64 64)
# ano: outer       [f | a<-A, b<-B]              64 64 & (x+y)%2==0
# ano: replicate   spawn Minion * Count          spawn Tree * Density
# ano: reshape     to Unit                       to Soldier 4 _
# ano: shift       prev.X (shift »)              neighbor.X (shift «/»)     -- decision record: row renamed from recurrence; prev is a shift, not a carry
# ano: def         def threat = Dmg*Spd/Rng      def ridge = sin(x/8)+sin(y/8)
# Semantics: every row of the table is one array operation read twice, over a first-order key (entities) and over a second-order cell (space); only the rank of the index changes.
# Invariant: each row computed in both habitats matches its hand-computed expected value.

# generator: the ground is minted by ↕, rank 1 and rank 2
! (↕5) ≡ 0‿1‿2‿3‿4
! (↕2‿3) ≡ 2‿3⥊⟨0‿0, 0‿1, 0‿2, 1‿0, 1‿1, 1‿2⟩

# reduce: scoped fold to one scalar
gold ← 7‿12‿5‿30‿9 ⋄ nord ← 1‿0‿1‿1‿0
! 42 ≡ +´nord/gold
elev ← 2‿3⥊1‿2‿3‿4‿5‿6
! 21 ≡ +´⥊elev

# scan: entities scan along the graded view, space scans in lattice order
dmg ← 4‿1‿3‿2‿5
! 1‿3‿6‿10‿15 ≡ +`(⍋dmg)⊏dmg
cost ← 2‿3⥊1
! (2‿3⥊1‿1‿1‿2‿2‿2) ≡ +`cost

# grade: top k by grading down
threat ← 10‿50‿20‿40‿30
! 1‿3‿4 ≡ 3↑⍒threat
safety ← 2‿3⥊3‿1‿4‿1‿5‿9
! 5‿4 ≡ 2↑⍒⥊safety

# outer: comprehension over two axes; the checkerboard is the same table with a parity predicate
a ← 1‿2‿3 ⋄ b ← 10‿20
! (3‿2⥊11‿21‿12‿22‿13‿23) ≡ a+⌜b
! (>⟨1‿0‿1‿0, 0‿1‿0‿1, 1‿0‿1‿0, 0‿1‿0‿1⟩) ≡ 0=2|+⌜˜↕4

# replicate: one source id per spawned copy
count ← 2‿0‿1‿3
! 0‿0‿2‿3‿3‿3 ≡ count/↕4
density ← 1‿0‿2‿1
! 0‿2‿2‿3 ≡ density/↕4

# reshape: a vector of soldiers into 4 rows, the trailing axis computed
! (4‿2⥊↕8) ≡ 4‿∘⥊↕8

# shift: prev is a parallel shift with fill, never an order-carried recurrence (decision record)
x ← 5‿8‿13‿21‿34
! 0‿5‿8‿13‿21 ≡ »x
g ← 2‿3⥊1‿2‿3‿4‿5‿6
! (2‿3⥊0‿0‿0‿1‿2‿3) ≡ »g

# def: a derived column and a derived field; ridge modeled with exact arithmetic (x+y) in place of sin
dmgC ← 10‿6‿8 ⋄ spd ← 3‿5‿2 ⋄ rng ← 5‿3‿4
! 6‿10‿4 ≡ dmgC×spd÷rng
! (>⟨0‿1‿2‿3, 1‿2‿3‿4, 2‿3‿4‿5, 3‿4‿5‿6⟩) ≡ +⌜˜↕4
