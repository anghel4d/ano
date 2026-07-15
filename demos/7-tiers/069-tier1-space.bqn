# 37 - Stored-field preservation and the regenerability counterexample
# ano: 8 8 & (x + y) % 2 == 0 , spawn Wheat
# ano: +/ Elevation @ 64 64
# ano: +\ Cost @ 64 64
# Semantics: the 8×8 habitat is fixed; masks and folds create derived values, while stored fields retain their habitat and shape. Reminting coordinates does not reconstruct values dropped by a filter.
# Invariant: post-state equals the hand-computed checkerboard, sum, and running rows; the filtered elevation is a 32-row derived view and is never assigned back to the 8×8 field.

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

# Filtering creates a derived subdomain. It does not replace or resize the stored elevation field.
visible ← (¬⥊mask)/⥊elevation
! 32 = ≠visible
! 1008 = +´visible
! 8‿8 ≡ ≢elevation
! elevation ≡ 8‿8⥊↕64

# Coordinates remint, values do not: ground reconstructs addresses only, while the 32-value view has no inverse back to elevation.
! ground ≡ ↕8‿8
! (×´≢elevation) ≠ ≠visible
