# The Greek letters (w4): a reflective spell is a query over the wand's own table.
# Ano: Spell |> order by Slot |> take 1 , spawn Copy          -- Alpha: the wand's first spell
#      Spell |> order by Slot desc |> take 1 , spawn Copy     -- Gamma: the last
#      Spell , spawn Copy                                     -- Omega: every spell
#      Spell & Mod , spawn Copy                               -- Mu: every modifier
# Semantics: Noita's Greek letters make the interpreter reify its own spell list — reflection bolted onto code. Here the wand was a table before any spell asked: Alpha is a grade-and-take (§15), Omega a one-per-row replicate (§17), Mu a mask, and parent records which slot each letter copied. No quotation, no staging — copies are rows, not code; the staging open question stays dormant until a spell must mint a SPELL that itself computes (ano-limitations.md).
# Invariant: over wand [DP, SB, Bomb, Triple] the four letters mint 1+1+4+1 rows with keys continuing 5..11 and parents ⟨1⟩, ⟨4⟩, ⟨1,2,3,4⟩, ⟨1⟩ — Alpha found the modifier at slot 0, Gamma the multicast at the end, Mu re-found Alpha's pick.

slot ← ↕4 ⋄ mod ← 1‿0‿0‿0
! 0 = ⊑ ⍋ slot                            # Alpha's pick: the first slot
! 3 = ⊑ ⍒ slot                            # Gamma's: the last
! ⟨0⟩ ≡ / mod                             # Mu's: the modifiers

# parent pins the reflection, in world rows (spells sit at 1+slot)
! 1‿4‿1‿2‿3‿4‿1 ≡ (1+⊑⍋slot)‿(1+⊑⍒slot) ∾ (1+↕4) ∾ 1+/mod

# keys mint statement by statement, 1 + 1 + 4 + 1 on top of rows 0..4
Mint ← {𝕩∾(1+⌈´𝕩)+↕𝕨}
! (↕12) ≡ 1 Mint 4 Mint 1 Mint 1 Mint ↕5
