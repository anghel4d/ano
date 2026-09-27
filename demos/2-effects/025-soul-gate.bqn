# ano: Nord & Dead & Soul , Soul = 0 ; spawn Ghost
# Semantics: both effects read the one gathered mask (souled dead Nords, rows 0 and 3); the write zeroes the fuel while the spawn mints; minted ghosts take the type-zero soul, so the next gather finds nothing.
# Invariant: the step is linear (one ghost per soul, ever) and terminating — Step ∘ Step ≡ Step.

Step ← {⟨k,n,d,g,s⟩:
  m ← n∧d∧s
  c ← +´m
  ⟨k∾(1+⌈´k)+↕c, n∾c⥊1, d∾c⥊1, g∾c⥊1, (s×¬m)∾c⥊0⟩
}

w0 ← ⟨0‿1‿2‿3‿4‿5, 1‿1‿0‿1‿0‿0, 1‿0‿0‿1‿0‿1, 0‿0‿0‿0‿0‿0, 1‿1‿0‿1‿0‿0⟩
w1 ← Step w0

! 8 ≡ ≠⊑w1
! 0‿1‿2‿3‿4‿5‿6‿7 ≡ 0⊑w1
! 1‿1‿0‿1‿0‿0‿1‿1 ≡ 1⊑w1
! 1‿0‿0‿1‿0‿1‿1‿1 ≡ 2⊑w1
! 0‿0‿0‿0‿0‿0‿1‿1 ≡ 3⊑w1
! 0‿1‿0‿0‿0‿0‿0‿0 ≡ 4⊑w1
"fixed point" ! (Step w1) ≡ w1
