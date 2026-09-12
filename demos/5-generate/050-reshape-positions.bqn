
n ← 70
keys ← ↕n
civ ← 5‿17‿29‿41‿53‿65
soldier ← ¬ keys ∊ civ
! 64 = +´soldier
! 64 = ×´8‿8

cells ← ⥊ ↕8‿8
! cells ≡ {(⌊𝕩÷8)⋈8|𝕩}¨ ↕64

pos0 ← n ⥊ <¯1‿¯1
pos ← cells⌾(soldier⊸/) pos0

! (soldier/pos) ≡ cells
! ((¬soldier)/pos) ≡ 6⥊<¯1‿¯1
! (20⊑pos) ≡ 2‿2
! n = ≠pos

w ← 4 ÷˜ +´soldier
! w = 16
cells2 ← ⥊ ↕4‿16
! cells2 ≡ {(⌊𝕩÷16)⋈16|𝕩}¨ ↕64
pos2 ← cells2⌾(soldier⊸/) pos0
! (soldier/pos2) ≡ cells2
! ((¬soldier)/pos2) ≡ 6⥊<¯1‿¯1
! (20⊑pos2) ≡ 1‿2

m ← 25
archer ← ¬ (↕m) ∊ 5‿11‿17‿23‿24
! 20 = +´archer
apos0 ← m ⥊ ¯1
apos ← (↕20)⌾(archer⊸/) apos0
! (archer/apos) ≡ ↕20
! ((¬archer)/apos) ≡ 5⥊¯1
! 5 = 6⊑apos
