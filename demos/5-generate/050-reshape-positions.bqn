# Reshape to spatial positions (ex24): pour an ordered selection into a shaped layout.
# ano: Soldier , pos = to 8 8
# ano: Soldier , pos = to 4 _
# ano: Archer  , pos = to 20
# Semantics: the selection in key order receives the cells of the shape's index lattice in row-major order; _ infers the free axis from the selection count; reshape repositions existing entities and mints no keys.
# Invariant: written positions match the arithmetic form of the row-major lattice, non-selected rows are untouched, and the key set after equals the key set before.

# World: 70 entities, 64 soldiers, 6 bystanders scattered through the key range.
n ← 70
keys ← ↕n
civ ← 5‿17‿29‿41‿53‿65
soldier ← ¬ keys ∊ civ
! 64 = +´soldier
! 64 = ×´8‿8                       # the shape must hold the selection exactly

# to 8 8: the row-major cells of the 8x8 lattice.
cells ← ⥊ ↕8‿8
! cells ≡ {(⌊𝕩÷8)⋈8|𝕩}¨ ↕64        # arithmetic form: cell k = ⟨k div 8, k mod 8⟩

pos0 ← n ⥊ <¯1‿¯1                  # pre-state: sentinel position everywhere
pos ← cells⌾(soldier⊸/) pos0       # scatter under the mask, selection in key order

! (soldier/pos) ≡ cells            # soldiers hold the lattice in selection order
! ((¬soldier)/pos) ≡ 6⥊<¯1‿¯1      # bystanders untouched
! (20⊑pos) ≡ 2‿2                   # hand spot: key 20 is soldier ordinal 18 (civs 5,17 precede), cell ⟨2,2⟩
! n = ≠pos                         # reshape repositions, mints no keys

# to 4 _: the free axis is inferred from the selection count.
w ← 4 ÷˜ +´soldier
! w = 16
cells2 ← ⥊ ↕4‿16
! cells2 ≡ {(⌊𝕩÷16)⋈16|𝕩}¨ ↕64
pos2 ← cells2⌾(soldier⊸/) pos0
! (soldier/pos2) ≡ cells2          # soldiers hold the 4×16 lattice in selection order
! ((¬soldier)/pos2) ≡ 6⥊<¯1‿¯1     # bystanders untouched
! (20⊑pos2) ≡ 1‿2                  # same soldier, ordinal 18 = 16+2, cell ⟨1,2⟩

# to 20: one rank, the cell is the flat index.
m ← 25
archer ← ¬ (↕m) ∊ 5‿11‿17‿23‿24
! 20 = +´archer
apos0 ← m ⥊ ¯1
apos ← (↕20)⌾(archer⊸/) apos0
! (archer/apos) ≡ ↕20
! ((¬archer)/apos) ≡ 5⥊¯1
! 5 = 6⊑apos                       # hand spot: key 6 is archer ordinal 5 (civ 5 precedes)
