# ano: Grunt , spawn Grunt
# Semantics: the unique line takes a kind word; `unique id nat` seals the naturals AND pairwise distinctness at rest, `unique slot int` admits negative keys. Spawn mints 1+max per unique column (the ¯1 seed floors the empty max), so the mint respects either carrier: a nat key column mints natural, a negative-keyed int column mints past its maximum.
# Invariant: post-spawn, both key columns stay pairwise-distinct and in-carrier.

id    ← 3‿5
slot  ← ¯5‿¯3
grunt ← 1‿0

tot ← +´grunt                     # one fresh row per selected source
id    ∾↩ (1+⌈´¯1∾id)+↕tot
slot  ∾↩ (1+⌈´¯1∾slot)+↕tot
grunt ∾↩ tot⥊1

! id ≡ 3‿5‿6
! slot ≡ ¯5‿¯3‿0
! grunt ≡ 1‿0‿1

# the intersection holds after the mint: distinct ∧ in-carrier, both columns
! (≠id) = ≠⍷id
! (≠slot) = ≠⍷slot
! ∧´ (id ≥ 0) ∧ id = ⌊id
! ∧´ slot = ⌊slot
