# Natural grain (t3): nat floors to the integer grid; the ceiling is 2^53.
# ano: Alive , fuel *= 4
#      Alive , fuel /= 8
# Semantics: 2^52 × 4 = 2^54 crosses the ceiling and clamps to 2^53; 2^53 ÷ 8 = 2^50 is then exact. 28 ÷ 8 = 3.5 floors to 3. The grain is ⌊, floor — division behaves as integer division.

natmax ← 9007199254740992   # 2^53
fuel ← 4503599627370496‿10‿7   # 2^52, 10, 7

fuel ↩ natmax⌊0⌈⌊ fuel × 4
! fuel ≡ natmax‿40‿28

fuel ↩ natmax⌊0⌈⌊ fuel ÷ 8
! fuel ≡ 1125899906842624‿5‿3   # 2^50, exact

! ∧´ fuel = ⌊fuel
