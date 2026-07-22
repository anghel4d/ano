
sNone‿sRed ← 0‿1

cursor    ← 0‿0‿1‿0‿0‿0
pos       ← 0‿10‿20‿30‿40‿50
flash     ← sNone¨↕6
shielded  ← 1‿1‿1‿0‿1‿0
nord      ← 1‿1‿0‿1‿1‿0
twoHanded ← 70‿50‿90‿61‿10‿99
gold      ← 5‿10‿15‿20‿25‿30
blessed   ← 0‿1‿0‿0‿0‿0

pos0‿flash0‿shielded0‿gold0‿blessed0 ← pos‿flash‿shielded‿gold‿blessed

sel ← cursor
pos +↩ 5×sel
flash ↩ (sRed¨)⌾(sel⊸/) flash
shielded ∧↩ ¬sel

! pos      ≡ 0‿10‿25‿30‿40‿50
! flash    ≡ sNone‿sNone‿sRed‿sNone‿sNone‿sNone
! shielded ≡ 1‿1‿0‿0‿1‿0

shieldedR ← shielded0 ∧ ¬sel
flashR    ← (sRed¨)⌾(sel⊸/) flash0
posR      ← pos0 + 5×sel
! pos‿flash‿shielded ≡ posR‿flashR‿shieldedR

sel ↩ nord ∧ twoHanded>60
gold +↩ 1000×sel
blessed ∨↩ sel

! sel     ≡ 1‿0‿0‿1‿0‿0
! gold    ≡ 1005‿10‿15‿1020‿25‿30
! blessed ≡ 1‿1‿0‿1‿0‿0

! ((¬sel)/gold)    ≡ (¬sel)/gold0
! ((¬sel)/blessed) ≡ (¬sel)/blessed0
