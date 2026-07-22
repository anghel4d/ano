
elevation ← 8‿8⥊↕64
! 2016 = +´⥊elevation

threat   ← ⟨3,9,4,7,5,8,2,6⟩
frontier ← ⟨1,0,1,1,0,0,1,1⟩
! 7 = ⌈´frontier/threat
! 9 = ⌈´threat

cost ← 8‿8⥊1
oneAxis ← +`cost
sat     ← +`˘ +`cost
! oneAxis ≡ (1+↕8) ×⌜ 8⥊1
! sat     ≡ (1+↕8) ×⌜ 1+↕8
! oneAxis ≢ sat
! 64 = ¯1‿¯1⊑sat
! (+`+`˘cost) ≡ +`˘+`cost

cost2 ← 3‿3⥊1+↕9
! (+`cost2)     ≡ 3‿3⥊⟨1,2,3, 5,7,9, 12,15,18⟩
! (+`˘+`cost2)  ≡ 3‿3⥊⟨1,3,6, 5,12,21, 12,27,45⟩
! (+`cost2) ≢ +`˘+`cost2

ray ← ⟨2,5,3,7,6,7,9,4⟩
! (⌈`ray) ≡ ⟨2,5,5,7,7,7,9,9⟩

! 8‿8 ≡ ≢elevation
! 8‿8 ≡ ≢cost
! 8‿8 ≡ ≢oneAxis
! 8‿8 ≡ ≢sat
! elevation ≡ 8‿8⥊↕64
! cost ≡ 8‿8⥊1
