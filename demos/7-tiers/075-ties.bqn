# 075-ties - Tie policy: stable ordinal rank observes order; dense rank is value-only
# ano: Unit , Rank = rank(Gold)
# Semantics: stable ordinal rank observes the declared input order, while dense rank depends only on value. Equivariance is tested here as a local property, not imposed as a universal entity law.
# Invariant: on tied data ⍋⍋ (stable ordinal rank) violates the law at σ = swap(0,1), and dense value-only rank (∧⍷c)⊐c satisfies it at every σ ∈ Sym(3).

c ← 5‿5‿3
s ← 1‿0‿2                      # σ = swap(0,1): permutes the two tied 5s
Ord ← ⍋∘⍋                      # stable ordinal rank
Den ← {(∧⍷𝕩)⊐𝕩}                # dense value-only rank

# hand-computed anchors
! 1‿2‿0 ≡ Ord c
! 1‿1‿0 ≡ Den c

# stable rank: relabeling the tied pair changes which slot gets which ordinal — the law fails
! ¬ (Ord s⊏c) ≡ s⊏Ord c        # 1‿2‿0 vs 2‿1‿0

# dense rank: rank depends on the value alone, so it commutes with every relabeling
perms ← ⟨0‿1‿2, 0‿2‿1, 1‿0‿2, 1‿2‿0, 2‿0‿1, 2‿1‿0⟩
! ∧´ {(Den 𝕩⊏c) ≡ 𝕩⊏Den c}¨ perms
