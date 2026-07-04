# Conway's Game of Life, one synchronous generation (c1; B3/S23 over the Moore stencil).
# Ano: def kin = #/ (moore' & Planted)
#      Plot , Planted = kin == 3 | Planted & kin == 2   -- fused: one value write, one barrier
#      -- or the snapshot triple, three barriers, order-free:
#      Plot , Kin = #/ (moore' & Planted)
#      Plot & !Planted & Kin == 3 , +Planted
#      Plot & Planted & Kin != 2 & Kin != 3 , -Planted
# Semantics: kin is the γ count over the registered 8-stencil fiber (out-of-grid contributes 0: the border is dead); every read in a statement observes pre-state, so the fused step is synchronous by construction — the barrier is Life's double buffer, and the in-place bug cannot be written in one statement.
# Invariant: fused ≡ triple (either order of the ± pair) ≡ the hand-stepped blinker; the oscillator returns to its seed at period 2; the stencil count equals the γ fold over the registered moore fibers; birth and death touch provably disjoint cells over the frozen count.

S ← {(»𝕩)+(«𝕩)+(»˘𝕩)+(«˘𝕩)+(»»˘𝕩)+(»«˘𝕩)+(«»˘𝕩)+(««˘𝕩)}   # Moore counts, dead border
Life ← {n ← S 𝕩 ⋄ (3=n) ∨ 𝕩 ∧ 2=n}

p0 ← 5‿5⥊0‿0‿0‿0‿0‿ 0‿0‿0‿0‿0‿ 0‿1‿1‿1‿0‿ 0‿0‿0‿0‿0‿ 0‿0‿0‿0‿0
p1 ← Life p0
! (5‿5⥊0‿0‿0‿0‿0‿ 0‿0‿1‿0‿0‿ 0‿0‿1‿0‿0‿ 0‿0‿1‿0‿0‿ 0‿0‿0‿0‿0) ≡ p1
! p0 ≡ Life p1                       # period 2: the blinker returns to its seed

# --- the snapshot triple: Kin frozen at barrier 1, then birth and death against the frozen column ---
kin ← S p0
! (5‿5⥊0‿0‿0‿0‿0‿ 1‿2‿3‿2‿1‿ 1‿1‿2‿1‿1‿ 1‿2‿3‿2‿1‿ 0‿0‿0‿0‿0) ≡ kin
birth ← (¬p0) ∧ 3=kin
death ← p0 ∧ ¬ (2=kin) ∨ 3=kin
! ¬ ∨´ ⥊ birth∧death                 # complementary guards: the two writes touch disjoint cells
! p1 ≡ (p0 ∨ birth) ∧ ¬death         # birth then death
! p1 ≡ (p0 ∧ ¬death) ∨ birth         # death then birth: the pair commutes over the frozen count

# --- γ cross-check: the stencil count is the grouped fold over the registered moore fibers ---
h‿w ← ≢p0
d ← ⟨¯1‿¯1, ¯1‿0, ¯1‿1, 0‿¯1, 0‿1, 1‿¯1, 1‿0, 1‿1⟩
Ok ← {∧´(0≤𝕩)∧𝕩<h‿w}
nbrs ← {𝕊 c: Ok¨⊸/ (c⊸+)¨d}¨ ⥊↕h‿w   # neighbor coords per cell, boundary shrunk
ids  ← (≠¨nbrs) / ↕h×w               # source cell per edge
vals ← ({+´𝕩×w‿1}¨ ∾nbrs) ⊏ ⥊p0      # neighbor Planted per edge
! (⥊kin) ≡ +´¨ (ids∾h×w) ⊔ vals
