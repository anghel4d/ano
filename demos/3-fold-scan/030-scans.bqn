# ano: the running forms over weight 3 1 8 4 9 — sum, ordered subtraction, Greater/Lesser with their bridges, the mean's prefixes, and the running count over a mask.
# Semantics: BQN's ` is already the left scan, so every scan holds A13's recurrence natively; the mean is a prefix machine whose emit is sum-so-far over count-so-far; the count consumes admitted rows.
# Invariant: the last element of every scan is its fold; the empty scan is empty; the all-false count never advances.

w ← 3‿1‿8‿4‿9
! 3‿4‿12‿16‿25 ≡ +` w
! 3‿2‿¯6‿¯10‿¯19 ≡ -` w
! 3‿3‿8‿8‿9 ≡ ⌈` w
! 3‿1‿1‿1‿1 ≡ ⌊` w
! 3‿2‿4‿4‿5 ≡ (+`w)÷+`1¨w                  # the mean machine's prefixes
! (¯1⊑+`w) ≡ +˜´⌽(0∾w)                     # fold = last prefix
mixed ← 1‿0‿1‿1‿0‿0‿1‿0
! 1‿1‿2‿3‿3‿3‿4‿4 ≡ +` mixed               # running cardinality over admitted rows
! ⟨⟩ ≡ +` ⟨⟩                               # the empty scan is an empty column
! 0‿0‿0‿0‿0‿0‿0‿0 ≡ +` 8⥊0                 # nonempty scope, count never advances
