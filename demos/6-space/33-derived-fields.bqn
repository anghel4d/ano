# Spatial derived fields (ex33, spec §23). Version B canonical: the boundary policy is visible.
# Ano: def ridge = sin(x / 8) + sin(y / 8)
#      def basin = ridge < 0
#      def slope = abs(Height - neighbor(clamp).Height)
#      64 64 & ridge > 0.5 , spawn Peak
#      64 64 & basin , Water = 100
#      64 64 & slope > 30 , +Cliff
# Semantics: a def is a derived column over the coordinate columns or over a cell and its clamp-boundary neighbor shift; each statement masks on the derived field and scatters its effect onto the masked cells; the window's frame is pos = o + S·k with o = ¯4 ¯4 here so the basin is inhabited.
# Invariant: ridge mixes BOTH coordinates — sin(x/8)+sin(y/8), asserted against hand-computed corner values, never the §23 dfn's 2·sin(w/8) which reads one coordinate twice; basin, peak, and cliff land on exactly the hand-computed cells; the clamped boundary row has zero slope.

# 8x8 window, frame origin ¯4 ¯4: coordinate columns x (row) and y (column) span ¯4..3
xs ← ¯4+↕8
Sin ← •math.Sin

# def ridge = sin(x / 8) + sin(y / 8), one value per cell, x and y BOTH read
ridge ← (Sin xs÷8) +⌜ Sin xs÷8

# hand-computed corners: 2sin(-1/2), sin(-1/2)+sin(3/8), 2sin(3/8)
! 1e¯12 > | (0‿0⊑ridge) - ¯0.958851077208406
! 1e¯12 > | (0‿7⊑ridge) - ¯0.11315300951815543
! 1e¯12 > | (7‿0⊑ridge) - ¯0.11315300951815543
! 1e¯12 > | (7‿7⊑ridge) - 0.7325450581720951

# the wrong dfn — Ridge ← {(1○⍵÷8)+(1○⍵÷8)} — doubles ONE coordinate: at cell (0,7) it
# gives 2sin(-1/2) where the true field gives sin(-1/2)+sin(3/8); the fields differ
wrong ← (2×Sin xs÷8) +⌜ 8⥊0
! 1e¯12 > | (0‿7⊑wrong) - ¯0.958851077208406
! ridge ≢ wrong

# def basin = ridge < 0 : 36 cells by hand (pair sums of sin(k/8), k=¯4..3, below zero)
basin ← ridge<0
! 36 = +´⥊basin

# 64 64 & basin , Water = 100 : scatter onto the basin cells only
water ← 64⥊7
water ↩ (100¨)⌾((⥊basin)⊸/) water
! water ≡ (7×¬⥊basin)+100×⥊basin
! 36 = +´water=100

# 64 64 & ridge > 0.5 , spawn Peak : by hand only sin(2/8)+sin(3/8) and 2sin(3/8) clear 0.5,
# so the peaks are cells (6,7) (7,6) (7,7) — flat 55 62 63 — world coords (2,3) (3,2) (3,3)
peak ← ridge>0.5
! 3 = +´⥊peak
! ⟨55,62,63⟩ ≡ /⥊peak

# def slope = abs(Height - neighbor(clamp).Height) : one functional hop along the row axis,
# index clamped at the boundary so the last row is its own neighbor
height ← ⟨0,10,20,30,80,90,100,110⟩ ×⌜ 8⥊1
nbr   ← (7⌊1+↕8)⊏height
slope ← |height-nbr
! slope ≡ ⟨10,10,10,50,10,10,10,0⟩ ×⌜ 8⥊1    # clamp: boundary row slope 0

# 64 64 & slope > 30 , +Cliff : only the 30→80 jump, row 3 entire
cliff ← slope>30
! 8 = +´⥊cliff
! (24+↕8) ≡ /⥊cliff
