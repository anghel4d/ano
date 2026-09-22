
xs ← ¯4+↕8
Sin ← •math.Sin

ridge ← (Sin xs÷8) +⌜ Sin xs÷8

! 1e¯12 > | (0‿0⊑ridge) - ¯0.958851077208406
! 1e¯12 > | (0‿7⊑ridge) - ¯0.11315300951815543
! 1e¯12 > | (7‿0⊑ridge) - ¯0.11315300951815543
! 1e¯12 > | (7‿7⊑ridge) - 0.7325450581720951

wrong ← (2×Sin xs÷8) +⌜ 8⥊0
! 1e¯12 > | (0‿7⊑wrong) - ¯0.958851077208406
! ridge ≢ wrong

basin ← ridge<0
! 36 = +´⥊basin

water ← 64⥊7
water ↩ (100¨)⌾((⥊basin)⊸/) water
! water ≡ (7×¬⥊basin)+100×⥊basin
! 36 = +´water=100

peak ← ridge>0.5
! 3 = +´⥊peak
! ⟨55,62,63⟩ ≡ /⥊peak

height ← ⟨0,10,20,30,80,90,100,110⟩ ×⌜ 8⥊1
nbr   ← (7⌊1+↕8)⊏height
slope ← |height-nbr
! slope ≡ ⟨10,10,10,50,10,10,10,0⟩ ×⌜ 8⥊1

cliff ← slope>30
! 8 = +´⥊cliff
! (24+↕8) ≡ /⥊cliff

! 8‿8 ≡ ≢ridge
! 8‿8 ≡ ≢basin
! 8‿8 ≡ ≢8‿8⥊water
! 8‿8 ≡ ≢height
! 8‿8 ≡ ≢slope
! 8‿8 ≡ ≢cliff
