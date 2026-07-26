# ano:  +/ and +\ over big, small, and grain
# Semantics: the same left recurrence at three scales. big is large beside large and cancels to 0 before its trailing 1 arrives; small is small beside small and loses its subnormal head into a normal neighbour; grain lies wholly inside the subnormals, where addition is exact and the two accumulation orders therefore agree.
# Invariant: left and right disagree on big and on small and agree on grain; the fold is the last element of its scan in all three; and nothing here manufactures an infinity — every intermediate stays finite.

big   ← 1e300‿2e300‿¯3e300‿1
small ← 1e¯320‿1e¯300‿¯1e¯300‿0
grain ← 2.5e¯323‿5e¯324‿¯2.5e¯323‿0

LeftSum ← {+˜´⌽(0∾𝕩)}

! 1 ≡ LeftSum big
! 0 ≡ +´ big
! 1e300‿3e300‿0‿1 ≡ +` big
! (LeftSum big) ≡ ¯1⊑+` big

! 0 ≡ LeftSum small
! 1e¯320 ≡ +´ small
! (LeftSum small) ≡ ¯1⊑+` small

# subnormal addition is exact, so grain answers one unit in the last place either way
! 5e¯324 ≡ LeftSum grain
! 5e¯324 ≡ +´ grain
! (LeftSum grain) ≡ ¯1⊑+` grain

! ∧´ (¯∞<big∾small∾grain) ∧ (big∾small∾grain)<∞
