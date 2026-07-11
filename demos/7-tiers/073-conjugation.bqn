# 073-conjugation - Tier 2 conjugation: grade out, do free order-work, index back aligned
# ano: Unit , Rank = rank(Gold)          -- the rank scatter is the conjugated form of order-work
# Semantics: on tie-free c the conjugate h = σ_c⁻¹ ∘ f ∘ σ_c (grade out, apply free Tier-1 f in value order, inverse-grade back) is a lawful Tier-2 write-back: h(c∘τ) ≡ h(c)∘τ for every relabeling τ; sorting alone (c∘σ_c) is not, it drops the inverse leg.
# Invariant: h with f = +` and f = ⌽ is Sym(I)-equivariant across several τ, the grade identity σ_{c∘τ} = τ⁻¹∘σ_c holds, and the hand-computed running-total column anchors h.

c ← 40‿10‿30‿20‿50             # tie-free gold
_h ← {s←⍋𝕩 ⋄ (⍋s)⊏𝔽s⊏𝕩}        # h = grade-inverse ∘ f ∘ grade
Hscan ← +`_h
Hrev  ← ⌽_h

# hand-computed anchor: each entity's running total in value order, written back to its own slot
! 100‿10‿60‿30‿150 ≡ Hscan c

taus ← ⟨4‿3‿2‿1‿0, 1‿2‿3‿4‿0, 2‿0‿1‿4‿3, 3‿0‿4‿1‿2, 0‿1‿2‿3‿4⟩

# Sym(I)-equivariance of the conjugate: h(c∘τ) ≡ h(c)∘τ
! ∧´ {(Hscan 𝕩⊏c) ≡ 𝕩⊏Hscan c}¨ taus
! ∧´ {(Hrev 𝕩⊏c) ≡ 𝕩⊏Hrev c}¨ taus

# the grade identity behind it: σ_{c∘τ} = τ⁻¹ ∘ σ_c  (as a gather, (⍋c)⊏⍋τ)
! ∧´ {(⍋𝕩⊏c) ≡ (⍋c)⊏⍋𝕩}¨ taus

# sorting alone is not a lawful write-back: sort(c∘τ) is sort(c), never sort(c)∘τ for a moving τ
tau ← 4‿3‿2‿1‿0
! ¬ (∧tau⊏c) ≡ tau⊏∧c
