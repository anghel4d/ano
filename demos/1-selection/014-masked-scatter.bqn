# ano: Bandit , Faction = :Hostile

symBandit‿symHostile‿symFriendly ← ↕3   # enum codes standing in for :Bandit :Hostile :Friendly

bandit  ← 0‿1‿0‿1‿1‿0
faction ← symFriendly‿symBandit‿symFriendly‿symBandit‿symBandit‿symFriendly

got ← (symHostile¨)⌾(bandit⊸/) faction
# the same scatter spelled index-from-mask: /bandit is the index list
alt ← (symHostile¨)⌾((/bandit)⊸⊏) faction

expected ← symFriendly‿symHostile‿symFriendly‿symHostile‿symHostile‿symFriendly
! expected ≡ got
! expected ≡ alt
! 1‿3‿4 ≡ /bandit

bug ← (symHostile¨)⌾((⍷bandit)⊸⊏) faction
"mask-as-index is not the scatter" ! ¬ expected ≡ bug
! symHostile‿symHostile‿symFriendly‿symBandit‿symBandit‿symFriendly ≡ bug
