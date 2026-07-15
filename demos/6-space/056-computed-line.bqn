# Computed line generation (ex28, spec §19). Version B canonical per the recurrence decision.
# Ano: 12 , offset = fib(index)
#         , spawn Cheese at Player.pos + (offset, 0)
#      12 , spawn Cheese at Player.pos + polar(index, index * 137.5)
#      8  , spawn Pillar at Player.pos + (index * 2, 0)
# Semantics: a bare shape n generates an anonymous line habitat distinct from the entity world; fib runs host-side because prev is a parallel shift, and spawn consumes the line through a source projection without growing or retyping it.
# Invariant: fib(index) matches the hand-computed Fibonacci list; one shift-add barrier step over a fresh offset column stays zero and differs from Fibonacci; spiral input is 12 index-angle pairs, not a flat 24-list; all spawned positions match hand-computed values.

playerPos ← 100‿200

# Version B: offset = fib(index), the recurrence inside the registered host function
fib ← 12↑{𝕩∾+´¯2↑𝕩}⍟10 0‿1
! fib ≡ ⟨0,1,1,2,3,5,8,13,21,34,55,89⟩

# Version A gloss: prev is a shift, not a carry; on a freshly minted line offset pre-state is zero,
# so one barrier step of the two-back stencil computes shift-and-add over zeros, never Fibonacci
offset ← 12⥊0
step ← (»offset)+»»offset
! step ≡ 12⥊0
! step ≢ fib

# cheese along the line: pos_i = Player.pos + (fib_i, 0)
cheesePos ← playerPos⊸+¨ fib ⋈¨ 0
! (⊑cheesePos)   ≡ 100‿200
! (1⊑cheesePos)  ≡ 101‿200
! (¯1⊑cheesePos) ≡ 189‿200

# phyllotaxis spiral: polar consumes index-angle PAIRS — zip with ⋈¨, never catenate with ∾
pairs ← (↕12) ⋈¨ 137.5×↕12
! 12 = ≠pairs
! (⊑pairs)  ≡ 0‿0
! (1⊑pairs) ≡ 1‿137.5
! 24 = ≠(↕12)∾137.5×↕12    # the old catenated form is a flat 24-list, not 12 pairs
d ← π÷180
Polar ← {r‿t: r×(•math.Cos t×d)⋈•math.Sin t×d}
spiralPos ← playerPos⊸+¨ Polar¨ pairs
! 12 = ≠spiralPos
! (⊑spiralPos) ≡ 100‿200                                          # r=0 sits on the player
! ∧´ 1e¯9 > | (1⊑spiralPos) - 100‿200+⟨¯0.737277336810124, 0.6755902076156604⟩
! ∧´ 1e¯9 > | (2⊑spiralPos) - 100‿200+⟨0.17431148549531578, ¯1.992389396183491⟩

# evenly spaced row: pos_i = Player.pos + (2i, 0)
pillarPos ← playerPos⊸+¨ (2×↕8) ⋈¨ 0
! pillarPos ≡ (100+2×↕8) ⋈¨ 200

# Domain witness: the generated line remains twelve rows while spawn creates 32 entity rows.
lineDomain ← ↕12
spawnSource ← lineDomain∾lineDomain∾↕8
! 12 = ≠lineDomain
! 12 = ≠fib
! 32 = ≠spawnSource
! 44 = 12 + ≠spawnSource
! lineDomain ≡ ↕12
