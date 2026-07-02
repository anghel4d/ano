# Presence patterns (ex6).
# ano: (Nord, TwoHanded > 60) , Gold += 1000
# ano: (Nord, TwoHanded _) , +Trained
# ano: (Nord, !TwoHanded) , +Untrained
# Semantics: three presence states — present with a constrained value, present with any value, absent; a value predicate on a sparse component implies the presence guard.
# Invariant: rows lacking TwoHanded never satisfy the value form, even where the backing cell holds junk.

nord         ← 1‿1‿1‿0‿1‿1
hasTwoHanded ← 1‿1‿0‿1‿1‿0
twoHanded    ← 80‿55‿99‿70‿61‿77   # 99 and 77 are junk cells behind absent components
gold         ← 0‿0‿0‿0‿0‿0
trained      ← 0‿0‿0‿0‿0‿0
untrained    ← 0‿0‿0‿0‿0‿0

sel ← nord ∧ hasTwoHanded ∧ twoHanded>60
gold +↩ 1000×sel
trained   ∨↩ nord ∧ hasTwoHanded
untrained ∨↩ nord ∧ ¬hasTwoHanded

! 1000‿0‿0‿0‿1000‿0 ≡ gold
! 1‿1‿0‿0‿1‿0 ≡ trained
! 0‿0‿1‿0‿0‿1 ≡ untrained
# row 2's junk 99 never leaks through the presence guard
"presence guards the value predicate" ! 0 ≡ 2⊑sel
