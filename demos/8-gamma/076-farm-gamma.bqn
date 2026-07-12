# Farm interlude, the three gamma lines (ex36; decision-record farmRewrites and stencilRewrite).
# ano: Pen , Headcount = #/ (livestock' & Cattle)
# ano: Plot & !Planted & #/ (neighbors' & Planted) >= 2 , +Planted
# ano: Plot , Moisture = avg/ neighbors'.Moisture   -- per-plot mean over the neighbor fiber
# Semantics: the tick marks the hop that fans out; a fold prefix collapses each fiber to one value per selected source, written back under the ordinary Tier-2 alignment rule.
# Invariant: every written column matches its hand-computed per-pen / per-plot value, edge and corner cells included; the shift-stencil form and the grouped-fiber (gamma) form agree cell for cell.

# --- Pen , Headcount = #/ (livestock' & Cattle) ---
# pen: Animal -> Pen is functional; livestock' at pen j is the preimage. Pen 3 holds no animals: #/ has identity 0.
pen    ← 0‿0‿1‿1‿1‿2‿0        # target pen per animal
cattle ← 1‿0‿1‿1‿1‿0‿1        # Cattle mask per animal
pens   ← ↕4
headcount ← +´¨ (pen∾≠pens) ⊔ cattle   # group the mask by pen, count each filtered fiber
! 2‿3‿0‿0 ≡ headcount

# --- Plot & !Planted & #/ (neighbors' & Planted) >= 2 , +Planted ---
# neighbors: the registered 4-stencil, fiber shrinks at the boundary. One barrier step: a cross seed grows to its block, edge counts of 1 stay unplanted.
S ← »+«+»˘+«˘                  # planted-neighbor count per cell; out-of-grid contributes 0
p ← 5‿5⥊0‿0‿0‿0‿0‿ 0‿0‿1‿0‿0‿ 0‿1‿1‿1‿0‿ 0‿0‿1‿0‿0‿ 0‿0‿0‿0‿0
p1 ← p ∨ (¬p) ∧ 2 ≤ S p
! (5‿5⥊0‿0‿0‿0‿0‿ 0‿1‿1‿1‿0‿ 0‿1‿1‿1‿0‿ 0‿1‿1‿1‿0‿ 0‿0‿0‿0‿0) ≡ p1

# --- Plot , Moisture = avg/ neighbors'.Moisture ---
# Per-cell mean over the 4-neighbor fiber; the fiber shrinks to 3 at edges and 2 at corners, so no fiber is empty and every row survives.
m ← 3‿3⥊1+↕9
cnt ← S 1¨m                    # fiber size per cell
m1 ← (S m) ÷ cnt
! (3‿3⥊2‿3‿2‿3‿4‿3‿2‿3‿2) ≡ cnt
! ((3‿3⥊6‿9‿8‿13‿20‿17‿12‿21‿14) ÷ 3‿3⥊2‿3‿2‿3‿4‿3‿2‿3‿2) ≡ m1

# Cross-check: the stencil fiber is gamma. Flatten the registered neighbor relation to (source cell, neighbor value) edges, group by source, fold each fiber.
h‿w ← ≢m
d ← ⟨¯1‿0, 1‿0, 0‿¯1, 0‿1⟩
Ok ← {∧´(0≤𝕩)∧𝕩<h‿w}
nbrs ← {𝕊 c: Ok¨⊸/ (c⊸+)¨d}¨ ⥊↕h‿w      # neighbor coords per cell, boundary shrunk
ids  ← (≠¨nbrs) / ↕h×w                   # source cell per edge
vals ← ({+´𝕩×w‿1}¨ ∾nbrs) ⊏ ⥊m           # neighbor Moisture per edge
! (⥊m1) ≡ (+´÷≠)¨ (ids∾h×w) ⊔ vals

# The planted section under the same reading: the count fold over the 5×5 seed's fibers.
h2‿w2 ← ≢p
Ok2 ← {∧´(0≤𝕩)∧𝕩<h2‿w2}
nbrs2 ← {𝕊 c: Ok2¨⊸/ (c⊸+)¨d}¨ ⥊↕h2‿w2
ids2  ← (≠¨nbrs2) / ↕h2×w2
vals2 ← ({+´𝕩×w2‿1}¨ ∾nbrs2) ⊏ ⥊p
! (⥊S p) ≡ +´¨ (ids2∾h2×w2) ⊔ vals2
