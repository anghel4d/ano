# ano: 両手が六十を超える北に、金を千与える。
# ano: Nord & TwoHanded > 60 , Gold += 1000
# Semantics: attributive position is restriction — the clause 両手が六十を超える filters 北 by juxtaposition, no relativizer — and the mask is the same whichever side of & the clause sits on.
# Invariant: clause∧noun equals noun∧clause, and Gold's post-state matches the hand-computed column.

nord      ← 1‿0‿1‿1‿0‿1
twoHanded ← 61‿75‿30‿90‿55‿60
gold      ← 5‿5‿5‿5‿5‿5
clause ← twoHanded>60
"ex43: juxtaposition commutes"!((clause∧nord)≡nord∧clause)
gold +↩ 1000×nord∧clause
"ex43: post-state Gold"!(1005‿5‿5‿1005‿5‿5≡gold)
