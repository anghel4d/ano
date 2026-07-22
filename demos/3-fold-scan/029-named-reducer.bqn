
ThreatReduce ← ⌈

enemies ← 0‿1‿1‿0‿1‿0‿1‿0
damage  ← 5‿12‿7‿0‿30‿4‿9‿2

! 30 ≡ ThreatReduce´ enemies/damage

! (ThreatReduce´ enemies/damage) ≡ ThreatReduce˜´ ⌽enemies/damage

none ← 8⥊0
! ¯∞ ≡ ⌈´ none/damage
Threat ← {!0<≠𝕩 ⋄ ThreatReduce´𝕩}
! 30 ≡ Threat enemies/damage
! "row fails" ≡ Threat⎊("row fails"˙) none/damage
