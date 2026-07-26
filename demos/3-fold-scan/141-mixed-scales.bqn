# ano:  +/ +\ */ *\ avg/ avg\ max/ min/ over 1, 0.1, 2, 0.2, 3, 0.3
# Semantics: integers and fractions in one column. No magnitude here is extreme, but every prefix rounds, so the sum and the product land one ulp away from their reassociated twins. The mean is a prefix machine whose finish is the state after the last element, not a second reduction over the payload.
# Invariant: the left sum and product differ from the right ones in the last bit; the fold is the last element of its scan for every instance; and the mean's finish equals the last running prefix, which is the law a backend-ordered finish breaks.

mix ← 1‿0.1‿2‿0.2‿3‿0.3

LeftSum  ← {+˜´⌽(0∾𝕩)}
LeftProd ← {×˜´⌽(1∾𝕩)}
Mean     ← {(LeftSum 𝕩)÷≠𝕩}

! 6.6000000000000005 ≡ LeftSum mix
! 6.6 ≡ +´ mix
! (LeftSum mix) ≡ ¯1⊑+` mix

! 0.036000000000000004 ≡ LeftProd mix
! 0.036 ≡ ×´ mix
! (LeftProd mix) ≡ ¯1⊑×` mix

! 1.1 ≡ Mean mix
! (Mean mix) ≡ ¯1⊑(+`mix)÷+`1¨mix

! 3 ≡ ⌈˜´⌽mix
! 0.1 ≡ ⌊˜´⌽mix
