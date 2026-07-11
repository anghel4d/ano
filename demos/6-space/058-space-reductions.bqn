# Space reductions and scans (ex30, spec §20). Modeled at 8 8; the doc's 64 64 is the same statement at scale.
# Ano: +/ Elevation @ 64 64                 -- Version A and B
#      max/ Threat @ Frontier               -- Version A and B
#      +\ Cost @ 64 64                      -- Version A: leading-axis scan
#      scan2(+) Cost @ 64 64                -- Version B: summed-area table
#      max\ Height @ (Eye + ↕n * north)     -- Version A and B
# Semantics: a fold behind @ is scoped-global — one scalar over the lattice or named-region scope; a scan is a read-side column over the scope's order; +\ scans the leading axis only, scan2(+) scans both axes, and the two are different fields.
# Invariant: fold totals and scoped max match hand-computed values; the one-axis scan and the summed-area table are asserted cell-by-cell AND asserted unequal; the sightline running max matches the hand-computed profile.

# +/ Elevation @ 8 8 : one scalar over the whole lattice
elevation ← 8‿8⥊↕64
! 2016 = +´⥊elevation          # 0+1+…+63 = 63×64÷2

# max/ Threat @ Frontier : one scalar over the named-region scope, not the world max
threat   ← ⟨3,9,4,7,5,8,2,6⟩
frontier ← ⟨1,0,1,1,0,0,1,1⟩
! 7 = ⌈´frontier/threat
! 9 = ⌈´threat                 # world max sits outside the frontier; the scope matters

# Version A vs Version B on a uniform cost field: one-axis scan gives row i ≡ i+1,
# the summed-area table gives cell (i,j) = (i+1)(j+1)
cost ← 8‿8⥊1
oneAxis ← +`cost
sat     ← +`˘ +`cost
! oneAxis ≡ (1+↕8) ×⌜ 8⥊1
! sat     ≡ (1+↕8) ×⌜ 1+↕8
! oneAxis ≢ sat                # Version A and Version B are different fields
! 64 = ¯1‿¯1⊑sat               # SAT corner is the whole-lattice fold
! (+`+`˘cost) ≡ +`˘+`cost      # axis order commutes

# the same distinction on a non-uniform 3 3 field, every cell hand-computed
cost2 ← 3‿3⥊1+↕9
! (+`cost2)     ≡ 3‿3⥊⟨1,2,3, 5,7,9, 12,15,18⟩
! (+`˘+`cost2)  ≡ 3‿3⥊⟨1,3,6, 5,12,21, 12,27,45⟩
! (+`cost2) ≢ +`˘+`cost2

# max\ Height @ (Eye + ↕n * north) : running max along the ordered sightline
ray ← ⟨2,5,3,7,6,7,9,4⟩
! (⌈`ray) ≡ ⟨2,5,5,7,7,7,9,9⟩
