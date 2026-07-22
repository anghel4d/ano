# ano: Enemy |> order by Threat desc |> take 5 , +Targeted
# semantics: filter to Enemy, order that sub-column by Threat descending, take the first five, set Targeted on those world rows; grade indices live in the filtered index space and must be mapped through /enemy before touching a world-aligned column.
# invariant: exactly the five highest-threat enemies gain Targeted; non-enemies never do, however high their threat; prior Targeted rows survive the or.

# world: 8 entities, columns
enemy    ← 1‿0‿1‿1‿1‿0‿1‿1          # 6 enemies; rows 1,5 are not enemies
threat   ← 40‿95‿10‿75‿60‿99‿85‿20  # the two non-enemies carry the highest raw threat
targeted ← 0‿1‿0‿0‿0‿0‿0‿0          # row 1 pre-targeted (a non-enemy), must survive

# or-ing a raw 5-element index list into an 8-long boolean column is not a mask at all;
# and even as membership the indices are filtered-space, not world-space:
buggy ← (↕≠threat) ∊ 5↑⍒enemy/threat            # membership over untranslated indices
chosen ← (/enemy) ⊏˜ 5↑⍒enemy/threat            # world rows of the top-5 enemies
fixed  ← (↕≠threat) ∊ chosen

! ¬ buggy ≡ fixed
! 1‿0‿1‿1‿1‿1‿0‿0 ≡ buggy
! 6‿3‿4‿0‿7 ≡ chosen
! 1‿0‿0‿1‿1‿0‿1‿1 ≡ fixed

# apply the statement
targeted ∨↩ fixed

# expected post-state: top-5 enemy rows set, prior row 1 kept, enemy row 2 still untargeted
! 1‿1‿0‿1‿1‿0‿1‿1 ≡ targeted
