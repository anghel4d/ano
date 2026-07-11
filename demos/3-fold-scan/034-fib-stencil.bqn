# Spec §19 — One barrier step of the Fibonacci statement: a shift-add stencil over pre-state, not a recurrence.
# ano:  12 , offset = prev.offset + prev.prev.offset
# Semantics: the comma is gather-effect-scatter and every read observes pre-state, so prev is a shift, not a carry; on a freshly minted line the pre-state of offset is undefined-or-zero, the statement computes one parallel shift-and-add, and iterating it k times gives k stencil steps, never the order-carried sequence — Version B, offset = fib(index), is the honest form (decision record, recurrence gloss).
# Invariant: one step from the zero pre-state is zero; one step from the fib seed is the shift-add column, not the fib prefix; k steps give the binomial ridge z^k(1+z)^(k+1), never fib.

n ← 12
Step ← {(»𝕩) + »»𝕩}          # one barrier step: offset = prev.offset + prev.prev.offset, all reads over old values

# freshly minted line: pre-state undefined-or-zero cannot generate Fibonacci
! (n⥊0) ≡ Step n⥊0

# same seed as the true recurrence: 1 1 0 ...
seed ← 1‿1∾(n-2)⥊0
fib  ← 1‿1‿2‿3‿5‿8‿13‿21‿34‿55‿89‿144
! (0‿1‿2‿1∾(n-4)⥊0) ≡ Step seed         # one shift-add step
! ¬ fib ≡ Step seed                     # differs from the recurrence after one step from the same seed

# k statements are k stencil steps: the mass shifts and spreads binomially, no carry ever forms
! (0‿0‿1‿3‿3‿1∾(n-6)⥊0) ≡ Step⍟2 seed
! (((n-1)⥊0)∾1) ≡ Step⍟(n-1) seed       # z^11(1+z)^12 truncated to the line
! ¬ fib ≡ Step⍟(n-1) seed
