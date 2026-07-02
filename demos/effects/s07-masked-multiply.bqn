# Masked multiply, corrected (spec §7): doubling merchants' gold.
# ano: Merchant , Gold *= 2
# Semantics: the multiplicative identity off-mask is 1, so the factor column is 1+merchant (general factor k: 1+(k-1)×mask); 2×merchant is 0 off-mask and wipes every non-merchant.
# Invariant: the corrected form doubles on-mask and preserves off-mask exactly; the wrong form differs and zeroes all off-mask gold.

merchant ← 0‿1‿0‿0‿1‿0
gold     ← 100‿200‿300‿400‿500‿600

right ← gold × 1+merchant   # correct: gold ×↩ 1+merchant
wrong ← gold × 2×merchant   # spec §7's old line: zeroes non-merchants

! right ≡ 100‿400‿300‿400‿1000‿600
! wrong ≡ 0‿400‿0‿0‿1000‿0
! ¬right ≡ wrong

! ((¬merchant)/right) ≡ (¬merchant)/gold   # off-mask preserved
! 0 ≡ +´ (¬merchant)/wrong                 # wrong form wipes off-mask

# general factor k, here k=3
! (gold × 1+2×merchant) ≡ 100‿600‿300‿400‿1500‿600
