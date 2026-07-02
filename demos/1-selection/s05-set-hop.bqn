# Set hop: image selection, quantifiers, duplicate rule (decision record setHop; spec §5, §9).
# ano: Frenzy.targets' , +Frenzied ; Health -= 10
# ano: Plot & |/ neighbors'.Planted , +Sprouting
# ano: Plot & !Planted & #/ (neighbors' & Planted) >= 2 , +Planted
# ano: Pen & &/ livestock'.Healthy , +Certified
# ano: Target , Hits += #/ attackers'
# Semantics: rel' is the per-source fiber of a set-valued relationship; sel.rel' in source position selects the image, the union of fibers — a mask, so a target reachable from two sources takes the effect once; quantifiers are the boolean folds over the fiber; per-in-edge accumulation is the gamma fold over the inverse fiber.
# Invariant: idempotent scatter on the image (in-degree never sums into a value effect); |/ and &/ over an empty fiber give the registered identities 0 and 1; the inverse-fiber count recovers in-degree.

# --- image and duplicate rule: Frenzy.targets' , +Frenzied ; Health -= 10
n ← 8
frenzy   ← 1‿1‿0‿0‿0‿0‿0‿0
targets  ← ⟨3‿4, 4‿5, ⟨⟩, ⟨⟩, ⟨⟩, ⟨⟩, ⟨⟩, ⟨⟩⟩   # set-valued: source → target set; 4 is reachable twice
frenzied ← n⥊0
health   ← n⥊100

tgtIds ← ∾ frenzy / targets          # every in-edge of the selection's fibers
image  ← 0 < n ↑ /⁼ ∧tgtIds          # union of fibers, a mask: multiplicity is gone
frenzied ∨↩ image
health -↩ 10 × image

! 0‿0‿0‿1‿1‿1‿0‿0 ≡ frenzied
! 100‿100‿100‿90‿90‿90‿100‿100 ≡ health
# entity 4, targeted by both frenzied sources, was hit once — a selection is a mask, masks have no multiplicity
"idempotent scatter" ! 90 ≡ 4⊑health

# --- gamma at the target: Target , Hits += #/ attackers'  (attackers = inverse fiber of targets)
srcIds    ← (≠¨frenzy/targets) / /frenzy   # source id per in-edge
attackers ← (tgtIds∾n) ⊔ srcIds            # inverse read: preimage fiber per target (ids⊔vals)
hits ← n⥊0
hits +↩ ≠¨attackers

! 0‿0‿0‿1‿2‿1‿0‿0 ≡ hits
"in-degree lives in the gamma fold, not the scatter" ! 2 ≡ 4⊑hits
! image ≡ hits>0

# --- any and count quantifiers over the neighbor fiber
plot      ← 1‿1‿1‿1‿1‿1
planted   ← 1‿0‿1‿0‿1‿0
neighbors ← ⟨⟨1⟩, 0‿2, 1‿3, 2‿4, 3‿5, ⟨4⟩⟩
sprouting ← 6⥊0

# Plot & |/ neighbors'.Planted , +Sprouting
any ← {∨´ 𝕩⊏planted}¨ neighbors
sprouting ∨↩ plot ∧ any
! 0‿1‿0‿1‿0‿1 ≡ sprouting

# Plot & !Planted & #/ (neighbors' & Planted) >= 2 , +Planted
cnt ← {+´ 𝕩⊏planted}¨ neighbors      # count of the filtered fiber
planted ∨↩ plot ∧ (¬planted) ∧ cnt≥2
! 1‿1‿1‿1‿1‿0 ≡ planted

# --- all quantifier and the empty fiber: Pen & &/ livestock'.Healthy , +Certified
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
