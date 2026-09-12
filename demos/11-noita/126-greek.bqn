# Ano: Spell |> order by Slot |> take 1 , spawn Copy          -- Alpha: the wand's first spell
#      Spell |> order by Slot desc |> take 1 , spawn Copy     -- Gamma: the last
#      Spell , spawn Copy                                     -- Omega: every spell
#      Spell & Mod , spawn Copy                               -- Mu: every modifier
# Invariant: over wand [DP, SB, Bomb, Triple] the four letters mint 1+1+4+1 rows with keys continuing 5..11 and parents ⟨1⟩, ⟨4⟩, ⟨1,2,3,4⟩, ⟨1⟩ — Alpha found the modifier at slot 0, Gamma the multicast at the end, Mu re-found Alpha's pick.

slot ← ↕4 ⋄ mod ← 1‿0‿0‿0
! 0 = ⊑ ⍋ slot                            # Alpha's pick: the first slot
! 3 = ⊑ ⍒ slot                            # Gamma's: the last
! ⟨0⟩ ≡ / mod                             # Mu's: the modifiers

! 1‿4‿1‿2‿3‿4‿1 ≡ (1+⊑⍋slot)‿(1+⊑⍒slot) ∾ (1+↕4) ∾ 1+/mod

# keys mint statement by statement, 1 + 1 + 4 + 1 on top of rows 0..4
Mint ← {𝕩∾(1+⌈´𝕩)+↕𝕨}
! (↕12) ≡ 1 Mint 4 Mint 1 Mint 1 Mint ↕5
