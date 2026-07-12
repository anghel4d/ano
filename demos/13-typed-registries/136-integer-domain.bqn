# Integer domain (t4): int is ℤ∩[±2^53]; writes floor toward −∞.
# ano: Alive , drift *= 2
#      Alive , drift /= 4
# Semantics: the signed twin of nat. ¯6÷4 = ¯1.5 floors to ¯2 and ¯2÷4 = ¯0.5 floors to ¯1 — ⌊ is floor, not truncation, so the grid is walked the APL way. The clamp is symmetric at ±2^53.

natmax ← 9007199254740992   # 2^53
drift ← ¯3‿2‿¯1‿0

drift ↩ natmax⌊(-natmax)⌈⌊ drift × 2
! drift ≡ ¯6‿4‿¯2‿0

drift ↩ natmax⌊(-natmax)⌈⌊ drift ÷ 4
! drift ≡ ¯2‿1‿¯1‿0

! ∧´ drift = ⌊drift
