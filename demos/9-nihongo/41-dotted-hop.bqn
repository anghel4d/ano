# Ex41 — Japanese dotted hop: の is the relationship gather.
# ano: 北 と 師匠 の 両手 八十 より 、 金 に 千 たす
# ano: Nord & mentor.TwoHanded > 80 , Gold += 1000
# Semantics: mentor.TwoHanded is one indexed read — gather each entity's mentor ID, index TwoHanded at it — and the mask keeps Nords whose mentor's skill exceeds 80.
# Invariant: Gold rises by 1000 exactly on Nord rows whose gathered mentor TwoHanded is over 80.

mentor    ← 2‿3‿3‿0‿5‿1        # all links valid; the dangling case is ex44
twoHanded ← 70‿85‿90‿81‿60‿40
nord      ← 1‿1‿1‿0‿1‿0
gold      ← 0‿10‿20‿30‿40‿50
gold +↩ 1000×nord∧80<mentor⊏twoHanded
"ex41: post-state Gold"!(1000‿1010‿1020‿30‿40‿50≡gold)
