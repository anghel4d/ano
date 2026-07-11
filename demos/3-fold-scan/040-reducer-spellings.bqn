# s64 — one reducer, three spellings: threat/ (slash on the name), fold(threat) (the long form), threat\ (the scan comes free).
# ano:  threat/ Damage @ Enemies
#       fold(threat) Damage @ Enemies
#       threat\ Damage @ Enemies
# Semantics: the slash attaches to a registered reducer name exactly as it attaches to an operator — one grammar row, fold(f) the long form — and the backslash gives the scan of the same reducer: one running value per selected cell.
# Invariant: both fold spellings are the same fold (nothing to witness apart — the .ano pins both against one value); the scan's last element is the fold; the scan has the length of its selection.

ThreatReduce ← ⌈    # registered reducer: threat is peak damage — associative, no identity

enemies ← 0‿1‿1‿0‿1‿0‿1‿0
damage  ← 5‿12‿7‿0‿30‿4‿9‿2

! 30 ≡ ThreatReduce´ enemies/damage           # the fold, either spelling
! 12‿12‿30‿30 ≡ ThreatReduce` enemies/damage  # the scan: running peak

# the scan's last element is the fold; same-length
! (ThreatReduce´ enemies/damage) ≡ ¯1⊑ ThreatReduce` enemies/damage
! (+´enemies) ≡ ≠ ThreatReduce` enemies/damage

# empty scope: the scan is length-preserving, so it yields the empty column — no identity consulted;
# the fold of the same scope fails the row (029-named-reducer.bqn pins that half)
none ← 8⥊0
! ⟨⟩ ≡ ThreatReduce` none/damage
