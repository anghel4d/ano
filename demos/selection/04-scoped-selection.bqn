# Scoped selection (ex4).
# ano: Merchant @ Whiterun , Gold += 5000
# Semantics: @ is the locative scope — evaluate the mask within the region, an AND with the region's membership mask.
# Invariant: only merchants inside Whiterun gain gold; merchants elsewhere and non-merchants inside are untouched.

symWhiterun‿symRiften ← ↕2          # region enum codes standing in for `Whiterun `Riften

merchant ← 1‿1‿0‿1‿0‿1
region   ← symWhiterun‿symRiften‿symWhiterun‿symWhiterun‿symRiften‿symRiften
whiterun ← region = symWhiterun     # the scope resolves to a membership mask
gold     ← 100‿100‿100‿100‿100‿100

gold +↩ 5000 × merchant ∧ whiterun

! 5100‿100‿100‿5100‿100‿100 ≡ gold
