# 073-conjugation - A local tie-free conjugation theorem
# ano: Unit , Rank = rank(Gold)          -- the rank scatter is the conjugated form of order-work
# Semantics: on this tie-free column, grade out, act in value order, and inverse-grade back yields a permutation-equivariant column map for the tested relabellings. Sorting alone drops the lineage needed to return.
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
