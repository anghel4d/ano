# Ano: SparkBolt , Damage += 10                 -- Damage Plus
#      SparkBolt , Damage += 10 ; Damage += 10  -- two Damage Plus, ONE barrier: the additive merge
# Semantics: each statement is a barrier, so a later card reads what an earlier card committed — the wand's slot order is the program. Within one barrier batched effects observe the pre-state and must commute under a registered merge law (§10): two additive increments merge to +20; += beside *= admits no merge law, so those cards can never share a barrier and their order is semantics.

sb ← 0‿1‿1‿1‿0 ⋄ d0 ← 0‿3‿3‿3‿8 ⋄ s0 ← 0‿6‿6‿6‿2
DP  ← {𝕩 + 10×sb}      # masked add
HSd ← {𝕩 × 1+sb}       # masked double: ×(1+mask), spec §7
HSs ← {𝕩 - 2×sb}

! 0‿26‿26‿26‿8 ≡ HSd DP d0           # wand order A: Heavy Shot doubles what Damage Plus committed
! 0‿16‿16‿16‿8 ≡ DP HSd d0           # wand order B: the same cards, the other program
! (HSd DP d0) ≢ DP HSd d0            # no merge law joins += and *=: the barrier forces an order
! 0‿4‿4‿4‿2 ≡ HSs s0

# two Damage Plus in one barrier: both deltas against pre-state, additive increments commute
merged ← d0 + (10×sb) + 10×sb
! 0‿23‿23‿23‿8 ≡ merged
lastWins ← d0 + 10×sb                # commit only the second delta: a commuting card silently lost
! 0‿13‿13‿13‿8 ≡ lastWins
! merged ≢ lastWins
