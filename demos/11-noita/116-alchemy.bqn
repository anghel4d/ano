
i ← ↕12
pool ← ((5⊸≤∧≤⟜7) i) ∧⌜ (3⊸≤∧≤⟜5) i
vein ← (6=i) ∧⌜ (6⊸≤) i
oil ← pool∨vein
water ← ((9⊸≤∧≤⟜10) i) ∧⌜ (3⊸≤∧≤⟜8) i
! 15 = +´⥊oil

disc ← 5 ≥ (×˜i-6) +⌜ ×˜i-6
fire0 ← oil∧disc
! 9 = +´⥊fire0

S ← {(»𝕩)+(«𝕩)+(»˘𝕩)+(«˘𝕩)+(»»˘𝕩)+(»«˘𝕩)+(«»˘𝕩)+(««˘𝕩)}
fire1 ← fire0 ∨ oil ∧ (¬fire0) ∧ 1 ≤ S fire0
! 13 = +´⥊fire1
! (fire1>fire0) ≡ 1¨⌾(⟨5‿3, 6‿3, 7‿3, 6‿9⟩⊸⊑) 12‿12⥊0
! ¬ ∨´ ⥊ fire1∧water
! ∧´ ⥊ fire1≤oil
