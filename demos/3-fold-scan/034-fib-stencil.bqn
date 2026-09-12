
n ← 12
Step ← {(»𝕩) + »»𝕩}

! (n⥊0) ≡ Step n⥊0

seed ← 1‿1∾(n-2)⥊0
fib  ← 1‿1‿2‿3‿5‿8‿13‿21‿34‿55‿89‿144
! (0‿1‿2‿1∾(n-4)⥊0) ≡ Step seed
! ¬ fib ≡ Step seed

! (0‿0‿1‿3‿3‿1∾(n-6)⥊0) ≡ Step⍟2 seed
! (((n-1)⥊0)∾1) ≡ Step⍟(n-1) seed
! ¬ fib ≡ Step⍟(n-1) seed
