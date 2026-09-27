# ano: Unit , Rank = rank(Gold)
# invariant: on gold 5 5 3, ordinal rank is 1 2 0 (tie split by position) and dense rank is 1 1 0 (tie shared); permuting the tied pair leaves the column bitwise identical, yet ordinal rank fails w(c∘σ) ≡ w(c)∘σ while dense rank satisfies it.

# world: 3 units, all selected
gold ← 5‿5‿3
rank ← 3⥊¯1

rank ↩ ⍋⍋gold
! 1‿2‿0 ≡ rank            # stable: unit 0 outranks unit 1 by position, not value

# the same write-back under dense value-only rank
rank ↩ (∧⍷gold)⊐gold
! 1‿1‿0 ≡ rank            # tied units share a rank; ranks are a function of the value

p ← 1‿0‿2
! (p⊏gold) ≡ gold                              # the permuted column is bitwise the same
! ¬ (⍋⍋p⊏gold) ≡ p⊏⍋⍋gold                      # ordinal: same column in, ranks follow position, not the name
! ((∧⍷p⊏gold)⊐p⊏gold) ≡ p⊏(∧⍷gold)⊐gold        # dense: commutes with the relabeling
