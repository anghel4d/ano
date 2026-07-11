# Ex42 — Japanese scoped selection: で is the locative scope.
# ano: 商人 ホワイトラン で 、 金 に 五千 たす
# ano: Merchant @ Whiterun , Gold += 5000
# Semantics: @ evaluates the selection within a scope — the Whiterun region mask, which per the decision record also fixes the coordinate frame — and the effect adds 5000 Gold under Merchant∧Whiterun.
# Invariant: Gold rises by 5000 exactly on merchant rows inside the Whiterun region.

region   ← 1‿1‿0‿1‿2‿1        # region codes; 1 = Whiterun
whiterun ← region=1
merchant ← 1‿0‿1‿1‿0‿1
gold     ← 10‿20‿30‿40‿50‿60
gold +↩ 5000×merchant∧whiterun
"ex42: post-state Gold"!(5010‿20‿30‿5040‿50‿5060≡gold)
