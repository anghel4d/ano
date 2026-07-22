
sb ← 1‿1‿0‿0 ⋄ homing ← 1‿0‿0‿0 ⋄ target ← 2‿¯1‿¯1‿¯1
posx ← 10‿10‿40‿0 ⋄ velx ← 6‿6‿0‿0

m ← sb∧homing
velx ↩ velx + m × (((0⌈target)⊏posx) - posx) ÷ 2
! 21‿6‿0‿0 ≡ velx

posx ↩ posx + sb×velx
! 31‿16‿40‿0 ≡ posx
