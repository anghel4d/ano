# The glider, acted out (c2): one statement is one generation, the statement log is the flipbook.
# Ano: def kin = #/ (moore' & Planted)
#      #/ (Planted)                                     -- console peek: a glider carries 5 cells forever
#      Plot , Planted = kin == 3 | Planted & kin == 2   -- repeated: one barrier per generation
# Semantics: performed, each repetition of the line advances one generation (§10, one barrier per statement); installed with the arrow the same line runs one generation per tick (§11) — the hinge alone changes the schedule.
# Invariant: generations 1-3 match the hand-stepped truth cell by cell; generation 4 is the seed translated one cell down-right; population holds at 5 and every generation remains on the same 8×8 field habitat.

S ← {(»𝕩)+(«𝕩)+(»˘𝕩)+(«˘𝕩)+(»»˘𝕩)+(»«˘𝕩)+(«»˘𝕩)+(««˘𝕩)}   # Moore counts, dead border
Life ← {n ← S 𝕩 ⋄ (3=n) ∨ 𝕩 ∧ 2=n}

p0 ← 8‿8⥊(0‿1‿0‿0‿0‿0‿0‿0‿ 0‿0‿1‿0‿0‿0‿0‿0‿ 1‿1‿1‿0‿0‿0‿0‿0) ∾ 40⥊0
p1 ← Life p0 ⋄ p2 ← Life p1 ⋄ p3 ← Life p2 ⋄ p4 ← Life p3
! p1 ≡ 8‿8⥊(0‿0‿0‿0‿0‿0‿0‿0‿ 1‿0‿1‿0‿0‿0‿0‿0‿ 0‿1‿1‿0‿0‿0‿0‿0‿ 0‿1‿0‿0‿0‿0‿0‿0) ∾ 32⥊0
! p2 ≡ 8‿8⥊(0‿0‿0‿0‿0‿0‿0‿0‿ 0‿0‿1‿0‿0‿0‿0‿0‿ 1‿0‿1‿0‿0‿0‿0‿0‿ 0‿1‿1‿0‿0‿0‿0‿0) ∾ 32⥊0
! p3 ≡ 8‿8⥊(0‿0‿0‿0‿0‿0‿0‿0‿ 0‿1‿0‿0‿0‿0‿0‿0‿ 0‿0‿1‿1‿0‿0‿0‿0‿ 0‿1‿1‿0‿0‿0‿0‿0) ∾ 32⥊0
! p4 ≡ » »˘ p0                       # period 4: the seed, one cell down-right
! p4 ≢ p0                            # it did move
! 5‿5‿5‿5‿5 ≡ +´∘⥊¨ ⟨p0, p1, p2, p3, p4⟩

# Every generation remains a field on the original 8×8 habitat.
! ∧´ 8‿8⊸≡¨ ≢¨ ⟨p0,p1,p2,p3,p4⟩
