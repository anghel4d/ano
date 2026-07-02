# ex18 - Grade and rank
# ano: Unit , Slot = rank(Initiative)
#      top 5 (grade desc Threat) , +Targeted
# semantics: dense value-only rank of the Initiative column within the Unit selection, scattered back aligned (Tier-2 write, non-Unit rows untouched); the second statement grades the whole world by Threat descending, takes the five highest, and sets their Targeted mask.
# invariant: ranks land only on Unit rows and are a function of the value alone (§15: tied values share a rank; ordinal rank would split them by position and break Sym(I)-equivariance); +Targeted is a mask-or, so prior Targeted rows survive and exactly the top-5 threat rows join them.

# world: 8 entities, columns
unit       ← 1‿1‿0‿1‿1‿0‿1‿1
initiative ← 12‿7‿99‿7‿15‿50‿3‿9    # rows 1 and 3 tie at 7; rows 2,5 are not units
threat     ← 30‿80‿55‿10‿70‿90‿20‿60
slot       ← 8⥊¯1                    # ¯1 marks the never-written slot
targeted   ← 0‿0‿0‿1‿0‿0‿0‿0        # row 3 pre-targeted, threat 10, not in the top 5

# Unit , Slot = rank(Initiative): dense value-only rank of the filtered column, written back under the mask
slot ↩ ((∧⍷unit/initiative)⊐unit/initiative)˙⌾(unit⊸/) slot

# top 5 (grade desc Threat) , +Targeted: membership of the top-5 grade indices, or-ed in
targeted ∨↩ (↕≠threat) ∊ 5↑⍒threat

# expected post-state, by hand:
# unit rows 0,1,3,4,6,7 carry initiative 12,7,7,15,3,9 -> dense ranks 3,1,1,4,0,2 (the tied 7s share rank 1)
! 3‿1‿¯1‿1‿4‿¯1‿0‿2 ≡ slot
# top-5 threat: 90(r5) 80(r1) 70(r4) 60(r7) 55(r2); row 3 keeps its prior mark
! 0‿1‿1‿1‿1‿1‿0‿1 ≡ targeted
# the tie is value-only: rows 1 and 3 share the rank
! (slot⊑˜1) = slot⊑˜3
