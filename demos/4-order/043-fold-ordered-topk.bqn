# ano: +/ threat @ (Enemy |> order by dps desc |> take 10)
# semantics: filter to Enemy, order the filtered rows by dps descending, take ten, sum their threat; the grade of the filtered dps column indexes the filtered threat column, never the unfiltered one.
# invariant: the fold sums threat over exactly the ten highest-dps enemies; indexing the unfiltered threat with filtered-space grade indices picks masked-out rows and gives a different sum.

# world: 14 entities, 12 enemies (take 10 must actually drop two)
enemy  ← 0‿1‿1‿0‿1‿1‿1‿1‿1‿1‿1‿1‿1‿1                       # rows 0,3 are not enemies
dps    ← 999‿50‿10‿888‿70‿20‿90‿30‿80‿60‿40‿95‿65‿15       # non-enemies carry huge dps
threat ← 1000‿5‿7‿2000‿11‿13‿17‿19‿23‿29‿31‿37‿41‿43

# the grade indices are into the FILTERED dps column, but they index the UNFILTERED threat
buggy ← +´ threat ⊏˜ 10↑⍒enemy/dps
fixed ← +´ (enemy/threat) ⊏˜ 10↑⍒enemy/dps

! 226 ≡ fixed
! 3150 ≡ buggy
! ¬ buggy ≡ fixed
