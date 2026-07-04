# Homing (n3): the behavior card is data plus law — a marker component, a target relationship, a standing rule.
# Ano: def homing = SparkBolt & Homing => Velx += (target.Posx - Posx) / 2
#      SparkBolt , Posx += Velx
# Semantics: Noita bakes homing into the projectile's update loop; ano splits it — +Homing and target are columns the cast writes, the steer is an installed rule, and the rule body is one masked gather-subtract: target.Posx is the §5 hop, an indexed read. Guidance is a world law, not per-projectile code. One axis shown; the y line is the same op.
# Invariant: one tick halves the marked bolt's gap to the quarry then flies (velx 6 to 21, posx 10 to 31); the unmarked bolt flies straight to 16; the quarry never moves.

sb ← 1‿1‿0‿0 ⋄ homing ← 1‿0‿0‿0 ⋄ target ← 2‿¯1‿¯1‿¯1
posx ← 10‿10‿40‿0 ⋄ velx ← 6‿6‿0‿0

m ← sb∧homing
velx ↩ velx + m × (((0⌈target)⊏posx) - posx) ÷ 2   # the steer: gather the quarry's posx, halve the gap
! 21‿6‿0‿0 ≡ velx

posx ↩ posx + sb×velx                              # one frame of flight
! 31‿16‿40‿0 ≡ posx
