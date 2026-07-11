# Named selections (ex9).
# ano: def master = Human & Nord & TwoHanded > 60
# ano: def rich   = Gold > 10000
# ano: master , Gold += 1000
# ano: rich , Faction = :Hostile ; +Marked
# ano: master & rich , +Legendary
# Semantics: a def names a predicate, not a saved mask — it folds into each statement's gather, so statement 1's scatter can move an entity into rich for statement 2; ;-batched effects share one barrier.
# Invariant: the entity at 9500 gold is rich only after the masters' +1000 commits.

symHostile‿symFriendly ← ↕2         # enum codes standing in for :Hostile :Friendly

human     ← 1‿1‿1‿0‿1
nord      ← 1‿1‿0‿1‿1
twoHanded ← 80‿61‿90‿70‿50
gold      ← 9500‿2000‿20000‿15000‿500
faction   ← symFriendly‿symFriendly‿symFriendly‿symFriendly‿symFriendly
marked    ← 0‿0‿0‿0‿0
legendary ← 0‿0‿0‿0‿0

IsMaster ← {𝕤 ⋄ human ∧ nord ∧ twoHanded>60}   # defs are predicates, re-gathered per statement
IsRich   ← {𝕤 ⋄ gold > 10000}

# master , Gold += 1000
gold +↩ 1000 × IsMaster @

# rich , Faction = :Hostile ; +Marked  — one barrier, two commuting effects on one mask
rich ← IsRich @
faction ↩ (symHostile¨)⌾(rich⊸/) faction
marked ∨↩ rich

# master & rich , +Legendary
legendary ∨↩ (IsMaster @) ∧ IsRich @

! 10500‿3000‿20000‿15000‿500 ≡ gold
! symHostile‿symFriendly‿symHostile‿symHostile‿symFriendly ≡ faction
! 1‿0‿1‿1‿0 ≡ marked
! 1‿0‿0‿0‿0 ≡ legendary
# entity 0 crossed 10000 only via statement 1's scatter; the def re-gathered, a saved mask would have missed it
"def is a predicate, not a snapshot" ! 1 ≡ 0⊑marked
