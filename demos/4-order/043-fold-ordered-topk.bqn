# ex26 - Fold over ordered top-k (Version B)
# ano: +/ threat @ (Enemy |> order by dps desc |> take 10)
# semantics: filter to Enemy, order the filtered rows by dps descending, take ten, sum their threat; the grade of the filtered dps column indexes the filtered threat column, never the unfiltered one.
# invariant: the fold sums threat over exactly the ten highest-dps enemies; indexing the unfiltered threat with filtered-space grade indices picks masked-out rows and gives a different sum.

# world: 14 entities, 12 enemies (take 10 must actually drop two)
enemy  ← 0‿1‿1‿0‿1‿1‿1‿1‿1‿1‿1‿1‿1‿1                       # rows 0,3 are not enemies
dps    ← 999‿50‿10‿888‿70‿20‿90‿30‿80‿60‿40‿95‿65‿15       # non-enemies carry huge dps
threat ← 1000‿5‿7‿2000‿11‿13‿17‿19‿23‿29‿31‿37‿41‿43       # and huge threat, so the bug shows

# doc bug (ex26): +´ threat⊏˜10↑⍒Enemy/dps
# the grade indices are into the FILTERED dps column, but they index the UNFILTERED threat
buggy ← +´ threat ⊏˜ 10↑⍒enemy/dps
# fix: grade of the filtered column indexes the filtered column
fixed ← +´ (enemy/threat) ⊏˜ 10↑⍒enemy/dps

# by hand: enemy rows by dps desc are r11(95) r6(90) r8(80) r4(70) r12(65) r9(60) r1(50) r10(40) r7(30) r5(20); r13(15) and r2(10) drop
# fixed = 37+17+23+11+41+29+5+31+19+13 = 226
! 226 ≡ fixed
# buggy reads world rows 9 4 6 2 10 7 0 8 5 3, dragging in the non-enemies (1000, 2000)
! 3150 ≡ buggy
! ¬ buggy ≡ fixed
