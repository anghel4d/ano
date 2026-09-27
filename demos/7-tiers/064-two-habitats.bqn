
! (↕5) ≡ 0‿1‿2‿3‿4
! (↕2‿3) ≡ 2‿3⥊⟨0‿0, 0‿1, 0‿2, 1‿0, 1‿1, 1‿2⟩

gold ← 7‿12‿5‿30‿9 ⋄ nord ← 1‿0‿1‿1‿0
! 42 ≡ +´nord/gold
elev ← 2‿3⥊1‿2‿3‿4‿5‿6
! 21 ≡ +´⥊elev

dmg ← 4‿1‿3‿2‿5
! 1‿3‿6‿10‿15 ≡ +`(⍋dmg)⊏dmg
cost ← 2‿3⥊1
! (2‿3⥊1‿1‿1‿2‿2‿2) ≡ +`cost

threat ← 10‿50‿20‿40‿30
! 1‿3‿4 ≡ 3↑⍒threat
safety ← 2‿3⥊3‿1‿4‿1‿5‿9
! 5‿4 ≡ 2↑⍒⥊safety

a ← 1‿2‿3 ⋄ b ← 10‿20
! (3‿2⥊11‿21‿12‿22‿13‿23) ≡ a+⌜b
! (>⟨1‿0‿1‿0, 0‿1‿0‿1, 1‿0‿1‿0, 0‿1‿0‿1⟩) ≡ 0=2|+⌜˜↕4

count ← 2‿0‿1‿3
! 0‿0‿2‿3‿3‿3 ≡ count/↕4
density ← 1‿0‿2‿1
! 0‿2‿2‿3 ≡ density/↕4

! (4‿2⥊↕8) ≡ 4‿∘⥊↕8

x ← 5‿8‿13‿21‿34
! 0‿5‿8‿13‿21 ≡ »x
g ← 2‿3⥊1‿2‿3‿4‿5‿6
! (2‿3⥊0‿0‿0‿1‿2‿3) ≡ »g

dmgC ← 10‿6‿8 ⋄ spd ← 3‿5‿2 ⋄ rng ← 5‿3‿4
! 6‿10‿4 ≡ dmgC×spd÷rng
! (>⟨0‿1‿2‿3, 1‿2‿3‿4, 2‿3‿4‿5, 3‿4‿5‿6⟩) ≡ +⌜˜↕4

entityIndex ← ↕6
cellIndex ← ↕6
entityHabitat ← ⟨"entity", entityIndex⟩
cellHabitat ← ⟨"cell", cellIndex⟩
! entityIndex ≡ cellIndex
! entityHabitat ≢ cellHabitat
