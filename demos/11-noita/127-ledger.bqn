# Ano: Wand , Mana += 30 * (#/ (Spell & AddMana)) ; Mana -= +/ ManaCost @ Spell
#      Wand , Delay = +/ CastDelay @ Spell
#      Wand , Recharge = +/ RechargeTime @ Spell
# Invariant: wand [Add Mana, SB, SB, Chainsaw] closes mana at 50+30-11 = 69 in either commit order; delay sums to -7; recharge sums to exactly 0. Flavor integers, not the wiki's.

am ← 1‿0‿0‿0 ⋄ mc ← 0‿5‿5‿1 ⋄ cd ← 0‿5‿5‿¯17 ⋄ rt ← 0‿10‿10‿¯20

! 69 = (50 + 30×+´am) - +´mc              # credit committed first
! 69 = (50 - +´mc) + 30×+´am              # debit committed first: additive writes merge, order-free

! ¯7 = +´cd                               # net-negative cast delay: data, clamped only at schedule time
! 0 = +´rt                                # the Chainsaw cancels the bolts' recharge exactly
