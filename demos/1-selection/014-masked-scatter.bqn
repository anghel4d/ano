# Assignment scatter, the corrected idiom (spec §8).
# ano: Bandit , Faction = :Hostile
# Semantics: assignment overwrites where the mask holds; the mask must become indices before the scatter — APL's faction[bandit] ← `hostile indexes by the mask's 0s and 1s, so the fix is faction[⍸bandit], and the BQN model is (value)-under-mask-compress.
# Invariant: under-mask-compress equals a hand scatter at /Bandit; the buggy mask-as-index read writes only rows 0 and 1.

symBandit‿symHostile‿symFriendly ← ↕3   # enum codes standing in for :Bandit :Hostile :Friendly

bandit  ← 0‿1‿0‿1‿1‿0
faction ← symFriendly‿symBandit‿symFriendly‿symBandit‿symBandit‿symFriendly

# corrected: (hostile value)-under-mask-compress
got ← (symHostile¨)⌾(bandit⊸/) faction
# the same scatter spelled index-from-mask: /bandit is the index list
alt ← (symHostile¨)⌾((/bandit)⊸⊏) faction

expected ← symFriendly‿symHostile‿symFriendly‿symHostile‿symHostile‿symFriendly
! expected ≡ got
! expected ≡ alt
! 1‿3‿4 ≡ /bandit

# the bug: faction[bandit] reads the mask 0 1 0 1 1 0 as an index list, whose net writes hit rows 0 and 1 only
bug ← (symHostile¨)⌾((⍷bandit)⊸⊏) faction   # ⍷ dedups the misread index list to 0‿1, APL's net effect
"mask-as-index is not the scatter" ! ¬ expected ≡ bug
! symHostile‿symHostile‿symFriendly‿symBandit‿symBandit‿symFriendly ≡ bug
