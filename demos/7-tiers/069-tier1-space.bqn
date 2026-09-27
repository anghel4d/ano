
ground ← ↕8‿8
wheat ← 8‿8⥊0
elevation ← 8‿8⥊↕64
cost ← 8‿8⥊1

mask ← 0=2|+⌜˜↕8
wheat ↩ wheat∨mask
cb ← >⟨1‿0‿1‿0‿1‿0‿1‿0
       0‿1‿0‿1‿0‿1‿0‿1
       1‿0‿1‿0‿1‿0‿1‿0
       0‿1‿0‿1‿0‿1‿0‿1
       1‿0‿1‿0‿1‿0‿1‿0
       0‿1‿0‿1‿0‿1‿0‿1
       1‿0‿1‿0‿1‿0‿1‿0
       0‿1‿0‿1‿0‿1‿0‿1⟩
! cb ≡ wheat

! 2016 ≡ +´⥊elevation
! (8‿8) ≡ ≢elevation

! ((1+↕8)+⌜8⥊0) ≡ +`cost

visible ← (¬⥊mask)/⥊elevation
! 32 = ≠visible
! 1008 = +´visible
! 8‿8 ≡ ≢elevation
! elevation ≡ 8‿8⥊↕64

! ground ≡ ↕8‿8
! (×´≢elevation) ≠ ≠visible
