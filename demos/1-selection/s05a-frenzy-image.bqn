# The set-hop image and in-degree (decision record setHop; spec §5, §9). Split from s05d-set-hop.bqn.
# ano: Frenzy.targets' , +Frenzied ; Health -= 10
# ano: Target , Hits += #/ attackers'
# Semantics: sel.rel' in source position selects the image, the union of fibers — a mask, so a target reachable from two sources takes the effect once; in-degree is the gamma fold over the inverse fiber, never the scatter.
# Invariant: idempotent scatter on the image (in-degree never sums into a value effect); the inverse-fiber count recovers in-degree.

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

# in-degree: the gamma fold over the inverse fiber (attackers = inverse of targets)
srcIds    ← (≠¨frenzy/targets) / /frenzy   # source id per in-edge
attackers ← (tgtIds∾n) ⊔ srcIds            # inverse read: preimage fiber per target (ids⊔vals)
hits ← n⥊0
hits +↩ ≠¨attackers

! 0‿0‿0‿1‿2‿1‿0‿0 ≡ hits
"in-degree lives in the gamma fold, not the scatter" ! 2 ≡ 4⊑hits
! image ≡ hits>0
