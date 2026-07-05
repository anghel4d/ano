# Sequenced effects (ex13): `;`-batched effects share one statement's barrier.
# ano: ^cursor , Knockback 5 ; Flash :Red ; -Shielded
#      Nord & TwoHanded > 60 , Gold += 1000 ; +Blessed
# Semantics: the selection gathers once; every batched effect reads that pre-state and the merged delta scatters once. Knockback 5 is a registered verb, modeled as +5 along x. The batch must commute: disjoint write columns here.
# Invariant: each batched effect lands on exactly the gathered rows, off-mask cells are bit-identical to pre-state, and applying the batch in reverse order gives the same post-state.

sNone‿sRed ← 0‿1   # Flash symbols: :None :Red

cursor    ← 0‿0‿1‿0‿0‿0    # the alias: a one-hot mask
pos       ← 0‿10‿20‿30‿40‿50
flash     ← sNone¨↕6
shielded  ← 1‿1‿1‿0‿1‿0
nord      ← 1‿1‿0‿1‿1‿0
twoHanded ← 70‿50‿90‿61‿10‿99
gold      ← 5‿10‿15‿20‿25‿30
blessed   ← 0‿1‿0‿0‿0‿0

pos0‿flash0‿shielded0‿gold0‿blessed0 ← pos‿flash‿shielded‿gold‿blessed

# statement 1: one gather, three effects, one scatter
sel ← cursor
pos +↩ 5×sel
flash ↩ (sRed¨)⌾(sel⊸/) flash
shielded ∧↩ ¬sel

! pos      ≡ 0‿10‿25‿30‿40‿50
! flash    ≡ sNone‿sNone‿sRed‿sNone‿sNone‿sNone
! shielded ≡ 1‿1‿0‿0‿1‿0

# reverse order over the same pre-state: the batch commutes
shieldedR ← shielded0 ∧ ¬sel
flashR    ← (sRed¨)⌾(sel⊸/) flash0
posR      ← pos0 + 5×sel
! pos‿flash‿shielded ≡ posR‿flashR‿shieldedR

# statement 2: Nord & TwoHanded > 60 , Gold += 1000 ; +Blessed
sel ↩ nord ∧ twoHanded>60
gold +↩ 1000×sel
blessed ∨↩ sel

! sel     ≡ 1‿0‿0‿1‿0‿0
! gold    ≡ 1005‿10‿15‿1020‿25‿30
! blessed ≡ 1‿1‿0‿1‿0‿0

# off-mask cells untouched
! ((¬sel)/gold)    ≡ (¬sel)/gold0
! ((¬sel)/blessed) ≡ (¬sel)/blessed0
