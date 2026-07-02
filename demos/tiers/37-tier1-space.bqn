# 37 - Tier 1 space: generated ground, projection, ordered scan
# ano: 8 8 & (x + y) % 2 == 0 , spawn Wheat
# ano: +/ Elevation @ 64 64
# ano: +\ Cost @ 64 64
# Semantics: space is a conserved ground minted by ↕; spawn inscribes a figure on it, a fold projects a field to a scalar without consuming it, a scan runs in the lattice's leading-axis order. Fields scaled to 8‿8 for hand-checking.
# Invariant: post-state equals the hand-computed checkerboard, sum, and running rows; annihilating a figure leaves the ground regenerable and the other fields untouched.

# world: fields over one 8‿8 ground, minted once
ground ← ↕8‿8
wheat ← 8‿8⥊0
elevation ← 8‿8⥊↕64
cost ← 8‿8⥊1

# statement 1: spawn Wheat on the checkerboard figure
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

# statement 2: project elevation to a scalar; the field survives as projection
! 2016 ≡ +´⥊elevation        # 0+1+…+63 = 63×64÷2
! (8‿8) ≡ ≢elevation

# statement 3: scan cost in lattice order (leading axis); all-ones cost gives running row totals
! ((1+↕8)+⌜8⥊0) ≡ +`cost

# annihilate: drop the wheat cells from the elevation figure; positions aren't owed, the ground rebuilds from ↕
! 1008 ≡ +´(¬⥊mask)/⥊elevation   # each parity class carries half of 2016
! ground ≡ ↕8‿8                   # every write re-valued a field; the ground minted at the top is what ↕ remints now
