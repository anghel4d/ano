# The assembled wand (n6): [Twin Spell, Damage Plus, Trigger, Firebolt], fired at an enemy standing over an oil vein.
# Ano: Wand , spawn SparkBolt * 2 at pos + (12, index * 4 - 2) ; Mana -= 25
#      SparkBolt , Damage += 10 ; +Trigger
#      SparkBolt , pos = pos + (24, 0)
#      [ b & e , +Hit | b <- SparkBolt, e <- Enemy, dist(b, e) < 8 ]
#      Enemy & Hit , Health -= +/ Damage @ SparkBolt
#      def payload = SparkBolt & Trigger & Hit => spawn Firebolt at pos ; ~
#      Oil & (x - 6) * (x - 6) + (y - 6) * (y - 6) <= 5 , +Fire
#      Oil & !Fire & #/ (moore' & Fire) >= 1 , +Fire
# Semantics: the n1-n5 ladder as one program — cast, cards riding the fresh volley in one barrier, one frame of flight, the collision join, the damage spend as a scoped fold (the volley's stacked damage, one scalar), the trigger rule minting a payload at each impact, then the material chunk takes over. The chunk names its own coordinates: a lattice predicate cannot yet read an entity column, so the detonation centers at chunk (6,6) by construction — the frame join, surfaced in the README.
# Invariant: both bolts hit; the volley spends 26 = 2 x (3+10) and the enemy keeps 4; both payloads hold their own impact pos and dangling parents; the performed spread ring equals n5's installed tick cell for cell.

# ── the cast: two copies, index per copy, cards in the next two barriers
boltPos ← ⟨5‿5+12‿¯2, 5‿5+12‿2⟩
! ⟨17‿3, 17‿7⟩ ≡ boltPos
dmg ← 3+10 ⋄ mana ← 100-25
! 75 = mana

# ── flight, then the collision join against the enemy at 40‿5
flight ← (24‿0⊸+)¨ boltPos
D ← {√+´×˜𝕨-𝕩}
! ∧´ 8 > (40‿5⊸D)¨ flight              # both bolts of the twin cast land the hit

# ── the spend: a scoped fold over the flight group, one scalar into the enemy
! 4 = 30 - +´ dmg‿dmg

# ── the trigger: both carriers consumed, both payloads minted at their own impact
! 5‿6 ≡ (1+⌈´↕5)+↕2                    # live keys 0..4 at the gather, payload keys mint 5 6
! ⟨41‿3, 41‿7⟩ ≡ flight                 # each payload holds its carrier's pos
! 3‿4 ≡ 3+↕2                           # parents: the carriers' gather-time rows, dangling after the barrier

# ── the chunk: detonation and one performed ring — n5's installed tick, performed here
i ← ↕12
oil ← (((5⊸≤∧≤⟜7) i) ∧⌜ (3⊸≤∧≤⟜5) i) ∨ (6=i) ∧⌜ (6⊸≤) i
water ← ((9⊸≤∧≤⟜10) i) ∧⌜ (3⊸≤∧≤⟜8) i
disc ← 5 ≥ (×˜i-6) +⌜ ×˜i-6
S ← {(»𝕩)+(«𝕩)+(»˘𝕩)+(«˘𝕩)+(»»˘𝕩)+(»«˘𝕩)+(«»˘𝕩)+(««˘𝕩)}
fire0 ← oil∧disc
fire1 ← fire0 ∨ oil ∧ (¬fire0) ∧ 1 ≤ S fire0
! 13 = +´⥊fire1
! ¬ ∨´ ⥊ fire1∧water
! ∧´ ⥊ fire1≤oil
