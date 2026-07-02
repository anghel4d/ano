# Ex15 — Named reducer form: apply a registered reducer to a column over a selected source.
# ano:  reduce(threat) Damage @ Enemies
# Semantics: reduce(name) resolves a registered associative reducer per §11 and folds scoped-global, one scalar over the Enemies scope; a reducer with no registered identity fails the empty scope, the §5 left-join-null rule extended to the empty fiber.
# Invariant: the fold equals the hand-computed threat, fold direction cannot matter under the associative reducer, and the empty scope fails rather than inventing a value.

ThreatReduce ← ⌈    # registered reducer: threat is peak damage — associative, no identity

enemies ← 0‿1‿1‿0‿1‿0‿1‿0
damage  ← 5‿12‿7‿0‿30‿4‿9‿2

! 30 ≡ ThreatReduce´ enemies/damage     # max of 12 7 30 9

# associative: right fold equals left fold
! (ThreatReduce´ enemies/damage) ≡ ThreatReduce˜´ ⌽enemies/damage

# no identity: the empty scope fails the row instead of inventing a value.
# BQN's own ⌈´⟨⟩ is the float identity ¯∞; ano component values are finite, so the registered reducer has no identity and guards the empty scope.
none ← 8⥊0
! ¯∞ ≡ ⌈´ none/damage
Threat ← {!0<≠𝕩 ⋄ ThreatReduce´𝕩}       # reduce(threat) as registered: fold plus the no-identity guard
! 30 ≡ Threat enemies/damage
! "row fails" ≡ Threat⎊("row fails"˙) none/damage
