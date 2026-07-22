# ano: Alive , hp += 30
#      Hp > 99 , hp -= 200
# Semantics: `range hp 0 100` is a value refinement beside the kind word — rest data seals at load, committed writes clamp at the barrier, fractions pass (the carrier is num). 80 + 30 and 95 + 30 park at the cap; the overkill then parks the same rows at the floor.
# Invariant: post-state sits inside [0, 100] and 50.5's fraction survived the first write.

hp ← 50.5‿80‿20‿95

hp ↩ 100⌊0⌈ hp + 30
! hp ≡ 80.5‿100‿50‿100

m ← hp > 99
hp ↩ 100⌊0⌈ hp - 200 × m
! hp ≡ 80.5‿0‿50‿0

! ∧´ (hp ≥ 0) ∧ hp ≤ 100
