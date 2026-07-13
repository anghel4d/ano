# rt.bqn — the anoc runtime prelude, prepended to every emitted program.
# Thin by design: per-demo callables (fib, polar, dist, verbs) are registry rows
# carrying their own BQN, mirroring host registration. Only surface-independent
# helpers live here. Names carry the Ano prefix because BQN identifiers are
# case-insensitive: a bare Rank would collide with a column named rank.

# Dense ranks for a numeric list; equal values share a rank.
AnoRank ← {(∧⍷𝕩)⊐𝕩}

# Scatter selection-length values into a column under a mask.
# Inputs: 𝕨 = ⟨mask (full length), vals (selection length)⟩, 𝕩 = column. Output: new column.
AnoScat ← {m‿v 𝕊 c: (v˙)⌾(m⊸/) c}

# Mean with the fold-and-finish reading. Inputs: nonempty list. Output: scalar.
AnoAvg ← +´÷≠

# Clamp-boundary neighbor shift along the leading axis: the boundary row is its own neighbor.
# Inputs: list or table. Output: same shape, row i holds row min(i+1, n-1).
AnoNbrClamp ← {((1+↕≠𝕩)⌊(≠𝕩)-1)⊏𝕩}

# Fiber image. 𝕨 is ⟨source mask, fibers of ids or keys⟩; 𝕩 is the current id/key column.
# Returns the world mask of referenced members.
AnoImage ← {m‿f 𝕊 ids: ids ∊ ∾⟨⟨⟩⟩∾m/f}

# Inverse fibers of a functional relationship. 𝕨 is the target index per source; 𝕩 is the
# target count. Returns ascending source-index fibers.
AnoInvFib ← {r 𝕊 n: (r∾n) ⊔ ↕≠r}
