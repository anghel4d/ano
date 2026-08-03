# ano: avg\ and avg/ over weight 400 500 600 900, and the empty scope under the mean and the count.
# Semantics: the mean machine's state is (sum,count); each prefix emits sum÷count and the finish is the state after the last element — the same left order the scan holds, so avg/ is the last prefix of avg\.
# Invariant: the prefixes are integral by fixture design; the empty mean has no result row; the empty count is 0.

w ← 400‿500‿600‿900
LeftSum ← {+˜´⌽(0∾𝕩)}
Mean    ← {(LeftSum 𝕩)÷≠𝕩}
prefixes ← (+`w)÷+`1¨w
! 400‿450‿500‿600 ≡ prefixes
! 600 ≡ Mean w
! (Mean w) ≡ ¯1⊑prefixes                   # the finish is the last prefix
! 2400 ≡ LeftSum w
! 4 ≡ ≠w
! ⟨⟩ ≡ (+`⟨⟩)÷+`1¨⟨⟩                        # empty scan is empty
! 0 ≡ +´⟨⟩                                  # count declares 0
