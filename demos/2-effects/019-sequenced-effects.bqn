# ano: ^cursor , Knockback 5 ; Flash :Red ; -Shielded, followed by a bare cursor effect
# Semantics: every gather and effect in one statement sees one frozen (world, overlay, host-input) triple. A host rebind becomes visible only at the next statement barrier, and the bare binding is independent.

sNone‿sRed ← 0‿1

liveCursor ← 0‿0‿1‿0‿0‿0
bareCursor ← 0‿0‿0‿0‿0‿1
pos        ← 0‿10‿20‿30‿40‿50
flash      ← sNone¨↕6
shielded   ← 1‿1‿1‿0‿1‿0
nord       ← 1‿1‿0‿1‿1‿0
twoHanded  ← 70‿50‿90‿61‿10‿99
gold       ← 5‿10‿15‿20‿25‿30
blessed    ← 0‿1‿0‿0‿0‿0

frozen ← liveCursor
pos +↩ 5×frozen
flash ↩ (sRed¨)⌾(frozen⊸/) flash
shielded ∧↩ ¬frozen

! pos      ≡ 0‿10‿25‿30‿40‿50
! flash    ≡ sNone‿sNone‿sRed‿sNone‿sNone‿sNone
! shielded ≡ 1‿1‿0‿0‿1‿0

sel ← nord ∧ twoHanded>60
gold +↩ 1000×sel
blessed ∨↩ sel
blessed ∨↩ bareCursor

! gold    ≡ 1005‿10‿15‿1020‿25‿30
! blessed ≡ 1‿1‿0‿1‿0‿1
! liveCursor ≡ frozen
! bareCursor ≡ 0‿0‿0‿0‿0‿1
