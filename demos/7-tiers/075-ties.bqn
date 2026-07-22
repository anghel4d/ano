
c ← 5‿5‿3
s ← 1‿0‿2
Ord ← ⍋∘⍋
Den ← {(∧⍷𝕩)⊐𝕩}

! 1‿2‿0 ≡ Ord c
! 1‿1‿0 ≡ Den c

! ¬ (Ord s⊏c) ≡ s⊏Ord c

perms ← ⟨0‿1‿2, 0‿2‿1, 1‿0‿2, 1‿2‿0, 2‿0‿1, 2‿1‿0⟩
! ∧´ {(Den 𝕩⊏c) ≡ 𝕩⊏Den c}¨ perms
