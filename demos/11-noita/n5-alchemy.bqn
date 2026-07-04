# Alchemy (n5): the material world is a lattice; the alchemy table is standing rules.
# Ano: Oil & (x - 6) * (x - 6) + (y - 6) * (y - 6) <= 5 , +Fire
#      def spread = Oil & !Fire & #/ (moore' & Fire) >= 1 => +Fire
# Semantics: the Firebolt payload detonates at chunk (6,6), staged from n4 the way split twins pin program points. Ignition is a coordinate-disc predicate meeting the oil mask (§20 — every regular figure is a predicate on the coordinate columns); spread is the 10-conways stencil under a fuel guard — fire is Life with birth-on-any-neighbor restricted to the oil mask. Water never enters a mask, so it cannot burn; oil persists under its fire — consumption is a second rule (Fire & Oil => -Oil) and the pair waits on the shared rule barrier exactly as c3's bloom/wither does.
# Invariant: the blast ignites exactly the 9 oil cells inside the disc; one tick licks exactly one ring further (4 cells, three at the pool's west face and one down the vein) and no more; fire never leaves the fuel and never touches the pond.

i ← ↕12
pool ← ((5⊸≤∧≤⟜7) i) ∧⌜ (3⊸≤∧≤⟜5) i          # 3x3 oil pool, x 5-7, y 3-5
vein ← (6=i) ∧⌜ (6⊸≤) i                       # the vein east along x=6
oil ← pool∨vein
water ← ((9⊸≤∧≤⟜10) i) ∧⌜ (3⊸≤∧≤⟜8) i        # the pond, x 9-10, y 3-8
! 15 = +´⥊oil

# detonation: the blast disc is an outer product on the coordinate columns
disc ← 5 ≥ (×˜i-6) +⌜ ×˜i-6
fire0 ← oil∧disc
! 9 = +´⥊fire0

# one tick of the spread rule: pre-state gather, one ring, fuel-guarded
S ← {(»𝕩)+(«𝕩)+(»˘𝕩)+(«˘𝕩)+(»»˘𝕩)+(»«˘𝕩)+(«»˘𝕩)+(««˘𝕩)}   # Moore counts, dead border
fire1 ← fire0 ∨ oil ∧ (¬fire0) ∧ 1 ≤ S fire0
! 13 = +´⥊fire1
! (fire1>fire0) ≡ 1¨⌾(⟨5‿3, 6‿3, 7‿3, 6‿9⟩⊸⊑) 12‿12⥊0    # exactly the ring: the pool's west face, one cell down the vein
! ¬ ∨´ ⥊ fire1∧water                                       # the pond never burns
! ∧´ ⥊ fire1≤oil                                           # fire stays on the fuel
