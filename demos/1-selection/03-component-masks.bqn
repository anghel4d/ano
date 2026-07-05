# Component masks (ex3).
# ano: Nord , Gold += 100
# ano: Dragon , Health = 0
# ano: Bandit & !Dead & Faction == :Bandit , Faction = :Hostile
# Semantics: bare names are presence masks combined by & | !; :Bandit is the enum symbol, Bandit the mask — Faction == :Bandit is a pointwise mask against the constant.
# Invariant: each statement rewrites exactly its masked rows; assignment scatters the constant under mask-compress ((v¨)⌾(mask⊸/)), never scalar-Under.

symBandit‿symHostile‿symFriendly‿symNone ← ↕4   # enum codes standing in for :Bandit :Hostile :Friendly :None

nord    ← 1‿0‿1‿0‿1‿0
dragon  ← 0‿1‿0‿0‿1‿0
bandit  ← 0‿0‿1‿1‿0‿1
dead    ← 0‿0‿0‿1‿0‿0
gold    ← 10‿20‿30‿40‿50‿60
health  ← 90‿80‿70‿60‿50‿40
faction ← symNone‿symNone‿symBandit‿symBandit‿symFriendly‿symFriendly

gold +↩ 100×nord
health ↩ (0¨)⌾(dragon⊸/) health
faction ↩ (symHostile¨)⌾((bandit ∧ (¬dead) ∧ faction=symBandit)⊸/) faction

! 110‿20‿130‿40‿150‿60 ≡ gold
! 90‿0‿70‿60‿0‿40 ≡ health
! symNone‿symNone‿symHostile‿symBandit‿symFriendly‿symFriendly ≡ faction
# the dead bandit (row 3) kept its faction: !Dead removed it from the selection
! symBandit ≡ 3⊑faction
