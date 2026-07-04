# The cast block (w1): Noita's draw algorithm is a scan.
# Ano: Spell , Bal = 1 + (+\ (DrawAdd - 1) @ Spell)
#      Spell , MinBal = 0 - (max\ (0 - Bal) @ Spell)
#      Spell @ deck & (Slot == 0 | prev.MinBal > 0) , +Member
# Semantics: each card carries drawAdd — a multicast adds 2 or 3 draws, a modifier 1, a projectile 0 — and the evaluator's state is a running balance: start at 1, each draw pays 1 and credits drawAdd. That is a scan (§14) along the slot order, and the block is the prefix where the balance's running MINIMUM stays positive. min-scan spells as the negated max-scan; membership gathers the previous slot's minimum through the prev shift over the deck view, slot 0 entering by fiat.
# Invariant: [Double, SB, SB, SB, DP, SB] draws exactly [Double, SB, SB]; the trap wand [SB, Double, SB, SB] ends at slot 0 even though the balance recovers to 1 — bal >= 0 marks three slots and is pinned WRONG; the running minimum is the block.

Draw ← {1 + +` 𝕩-1}                       # the balance after each draw
Member ← {((↕≠𝕩)=0) ∨ 0 < » ⌊` 𝕩}         # positive prefix-min, shifted; slot 0 by fiat

# ── the main wand: Double, SB, SB, SB, DP, SB
bal ← Draw 2‿0‿0‿0‿1‿0
! 2‿1‿0‿¯1‿¯1‿¯2 ≡ bal
! (⌊`bal) ≡ -⌈`-bal                       # min-scan is the negated max-scan, the twin's spelling
! 1‿1‿1‿0‿0‿0 ≡ Member bal                # Double + its two bolts; the rest waits for the next block

# ── the recovery trap: SB, Double, SB, SB
bal2 ← Draw 0‿2‿0‿0
! 0‿1‿0‿¯1 ≡ bal2                         # the balance climbs back to 1 after the block died
! 1‿0‿0‿0 ≡ Member bal2                   # the block is one bolt
! (0≤bal2) ≢ Member bal2                  # bal >= 0 would draw three slots: the wrong evaluator, pinned
