
nord      ← 1‿0‿1‿1‿0‿1‿0‿0
gold      ← 100‿50‿20‿5‿0‿25‿7‿3
hits      ← 0‿1‿1‿0‿0‿1‿0‿0
resist    ← 0‿0.25‿0.5‿0‿0‿0.75‿0‿0
party     ← 1‿1‿0‿1‿0‿0‿0‿0
alive     ← 1‿1‿0‿1‿1‿1‿1‿0
forest    ← 0‿0‿0‿0‿1‿1‿1‿1
burning   ← 0‿0‿1‿0‿0‿1‿0‿0
twoHanded ← 70‿80‿61‿60‿90‿10‿65‿50

! 150     ≡ +´ nord/gold
! 0.09375 ≡ ×´ hits/(1-resist)
! 1       ≡ ∧´ party/alive
! 1       ≡ ∨´ forest/burning
! 2       ≡ +´ nord∧twoHanded>60

! (+´ nord/gold) ≡ +´ ⌽nord/gold
! (×´ hits/(1-resist)) ≡ ×´ ⌽hits/(1-resist)

none ← 8⥊0
! 0 ≡ +´ none/gold
! 1 ≡ ×´ none/(1-resist)
! 1 ≡ ∧´ none/alive
! 0 ≡ ∨´ none/burning
