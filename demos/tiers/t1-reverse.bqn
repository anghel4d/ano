# t1-reverse - Tier 1 reverse conjugates translation, it does not commute with it
# ano: ⌽ g    -- the spec's Tier-1 line: reverse/reshape free, order is data not identity
# Semantics: on a length-8 cyclic lattice with translations shift_t = t⌽, reverse is lawful Tier-1 order-work but it is not G₀-equivariant: it normalizes the translation group rather than centralizing it, rev∘shift_t = shift_{-t}∘rev.
# Invariant: rev(shift_t v) ≡ shift_{-t}(rev v) for every t (assert), and rev(shift_t v) ≢ shift_t(rev v) at t=1 (assert), refuting the old criterion that Tier-1 legality is G₀-equivariance.

v ← 3‿1‿4‿1‿5‿9‿2‿6            # length-8 cyclic lattice

# conjugation law: reverse carries every translation to its inverse
! ∧´ {(⌽𝕩⌽v) ≡ (-𝕩)⌽⌽v}¨ ↕8

# non-commutation: at t=1 the naive equivariance rev∘shift ≡ shift∘rev is false
! ¬ (⌽1⌽v) ≡ 1⌽⌽v              # 3‿6‿2‿9‿5‿1‿4‿1 vs 2‿9‿5‿1‿4‿1‿3‿6
