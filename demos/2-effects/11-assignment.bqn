# Assignment (ex11): replace selected values in an aligned component column.
# ano: Bandit , Faction = :Hostile
#      Dead , Loot = :Empty
# Semantics: masked overwrite; the backtick names are symbol atoms (enum codes here), the constant lands on-mask only. Structural Under with a bare scalar is invalid BQN; the constant must be lifted per cell with each.
# Invariant: on-mask cells equal the constant, off-mask cells are bit-identical to pre-state.

sFriendly‿sBandit‿sHostile ← 0‿1‿2   # Faction symbols: :Friendly :Bandit :Hostile
sEmpty‿sCoins‿sGems        ← 0‿1‿2   # Loot symbols:    :Empty :Coins :Gems

bandit  ← 0‿1‿0‿1‿1‿0
dead    ← 1‿0‿0‿1‿0‿0
faction ← sFriendly‿sBandit‿sFriendly‿sBandit‿sBandit‿sFriendly
loot    ← sCoins‿sGems‿sCoins‿sGems‿sCoins‿sGems

faction0‿loot0 ← faction‿loot   # pre-state copies

faction ↩ (sHostile¨)⌾(bandit⊸/) faction
loot    ↩ (sEmpty¨)⌾(dead⊸/) loot

! faction ≡ sFriendly‿sHostile‿sFriendly‿sHostile‿sHostile‿sFriendly
! loot    ≡ sEmpty‿sGems‿sCoins‿sEmpty‿sCoins‿sGems

# off-mask cells untouched
! ((¬bandit)/faction) ≡ (¬bandit)/faction0
! ((¬dead)/loot)      ≡ (¬dead)/loot0
