# ano: Spawner |> expand Count , spawn Minion
# Semantics: expand Count replicates each selected source Count times before the effect — the one stage whose output has multiplicity, so it exists only feeding a generator; spawn Minion then mints one fresh key per replicated row.

keys    ← 0‿1‿2‿3‿4
spawner ← 1‿1‿1‿0‿0
count   ← 2‿0‿3‿0‿0

src      ← /spawner
expanded ← (spawner/count) / src   # the docs' Count/Spawner
! expanded ≡ 0‿0‿2‿2‿2             # hand-computed
! (+´spawner/count) = ≠expanded

# The flat-map reading: each source repeated its own count, then joined.
flat ← ∾ (spawner/count) ⥊¨ src
! flat ≡ expanded

# spawn Minion over the expanded stream: one fresh key per row...
newA ← (1+⌈´keys) + ↕≠expanded
newB ← (1+⌈´keys) + ↕+´spawner/count
! newA ≡ newB
! ∧´ ¬ newA ∊ keys                 # still generation: the keys are fresh
