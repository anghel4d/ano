# The naru register: the flora act Life out (c3; §11, the rule barrier).
# Ano: def bloom  = Plot & !Planted & #/ (moore' & Planted) == 3 => +Planted
#      def wither = Plot & Planted & #/ (moore' & Planted) != 2 & #/ (moore' & Planted) != 3 => -Planted
#      -- or fused, one rule: def life = Plot => Planted = kin == 3 | Planted & kin == 2
# Semantics: §11 — every rule active in a tick gathers the tick's pre-state and the set scatters once, one shared barrier. Installed, the field plays one generation per tick, forever; the statement log is empty and the flora keep acting.
# Invariant: under the shared barrier the bloom/wither pair equals the fused rule, either commit order; giving each rule its own barrier (bloom commits, wither re-gathers) breaks the blinker in BOTH orders, and differently — order-dependence is the proof that the shared barrier is semantics, not scheduling. Life is B3/S23 as a production system: Datalog states bloom and never wither — monotone growth only; death is retraction, exactly what the production register adds.

S ← {(»𝕩)+(«𝕩)+(»˘𝕩)+(«˘𝕩)+(»»˘𝕩)+(»«˘𝕩)+(«»˘𝕩)+(««˘𝕩)}   # Moore counts, dead border
Life ← {n ← S 𝕩 ⋄ (3=n) ∨ 𝕩 ∧ 2=n}

p0 ← 5‿5⥊0‿0‿0‿0‿0‿ 0‿0‿0‿0‿0‿ 0‿1‿1‿1‿0‿ 0‿0‿0‿0‿0‿ 0‿0‿0‿0‿0
next ← Life p0

# --- the shared barrier: both rules gather p0, the scatter merges disjoint row sets ---
n ← S p0
birth ← (¬p0) ∧ 3=n                  # bloom's mask over the tick's pre-state
death ← p0 ∧ ¬ (2=n) ∨ 3=n           # wither's mask over the SAME pre-state
! next ≡ (p0 ∨ birth) ∧ ¬death       # commit bloom first
! next ≡ (p0 ∧ ¬death) ∨ birth       # commit wither first: same bits, the pair merges

# --- per-rule barriers: the second rule re-gathers the first's commit, the tick is no longer one step ---
Bloom  ← {𝕩 ∨ (¬𝕩) ∧ 3 = S 𝕩}
Wither ← {n ← S 𝕩 ⋄ 𝕩 ∧ (2=n) ∨ 3=n}
ring ← Wither Bloom p0               # newborns feed the death count: a 4-ring, not the bar
lone ← Bloom Wither p0               # the arms starve before they can breed: one cell, then extinction
! ring ≡ 5‿5⥊0‿0‿0‿0‿0‿ 0‿0‿1‿0‿0‿ 0‿1‿0‿1‿0‿ 0‿0‿1‿0‿0‿ 0‿0‿0‿0‿0
! lone ≡ 5‿5⥊0‿0‿0‿0‿0‿ 0‿0‿0‿0‿0‿ 0‿0‿1‿0‿0‿ 0‿0‿0‿0‿0‿ 0‿0‿0‿0‿0
! next ≢ ring
! next ≢ lone
! ring ≢ lone                        # order-dependent, so per-rule commit is not even a deterministic wrong

# Shared-barrier and broken per-rule results all remain 5×5 fields; the difference is state, not rank.
! 5‿5 ≡ ≢next
! 5‿5 ≡ ≢ring
! 5‿5 ≡ ≢lone
