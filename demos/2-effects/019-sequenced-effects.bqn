# ano: ^cursor , Knockback 5 ; Flash :Red ; -Shielded — then ^cursor , Gold += 100 — then !^cursor , Blessed = 1
# Semantics: one submission freezes one overlay snapshot, so both sigiled statements read the same target mask; the effects sequence left to right behind it; negation applies to the resolved mask.
# Invariant: rows outside the frozen selection never move, and the blessing is exactly its complement.

sNone‿sRed ← 0‿1
sel       ← 0‿0‿1‿0‿0‿0                    # the sidecar's cursor mask, frozen for the whole run

pos       ← 0‿10‿20‿30‿40‿50
tint      ← sNone¨↕6
shielded  ← 1‿1‿1‿0‿1‿0
gold      ← 5‿10‿15‿20‿25‿30
blessed   ← 0‿1‿0‿0‿0‿0
pos0‿gold0 ← pos‿gold

pos +↩ 5×sel
tint ↩ (sRed¨)⌾(sel⊸/) tint
shielded ∧↩ ¬sel
gold +↩ 100×sel                            # the second statement reads the SAME sel
blessed ↩ blessed∨¬sel                     # and the third reads its complement

! pos      ≡ 0‿10‿25‿30‿40‿50
! tint     ≡ sNone‿sNone‿sRed‿sNone‿sNone‿sNone
! shielded ≡ 1‿1‿0‿0‿1‿0
! gold     ≡ 5‿10‿115‿20‿25‿30
! blessed  ≡ 1‿1‿0‿1‿1‿1
! ((¬sel)/pos) ≡ (¬sel)/pos0               # nothing outside the frozen mask moved
! ((¬sel)/gold) ≡ (¬sel)/gold0
