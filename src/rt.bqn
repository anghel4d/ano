# rt.bqn — the anoc runtime prelude, prepended to every emitted program.
# Thin by design: per-demo callables (fib, polar, dist, verbs) are registry rows
# carrying their own BQN, mirroring host registration. Only surface-independent
# helpers live here. Names carry the Ano prefix because BQN identifiers are
# case-insensitive: a bare Rank would collide with a column named rank.

# Dense value-only rank. Inputs: numeric list. Output: same-length ranks; ties share (Tier 2).
AnoRank ← {(∧⍷𝕩)⊐𝕩}

# Scatter selection-length values into a column under a mask.
# Inputs: 𝕨 = ⟨mask (full length), vals (selection length)⟩, 𝕩 = column. Output: new column.
AnoScat ← {m‿v 𝕊 c: (v˙)⌾(m⊸/) c}

# Mean with the fold-and-finish reading. Inputs: nonempty list. Output: scalar.
AnoAvg ← +´÷≠

# Clamp-boundary neighbor shift along the leading axis: the boundary row is its own neighbor.
# Inputs: list or table. Output: same shape, row i holds row min(i+1, n-1).
AnoNbrClamp ← {((1+↕≠𝕩)⌊(≠𝕩)-1)⊏𝕩}

# Fiber image: union of the fibers of the selected sources, as a mask over the world.
# Inputs: 𝕨 = ⟨mask over sources, fibers (lists of stable entity ids)⟩, 𝕩 = the id column.
# Output: boolean mask; membership is by stable id (ex12), multiplicity is gone.
AnoImage ← {m‿f 𝕊 ids: ids ∊ ∾⟨⟨⟩⟩∾m/f}

# Inverse fibers of a functional relationship: preimage per target id.
# Inputs: 𝕨 = rel column (target id per source, ¯1 dangling — ⊔ drops negatives), 𝕩 = n targets.
# Output: n fibers of ascending source ids (the s05 (pen∾4)⊔↕≠pen idiom).
AnoInvFib ← {r 𝕊 n: (r∾n) ⊔ ↕≠r}
