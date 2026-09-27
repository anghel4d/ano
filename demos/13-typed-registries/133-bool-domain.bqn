# ano: Alive , alarm = alarm + trips
#      Trips > 2 , alarm = alarm - 5
# Semantics: a bool column admits {0,1}. The loader refuses out-of-domain data at rest; a committed write passes the retraction 0<x at the barrier: 2→1, 3→1, ¯4→0. The retraction is a barrier property — merged writes retract once, at commit — and applies to the whole committed column, which on off-mask cells (in-domain by the load seal) is the identity.
# Invariant: post-state is boolean everywhere, and off-mask cells are bit-identical to pre-state.

alive ← 1‿1‿1‿1
alarm ← 0‿1‿0‿1
trips ← 2‿0‿3‿¯1

alarm ↩ 0< alarm + trips          # wire values 2‿1‿3‿0 retract to 1‿1‿1‿0
! alarm ≡ 1‿1‿1‿0

m ← trips > 2
alarm0 ← alarm
alarm ↩ 0< alarm - 5 × m          # row 2: ¯4 retracts to 0; off-mask rows pass 0< unchanged
! alarm ≡ 1‿1‿0‿0

# the retraction is idempotent on its own image: a sealed column is a fixpoint
! alarm ≡ 0< alarm
! ((¬m)/alarm) ≡ (¬m)/alarm0
