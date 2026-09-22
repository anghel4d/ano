
boltPos ← ⟨5‿5+12‿¯2, 5‿5+12‿2⟩
! ⟨17‿3, 17‿7⟩ ≡ boltPos
dmg ← 3+10 ⋄ mana ← 100-25
! 75 = mana

flight ← (24‿0⊸+)¨ boltPos
D ← {√+´×˜𝕨-𝕩}
! ∧´ 8 > (40‿5⊸D)¨ flight

! 4 = 30 - +´ dmg‿dmg

! 5‿6 ≡ (1+⌈´↕5)+↕2
! ⟨41‿3, 41‿7⟩ ≡ flight
! 3‿4 ≡ 3+↕2

i ← ↕12
oil ← (((5⊸≤∧≤⟜7) i) ∧⌜ (3⊸≤∧≤⟜5) i) ∨ (6=i) ∧⌜ (6⊸≤) i
water ← ((9⊸≤∧≤⟜10) i) ∧⌜ (3⊸≤∧≤⟜8) i
disc ← 5 ≥ (×˜i-6) +⌜ ×˜i-6
S ← {(»𝕩)+(«𝕩)+(»˘𝕩)+(«˘𝕩)+(»»˘𝕩)+(»«˘𝕩)+(«»˘𝕩)+(««˘𝕩)}
fire0 ← oil∧disc
fire1 ← fire0 ∨ oil ∧ (¬fire0) ∧ 1 ≤ S fire0
! 13 = +´⥊fire1
! ¬ ∨´ ⥊ fire1∧water
! ∧´ ⥊ fire1≤oil
