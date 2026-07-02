# Spec §19 — The true recurrence: an order-carried scan with a two-back state, the thing one barrier step cannot compute.
# ano:  12 , offset = fib(index)
# Semantics: Version B is canonical — the recurrence runs inside a registered host function, outside the calculus; a true recurrence is a scan whose step need not be associative, carrying state along the line's order, and a scan that feeds its own column's scatter breaks the barrier by construction (decision record, recurrence gloss).
# Invariant: the ordered scan from seed 1 1 yields the Fibonacci column; one shift-add barrier step from the same seed does not.

n ← 12
# ordered scan: state ⟨fib(i), fib(i+1)⟩ carried left to right; each step reads its own previous result — a carry, which the barrier forbids
states ← {⟨1⊑𝕨, +´𝕨⟩}` ⟨1‿1⟩∾(n-1)⥊0
fib    ← ⊑¨states
! 1‿1‿2‿3‿5‿8‿13‿21‿34‿55‿89‿144 ≡ fib

# the same seed under one barrier step — shift-add over pre-state — is not the recurrence
seed ← 1‿1∾(n-2)⥊0
! (0‿1‿2‿1∾(n-4)⥊0) ≡ (»seed)+»»seed
! ¬ fib ≡ (»seed)+»»seed
