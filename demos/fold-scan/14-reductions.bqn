# Ex14 — Reductions: collapse a selected vector to a scalar under an associative reducer.
# ano:  +/ Gold @ Nord
#       */ (1 - Resist) @ Hits
#       &/ Alive @ Party
#       |/ Burning @ Forest
#       #/ (Nord & TwoHanded > 60)
# Semantics: fold/ col @ scope is the scoped-global fold, one scalar over the scope; #/ counts a mask; an empty scope yields the fold's registered identity.
# Invariant: each fold equals its hand-computed scalar, gather order cannot matter under the associative+commutative reducer, and the empty scope returns the identity.

# world: 8 entities as columns
nord      ← 1‿0‿1‿1‿0‿1‿0‿0
gold      ← 100‿50‿20‿5‿0‿25‿7‿3
hits      ← 0‿1‿1‿0‿0‿1‿0‿0
resist    ← 0‿0.25‿0.5‿0‿0‿0.75‿0‿0
party     ← 1‿1‿0‿1‿0‿0‿0‿0
alive     ← 1‿1‿0‿1‿1‿1‿1‿0
forest    ← 0‿0‿0‿0‿1‿1‿1‿1
burning   ← 0‿0‿1‿0‿0‿1‿0‿0
twoHanded ← 70‿80‿61‿60‿90‿10‿65‿50

! 150     ≡ +´ nord/gold                # 100+20+5+25
! 0.09375 ≡ ×´ hits/(1-resist)          # 0.75×0.5×0.25
! 1       ≡ ∧´ party/alive
! 1       ≡ ∨´ forest/burning
! 2       ≡ +´ nord∧twoHanded>60        # entities 0 and 2

# order-independence: an associative+commutative reducer cannot see gather order
! (+´ nord/gold) ≡ +´ ⌽nord/gold
! (×´ hits/(1-resist)) ≡ ×´ ⌽hits/(1-resist)

# empty scope: the registered identity
none ← 8⥊0
! 0 ≡ +´ none/gold
! 1 ≡ ×´ none/(1-resist)
! 1 ≡ ∧´ none/alive
! 0 ≡ ∨´ none/burning
