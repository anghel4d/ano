# Derived columns (ex25): def names a column expression, reused as effect source, predicate, fold, and ordering key.
# ano: def threat = Damage * Speed / Range
#      def dps    = Damage / Cooldown
#      Enemy , Priority = threat
#      Enemy & threat > 100 , +Dangerous
#      +/ threat @ Enemy
#      order by threat desc
# Semantics: a def is a number-over-entities evaluated against pre-state wherever a raw column may appear; the masked write gathers the derived cells for the selected rows only.
# Invariant: the masked write lands on-mask with off-mask cells untouched; predicate, scoped fold, and descending grade over the derived column match hand computation.

damage   ← 10‿20‿30‿40‿50‿60
speed    ← 4‿6‿8‿10‿8‿3
range    ← 2‿4‿2‿2‿10‿9
cooldown ← 5‿4‿3‿8‿10‿6

enemy     ← 0‿1‿1‿1‿1‿0
priority  ← 9‿9‿9‿9‿9‿9
dangerous ← 0‿0‿0‿0‿0‿1

threat ← damage×speed÷range
dps    ← damage÷cooldown

! threat ≡ 20‿30‿120‿200‿40‿20
! dps    ≡ 2‿5‿10‿5‿5‿10

# Enemy , Priority = threat
priority ↩ (enemy/threat)⌾(enemy⊸/) priority
! priority ≡ 9‿30‿120‿200‿40‿9

# Enemy & threat > 100 , +Dangerous
dangerous ∨↩ enemy∧threat>100
! dangerous ≡ 0‿0‿1‿1‿0‿1

# +/ threat @ Enemy   (scoped-global fold: one scalar)
! 390 ≡ +´ enemy/threat

# order by threat desc   (stable descending grade)
! (⍒threat) ≡ 3‿2‿4‿1‿0‿5
! ((⍒threat)⊏threat) ≡ 200‿120‿40‿30‿20‿20
