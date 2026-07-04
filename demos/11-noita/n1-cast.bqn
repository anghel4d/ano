# The cast (n1): a projectile spell is an append; multicast is barrier batching; formation is the replicate's per-copy index.
# Ano: Wand , spawn SparkBolt at pos + (12, 0) ; Mana -= 5
#      Wand , spawn SparkBolt at pos + (12, 0) ; spawn Bomb at pos + (4, 0) ; Mana -= 30   -- Double Spell
#      Wand , spawn SparkBolt * 3 at pos + polar(12, (index - 1) * 90) ; Mana -= 15        -- Formation - Trifurcated
# Semantics: spawn appends rows — keys mint from 1+max, parent records the caster, each copy reads its source's pos and its own index (§17); the mana drain shares the cast's barrier. Double Spell is two prototypes in ONE barrier: row groups append in effect order, keys mint once across the batch — legal exactly because neither spawn reads what the other writes, the §10 commutation condition, witnessed here on the key mint.
# Invariant: minting two keys in one barrier equals minting one then one; the trifurcated fan is polar off the caster with index bound per copy; the wand's mana ledger closes at 50.

Polar ← {𝕨×(•math.Cos 𝕩×π÷180)⋈•math.Sin 𝕩×π÷180}

# key minting: append 𝕨 fresh keys to the ledger 𝕩
Mint ← {𝕩∾(1+⌈´𝕩)+↕𝕨}
! (2 Mint ⟨0⟩) ≡ 1 Mint 1 Mint ⟨0⟩            # Double Spell: one batch of two ≡ two barriers of one — why the batch commits deterministically
! (↕7) ≡ 3 Mint 1 Mint 1 Mint 1 Mint ⟨0⟩      # the file's four casts mint keys 1..6

# the trifurcated fan: index per copy, polar off the caster's own pos
fan ← {5‿5 + 12 Polar 𝕩}¨ 90 × ¯1‿0‿1
posF ← ⟨5‿5, 17‿5, 17‿5, 9‿5⟩ ∾ fan
! ∧´ 1e¯9 ≥ | ∾ posF - ⟨5‿5, 17‿5, 17‿5, 9‿5, 5‿¯7, 17‿5, 5‿17⟩

# the wand pays for every cast in the cast's own barrier
! 50 = 100 - +´ 5‿15‿15‿15
