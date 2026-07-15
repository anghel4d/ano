# 072-reverse - Reverse conjugates translation; maximal lattice equivariance is not a legality criterion
# ano: ⌽ g    -- a derived order operation on a declared cyclic habitat
# Semantics: on a length-8 cyclic habitat, reverse is a lawful derived order operation but is not translation-equivariant; it normalizes the translation group rather than centralizing it.
# Invariant: rev(shift_t v) ≡ shift_{-t}(rev v) for every t, while ordinary commutation fails at t=1. This is the executable counterexample to the retired criterion.

v ← 3‿1‿4‿1‿5‿9‿2‿6            # length-8 cyclic lattice

# conjugation law: reverse carries every translation to its inverse
! ∧´ {(⌽𝕩⌽v) ≡ (-𝕩)⌽⌽v}¨ ↕8

# non-commutation: at t=1 the naive equivariance rev∘shift ≡ shift∘rev is false
! ¬ (⌽1⌽v) ≡ 1⌽⌽v              # 3‿6‿2‿9‿5‿1‿4‿1 vs 2‿9‿5‿1‿4‿1‿3‿6
