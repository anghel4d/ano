# The modifier chain (w3): each modifier binds the next projectile in draw order; the join is a reverse min-scan.
# Ano: Spell , Bal = 1 + (+\ (DrawAdd - 1) @ Spell)
#      Spell , MinBal = 0 - (max\ (0 - Bal) @ Spell)
#      Spell @ deck & (Slot == 0 | prev.MinBal > 0) , +Member
#      Spell & Proj & Member & Slot == min/ Slot @ (Spell & Proj & Member) , Damage += 10 * (#/ (Spell & Mod & Member))
# Semantics: "next projectile at or after slot i" is min{slot_p : p projectile, slot_p >= i} — a reverse running-min over the slot line with non-projectiles at +inf, one scan, no loop; multicast markers are passed straight through, which is Noita's rule (the modifier recurses into the group and lands on its first bolt). When every modifier precedes every projectile the join collapses to "the block's first projectile takes the whole stack" — the twin's scoped-fold spelling; the interleaved wand pins where that shortcut dies, and the -b/-c twins run the general join on it — as an order-derived srel and as the value-level rel.
# Invariant: [DP, DP, Triple, SB, SB, SB] is one block and both DP land on the first bolt (3 to 23); the interleaved one-block wand [DP, Triple, SB, DP, SB, SB] binds DP0 to the bolt at slot 2 and DP3 to slot 4 — the general join and the shortcut disagree there, and the general join is the evaluator.

Draw ← {1 + +` 𝕩-1}
Member ← {((↕≠𝕩)=0) ∨ 0 < » ⌊` 𝕩}
NextP ← {⌽ ⌊` ⌽ (↕≠𝕩) + 99 × ¬𝕩}          # per slot: the next projectile's slot, 99 = none
Bind ← {t ← 𝕨/NextP 𝕩 ⋄ {+´t=𝕩}¨ ↕≠𝕩}     # 𝕨 mod mask, 𝕩 proj mask: modifiers riding each slot

# ── the demo wand: DP, DP, Triple, SB, SB, SB — one block, mods all up front
proj ← 0‿0‿0‿1‿1‿1 ⋄ mod ← 1‿1‿0‿0‿0‿0
bal ← Draw 1‿1‿3‿0‿0‿0
! 1‿1‿3‿2‿1‿0 ≡ bal
! 1‿1‿1‿1‿1‿1 ≡ Member bal                # the Triple's +3 keeps the whole wand in one block
! 3‿3 ≡ mod / NextP proj                  # both modifiers bind the multicast's first bolt
dmg ← 0‿0‿0‿3‿3‿3
general ← dmg + 10 × mod Bind proj
! 0‿0‿0‿23‿3‿3 ≡ general
first ← ⌊´ proj / ↕6                      # the twin's shortcut: the block's first projectile
! general ≡ dmg + (10 × +´mod) × (↕6) = first   # legal here: every mod precedes every bolt

# ── the interleaved wand: DP, Triple, SB, DP, SB, SB — still one block, the shortcut dies
proj2 ← 0‿0‿1‿0‿1‿1 ⋄ mod2 ← 1‿0‿0‿1‿0‿0
bal2 ← Draw 1‿3‿0‿1‿0‿0
! 1‿1‿1‿1‿1‿1 ≡ Member bal2
! 2‿4 ≡ mod2 / NextP proj2                # DP0 rides the Triple's bolt, DP3 the one after
dmg2 ← 0‿0‿3‿0‿3‿3
general2 ← dmg2 + 10 × mod2 Bind proj2
! 0‿0‿13‿0‿13‿3 ≡ general2
shortcut2 ← dmg2 + (10 × +´mod2) × (↕6) = ⌊´ proj2 / ↕6
! general2 ≢ shortcut2                    # the correlated join is the evaluator, pinned
