# Outer product comprehension (ex20): the comprehension is a theta-join, sigma_p(A x B).
# ano: [ t & c , +InRange | t <- Tower, c <- Creep, dist(t, c) < 50 ]
# ano: [ a & b , Collide  | a <- Body,  b <- Body,  a < b, overlap(a, b) ]
# Semantics: build the full cross product of the two generators, filter every pair by the predicate, apply the effect once per surviving pair; a self join dedups unordered pairs with a < b.
# Invariant: the filtered product matches the hand-computed pair list, and the effect lands with set semantics — each reached member tagged once, no multiplicity from in-degree.

# Towers and creeps as position columns, one ⟨x,y⟩ per entity.
tp ← ⟨0‿0, 200‿0, 50‿50⟩          # Tower positions
cp ← ⟨10‿10, 90‿10, 50‿80, 0‿49⟩  # Creep positions

# dist(t,c) < 50 as squared euclidean < 2500: integers stay exact.
d2 ← tp {+´×˜𝕨-𝕩}⌜ cp             # the product: 3x4 squared-distance table
m ← d2 < 2500                      # the predicate over the product
pairs ← (⥊m) / ⥊ (↕3) ⋈⌜ ↕4       # sigma_p(A x B): filter the pairs

# Hand-computed: d2(t0,c0)=200, d2(t0,c3)=2401, d2(t2,c2)=900 pass; all others ≥ 2500 (t2,c3 = 2501, just over).
! pairs ≡ ⟨0‿0, 0‿3, 2‿2⟩

# +InRange scatters once per reached member: a mask, not a count. t0 reaches two creeps, tagged once.
! (∨´˘ m) ≡ 1‿0‿1                  # towers gaining InRange
! (∨˝ m) ≡ 1‿0‿1‿1                # creeps gaining InRange

# Self join with a < b: bodies as ⟨lo,hi⟩ intervals, overlap(a,b) = lo_a < hi_b ∧ lo_b < hi_a.
bp ← ⟨0‿10, 5‿15, 20‿30, 28‿40⟩
ov ← bp {((⊑𝕨)<1⊑𝕩)∧(⊑𝕩)<1⊑𝕨}⌜ bp # 4x4 overlap table, symmetric, diagonal true
tri ← <⌜˜ ↕4                       # a < b: the strict upper triangle dedups unordered pairs
bpairs ← (⥊ov∧tri) / ⥊ (↕4) ⋈⌜ ↕4

# Hand-computed: b0∩b1 = [5,10), b2∩b3 = [28,30); every other pair disjoint.
! bpairs ≡ ⟨0‿1, 2‿3⟩

# Each unordered overlap appears twice off-diagonal, once under a < b.
! (+´⥊ov∧tri) = 2 ÷˜ +´⥊ ov ∧ ≠⌜˜↕4
