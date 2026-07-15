# Coordinate lattice patterns (ex27, spec §18).
# Ano: 8 8 & (x + y) % 2 == 0 , spawn Wheat
#      8 8 & (x + y) % 2 == 1 , spawn Barley
#      8 8 & x == y , spawn Pillar
#      8 8 & x + y < 8 , +Buildable
#      8 8 & x % 3 == 0 , spawn Fence
# Semantics: this witness constructs one anonymous 8×8 habitat carrying x and y; every mask remains a field-shaped value on that habitat. A future stored effect must reach a named field through explicit lineage rather than matching 64 rows.
# Invariant: every mask equals its outer-product form and its hand-computed count; wheat and barley partition the board; the diagonal is the flat indices 9×↕8.

# world as columns: 64 cells, x is the row coordinate, y the column, flat index 8x+y
x ← ⌊(↕64)÷8
y ← 8|↕64

wheat     ← 0=2|x+y
barley    ← 1=2|x+y
pillar    ← x=y
buildable ← 8>x+y
fence     ← 0=3|x

# masks match the spec's outer-product verification forms
! (8‿8⥊wheat)     ≡ 0=2|+⌜˜↕8
! (8‿8⥊barley)    ≡ 1=2|+⌜˜↕8
! (8‿8⥊pillar)    ≡ =⌜˜↕8
! (8‿8⥊buildable) ≡ 8>+⌜˜↕8
! (8‿8⥊fence)     ≡ (0=3|↕8)×⌜8⥊1

# checkerboards partition the board
! ∧´wheat≠barley
! 32 = +´wheat
! 32 = +´barley

# diagonal: 8 cells at flat indices 9×↕8
! 8 = +´pillar
! (9×↕8) ≡ /pillar

# triangle: 8+7+…+1 = 36 buildable cells
! 36 = +´buildable

# stripes: ranks 0 3 6, 24 cells at flat indices 0..7, 24..31, 48..55
! 24 = +´fence
! (∾⟨↕8, 24+↕8, 48+↕8⟩) ≡ /fence

# Habitat witness: every mask is a field-shaped value on the same 8×8 domain.
shape ← 8‿8
ground ← ↕shape
! shape ≡ ≢ground
! shape ≡ ≢8‿8⥊wheat
! shape ≡ ≢8‿8⥊barley
! shape ≡ ≢8‿8⥊pillar
! shape ≡ ≢8‿8⥊buildable
! shape ≡ ≢8‿8⥊fence
