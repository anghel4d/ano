# ano: Nord & TwoHanded > 60 , Gold += 1000
# Semantics: the selection mask Nord ∧ TwoHanded>60 gates a masked add down the Gold column.
# Invariant: selected rows gain exactly 1000, unselected rows are untouched.

nord      ← 1‿1‿0‿1‿0‿1
twoHanded ← 80‿55‿70‿61‿90‿60
gold      ← 100‿200‿300‿400‿500‿600

gold +↩ 1000 × nord ∧ twoHanded>60

! 1100‿200‿300‿1400‿500‿600 ≡ gold
