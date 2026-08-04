# ano: count and sum over Merchant & !Whiterun beside Merchant & !^Whiterun
# Semantics: the overlay changes the selection, not the reducer. Sum has numeric identity 0; maximum has no identity, so the empty maximum has no result row.

merchant ← 1‿1‿0‿1‿0‿1‿0‿0
bare     ← 1‿0‿1‿0‿0‿0‿1‿0
live     ← 0‿0‿0‿1‿0‿1‿0‿0
gold     ← 100‿50‿20‿5‿0‿25‿7‿3
none     ← 8⥊0

bareSel ← merchant ∧ ¬bare
liveSel ← merchant ∧ ¬live
LeftSum ← {+˜´⌽(0∾𝕩)}
NoMaximum ← {0=≠𝕩 ? ⟨⟩ ; ⟨⌈˜´⌽𝕩⟩}

! 3 ≡ +´bareSel
! 2 ≡ +´liveSel
! 80 ≡ LeftSum bareSel/gold
! 150 ≡ LeftSum liveSel/gold
! 0 ≡ LeftSum none/gold
! 0 = ≠ NoMaximum none/gold

# Deleting the live entry exposes the unchanged bare mask again.
Resolve ← {𝕨 ? 𝕩 ; bare}
! live ≡ 1 Resolve live
! bare ≡ 0 Resolve live
! bare ≡ 1‿0‿1‿0‿0‿0‿1‿0
