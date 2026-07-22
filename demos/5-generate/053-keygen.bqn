
keys ← 3‿7‿9‿12‿20

masks ← ⥊ ↕2‿2‿2‿2‿2
! 32 = ≠masks

Chk ← {f ← 𝕩/keys ⋄ (∧´f∊keys) ∧ f ≡ (keys∊f)/keys}
! ∧´ Chk¨ masks

! ∧´ {(≠𝕩/keys) ≤ ≠keys}¨ masks

! (1‿0‿1/3‿7‿9) ≡ (1‿1‿0/3‿9‿7)

count ← 2‿0‿3‿1‿0
new ← (1+⌈´keys) + ↕+´count
! 6 = ≠new
! ∧´ ¬ new ∊ keys

! ∧´ {¬∨´ (𝕩/keys) ∊ new}¨ masks

! (⥊↕3‿4) ≡ {(⌊𝕩÷4)⋈4|𝕩}¨ ↕12
