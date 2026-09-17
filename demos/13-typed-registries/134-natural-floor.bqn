# ano: Alive , hp -= 55
#      Alive , hp -= 55
# Semantics: nat is ℕ∩[0,2^53]. Every committed write passes ⌊ then the clamp 2^53⌊0⌈ at the barrier. 60 and 10 and 30 park at 0 across two ticks; 200 decrements exactly, 145 then 90.
# Invariant: the column never holds a negative and never wraps.

natmax ← 9007199254740992   # 2^53, the last contiguous integer of the double
hp ← 60‿10‿200‿30

hp ↩ natmax⌊0⌈⌊ hp - 55
! hp ≡ 5‿0‿145‿0

hp ↩ natmax⌊0⌈⌊ hp - 55
! hp ≡ 0‿0‿90‿0

! ∧´ hp ≥ 0
