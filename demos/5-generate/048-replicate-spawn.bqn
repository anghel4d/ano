# ano: Spawner , spawn Minion * Count
# ano: Nest    , spawn Egg * Fertility
# Semantics: for each selected source, mint Count fresh keys carrying the Minion components; the parent link column is the count-replicate of the source keys.
# Invariant: exactly +/Count new keys, all fresh — disjoint from every live key — parents in source order with multiplicity Count; a zero count emits nothing.

# World: keys 0..4; 0 1 2 are Spawners, 3 4 are bystanders.
keys    ← 0‿1‿2‿3‿4
spawner ← 1‿1‿1‿0‿0
count   ← 2‿0‿3‿0‿0               # per-source column, read under the Spawner mask

src    ← /spawner                  # selected source keys
minted ← (spawner/count) / src     # replicate: parent key per emitted row
new    ← (1+⌈´keys) + ↕≠minted     # fresh keys, one per emitted row

! 5 = ≠new                         # +/Count over the selection
! (+´ spawner/count) = ≠new
! minted ≡ 0‿0‿2‿2‿2               # hand-computed parent column
! ∧´ ¬ new ∊ keys                  # fresh: generation invents keys the live set never held
! ¬ ∨´ minted = 1                  # the zero-count spawner emits nothing

world ← keys ∾ new                 # the scatter commits at the statement barrier
! world ≡ 0‿1‿2‿3‿4‿5‿6‿7‿8‿9

# Nest line, same shape: Fertility counts eggs per nest.
nests     ← 100‿101‿102
fertility ← 1‿3‿0
eggparent ← fertility / nests
! eggparent ≡ 100‿101‿101‿101      # hand-computed
! (+´fertility) = ≠eggparent
