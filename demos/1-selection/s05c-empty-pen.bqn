# The all-quantifier and the empty fiber (decision record setHop; spec §5, §9). Split from s05d-set-hop.bqn.
# ano: Pen & &/ livestock'.Healthy , +Certified
# ano: Pen , Headcount = #/ livestock'
# Semantics: &/ over the fiber is the all-quantifier; |/ &/ #/ over an empty fiber give the registered identities 0, 1, 0 — a pen with no livestock is vacuously certified.

pen       ← 0‿0‿1‿1‿2               # animal → pen (functional); livestock is its inverse read
healthy   ← 1‿1‿1‿0‿1
livestock ← (pen∾4) ⊔ ↕≠pen          # preimage fibers; pen 3 holds no animals
certified ← 4⥊0

allH ← {∧´ 𝕩⊏healthy}¨ livestock
certified ∨↩ allH
! 1‿0‿1‿1 ≡ certified
# the empty fiber takes the fold's registered identity: &/ gives true, |/ gives false, #/ gives 0
"empty-fiber identities" ! 1‿0‿0 ≡ ⟨∧´⟨⟩, ∨´⟨⟩, +´⟨⟩⟩
! 2‿2‿1‿0 ≡ ≠¨livestock              # Pen , Headcount = #/ livestock'
