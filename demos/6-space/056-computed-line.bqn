
playerPos ← 100‿200

fib ← 12↑{𝕩∾+´¯2↑𝕩}⍟10 0‿1
! fib ≡ ⟨0,1,1,2,3,5,8,13,21,34,55,89⟩

offset ← 12⥊0
step ← (»offset)+»»offset
! step ≡ 12⥊0
! step ≢ fib

cheesePos ← playerPos⊸+¨ fib ⋈¨ 0
! (⊑cheesePos)   ≡ 100‿200
! (1⊑cheesePos)  ≡ 101‿200
! (¯1⊑cheesePos) ≡ 189‿200

pairs ← (↕12) ⋈¨ 137.5×↕12
! 12 = ≠pairs
! (⊑pairs)  ≡ 0‿0
! (1⊑pairs) ≡ 1‿137.5
! 24 = ≠(↕12)∾137.5×↕12
d ← π÷180
Polar ← {r‿t: r×(•math.Cos t×d)⋈•math.Sin t×d}
spiralPos ← playerPos⊸+¨ Polar¨ pairs
! 12 = ≠spiralPos
! (⊑spiralPos) ≡ 100‿200
! ∧´ 1e¯9 > | (1⊑spiralPos) - 100‿200+⟨¯0.737277336810124, 0.6755902076156604⟩
! ∧´ 1e¯9 > | (2⊑spiralPos) - 100‿200+⟨0.17431148549531578, ¯1.992389396183491⟩

pillarPos ← playerPos⊸+¨ (2×↕8) ⋈¨ 0
! pillarPos ≡ (100+2×↕8) ⋈¨ 200

lineDomain ← ↕12
spawnSource ← lineDomain∾lineDomain∾↕8
! 12 = ≠lineDomain
! 12 = ≠fib
! 32 = ≠spawnSource
! 44 = 12 + ≠spawnSource
! lineDomain ≡ ↕12
