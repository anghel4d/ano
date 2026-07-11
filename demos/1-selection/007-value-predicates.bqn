# Value predicates (ex7).
# ano: Gold == 500 , Gold += 100
# ano: Gold > Health * 10 , +Greedy
# ano: Stamina < Weight , Speed *= 0.5
# Semantics: pointwise column comparisons produce masks; masks drive value effects (masked add, masked halve) and structural adds (mask or).
# Invariant: *= 0.5 down a mask is v×1-0.5×mask; each statement gathers the previous statement's committed state.

gold    ← 500‿700‿500‿100‿90
health  ← 55‿50‿100‿5‿9
greedy  ← 0‿0‿0‿0‿1
stamina ← 5‿30‿10‿50‿4
weight  ← 10‿20‿10‿60‿4
speed   ← 8‿6‿4‿2‿10

gold +↩ 100×gold=500
greedy ↩ greedy ∨ gold>health×10
speed ×↩ 1 - 0.5×stamina<weight

! 600‿700‿600‿100‿90 ≡ gold
! 1‿1‿0‿1‿1 ≡ greedy
! 4‿6‿4‿1‿10 ≡ speed
# statement 2 observed statement 1's scatter: row 0 (600 > 550) was not greedy at its pre-state 500
"statements are sequential barriers" ! 1 ≡ 0⊑greedy
