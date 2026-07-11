# The wrap (w2): the deck is a ring; wrapping is modular index arithmetic, and no row ever moves.
# Ano: Spell , DrawPos = (Slot - 1) % 2
#      Spell , Bal = 1 + (scan(+) (DrawAdd - 1) along rot)
#      Spell & Proj & DrawPos == 1 , Damage += 10
# Semantics: the cast pointer p turns the slot line into a rotation — draw order p⌽↕n, per-slot position (slot - p) mod n; both are projections of the regenerable slot ground (Tier 1), so the wrap costs nothing and moves nothing. The balance runs along the rotation via scan-along; its write-back conjugates through the order's grade — sort, act, unsort, the Tier-2 law. The modifier's payload is whatever the rotation draws next, which across the seam is the wand's FIRST slot.
# Invariant: with wand [SB, DP] and the pointer at 1, the rotation is ⟨1,0⟩ and equals the modular form; the block draws the whole ring (DP then the wrapped SB); the bolt at drawPos 1 is slot 0 — the modifier hung at the buffer's end lands on the front bolt, damage 3 to 13.

p ← 1 ⋄ n ← 2
ord ← n | p + ↕n                          # draw order by modular arithmetic
! ord ≡ p ⌽ ↕n                            # the rotation and the mod form are one projection
dp ← n | (↕n) - p                         # each slot's position in the draw
! 1‿0 ≡ dp

da ← 0‿1                                  # SB draws nothing, DP draws one more
bal ← (⍋ord) ⊏ 1 + +` (ord⊏da) - 1        # scan in draw order, unsorted back to slots
! 0‿1 ≡ bal                               # DP holds the block open, the wrapped bolt closes it

! ⟨0⟩ ≡ / 1 = dp                          # drawn after the modifier: slot 0, the front
proj ← 1‿0
! 13‿0 ≡ 3‿0 + 10 × proj ∧ 1 = dp         # the wrap trick lands
