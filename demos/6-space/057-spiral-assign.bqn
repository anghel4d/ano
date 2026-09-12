
playerPos ← 100‿200

coin ← ⟨1,0,1,1,0,1,0,1⟩
pos  ← ⟨0‿0, 10‿10, 0‿0, 0‿0, 20‿20, 0‿0, 30‿30, 0‿0⟩

n ← +´coin
pairs ← (↕n) ⋈¨ 137.5×↕n
! 5 = ≠pairs
! 10 = ≠(↕n)∾137.5×↕n
! (⊑pairs)  ≡ 0‿0
! (1⊑pairs) ≡ 1‿137.5

d ← π÷180
Polar ← {r‿t: r×(•math.Cos t×d)⋈•math.Sin t×d}
spiral ← playerPos⊸+¨ Polar¨ pairs

pos ↩ spiral⌾((coin=1)⊸/) pos

! ⟨10‿10, 20‿20, 30‿30⟩ ≡ (¬coin)/pos

! (⊑pos) ≡ 100‿200
! ∧´ 1e¯9 > | (2⊑pos) - 100‿200+⟨¯0.737277336810124,  0.6755902076156604⟩
! ∧´ 1e¯9 > | (3⊑pos) - 100‿200+⟨0.17431148549531578, ¯1.992389396183491⟩
! ∧´ 1e¯9 > | (5⊑pos) - 100‿200+⟨1.826284287026163,   2.380060020873705⟩
! ∧´ 1e¯9 > | (7⊑pos) - 100‿200+⟨¯3.9392310120488325, ¯0.6945927106677191⟩
! 8 = ≠pos

entityDomain ← ↕8
selectedDomain ← /coin
! 8 = ≠entityDomain
! 5 = ≠selectedDomain
! 5 = ≠spiral
! 8 = ≠pos
