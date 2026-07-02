# Ex47 — Counters and becoming: a frame-typed numeral in the predicate, a masked doubling in the effect.
# ano: 蔵で三ヶ月より熟成したチーズは、値が倍になる。
# ano: Cheese @ cellar & Aged > 3mo , Price *= 2
# Semantics: 三ヶ月 is the counter-typed literal 3mo — the ヶ月 counter is the unit, checked at the で/@ boundary — precedence parses the line as (Cheese @ cellar) & (Aged > 3mo), and the doubling is the masked-factor idiom Price×1+mask; the Japanese casts the effect in the なる voice, which under the naru ruling would install a standing rule via =>, while the modeled ASCII line keeps the performed comma.
# Invariant: Price doubles exactly on cheese rows in the cellar aged over three months.

mo     ← 1                     # month unit; the counter check is unit consistency
cheese ← 1‿1‿1‿0‿1‿1
cellar ← 1‿1‿0‿1‿1‿1
aged   ← 4‿2‿6‿5‿3‿7           # months
price  ← 10‿20‿30‿40‿50‿60
price ×↩ 1+cheese∧cellar∧aged>3×mo
"ex47: post-state Price"!(20‿20‿30‿40‿50‿120≡price)
