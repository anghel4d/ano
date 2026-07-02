# Value effects (ex10): aligned masked arithmetic over dense columns.
# ano: Nord , Gold += 1000
#      Bandit , Health -= 25
#      Merchant , Gold *= 2
#      Wounded , Speed /= 2
# Semantics: each statement is one gather-effect-scatter; the effect is a masked array operation, the arithmetic identity holding off-mask (0 for +-, factor 1 for */).
# Invariant: on-mask cells take the operation, off-mask cells are bit-identical to pre-state.

nord     ← 1‿1‿0‿0‿1‿0‿0‿1
bandit   ← 0‿0‿1‿1‿0‿0‿1‿0
merchant ← 0‿1‿0‿0‿0‿1‿0‿0
wounded  ← 1‿0‿0‿1‿0‿0‿0‿0
gold     ← 100‿200‿300‿400‿500‿600‿700‿800
health   ← 50‿60‿70‿80‿90‿100‿40‿30
speed    ← 10‿8‿6‿4‿12‿14‿2‿16

gold0‿health0‿speed0 ← gold‿health‿speed   # pre-state copies

gold   +↩ 1000×nord
health -↩ 25×bandit
gold   ×↩ 1+merchant
speed  ÷↩ 1+wounded

! gold   ≡ 1100‿2400‿300‿400‿1500‿1200‿700‿1800
! health ≡ 50‿60‿45‿55‿90‿100‿15‿30
! speed  ≡ 5‿8‿6‿2‿12‿14‿2‿16

# off-mask cells untouched
! ((¬nord∨merchant)/gold)   ≡ (¬nord∨merchant)/gold0
! ((¬bandit)/health)        ≡ (¬bandit)/health0
! ((¬wounded)/speed)        ≡ (¬wounded)/speed0
