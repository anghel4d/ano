# A filter cannot invent a key (§16 "Generation makes keys", Part IV, foundations §2.1 generable vs nominal keys). (was s16-keygen)
# ano: source & predicate , effect        -- selection: output keys are a subset of input keys, always
# ano: Spawner , spawn Minion * Count     -- generation: the only key-creating primitive
# Semantics: selection is a subobject inclusion, a mask over the live keys; replicate/spawn and the shape generator ↕shape are the only forms that produce keys not already present.
# Invariant: exhaustively over all 2^5 masks of a 5-key world, filtered keys are an order-preserving subset of the input; generation mints keys disjoint from the input and unreachable by any mask; the space index is regenerable from the shape alone.

keys ← 3‿7‿9‿12‿20                 # nominal record keys, arbitrary and non-contiguous

# Every predicate outcome is one of the 2^5 masks. Exhaust them all.
masks ← ⥊ ↕2‿2‿2‿2‿2
! 32 = ≠masks

# Each filtered output is a subset of the input and an order-preserving subsequence of it.
Chk ← {f ← 𝕩/keys ⋄ (∧´f∊keys) ∧ f ≡ (keys∊f)/keys}
! ∧´ Chk¨ masks

# Filtering never grows the key set.
! ∧´ {(≠𝕩/keys) ≤ ≠keys}¨ masks

# The filter is lossy and the loss is unrecoverable: two different pre-states, one post-state.
! (1‿0‿1/3‿7‿9) ≡ (1‿1‿0/3‿9‿7)

# Generation is the key-creating primitive: replicate-spawn mints fresh keys...
count ← 2‿0‿3‿1‿0
new ← (1+⌈´keys) + ↕+´count
! 6 = ≠new
! ∧´ ¬ new ∊ keys                  # disjoint: these keys did not exist before the spawn

# ...and no mask over the old world reaches them.
! ∧´ {¬∨´ (𝕩/keys) ∊ new}¨ masks

# Space keys are the degenerate case: generable, not nominal. ↕shape remints the whole index from the shape alone.
! (⥊↕3‿4) ≡ {(⌊𝕩÷4)⋈4|𝕩}¨ ↕12
