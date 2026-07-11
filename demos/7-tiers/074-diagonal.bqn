# 074-diagonal - Tier 2 masked write: equivariant under the diagonal action, not under one column alone
# ano: Nord & TwoHanded > 60 , Gold += 1000    -- witnessed here as gold + 1000×nord, the spec's Tier-2 masked add
# Semantics: the masked add w(gold, nord) = gold + 1000×nord is a map on the whole record row, so the Sym(I) law reads diagonally: relabel every column by the same τ and the write commutes; relabeling gold alone tears value from mask and the naive single-column law fails.
# Invariant: w(gold∘τ, nord∘τ) ≡ w(gold, nord)∘τ for several τ (assert), and w(gold∘τ, nord) ≢ w(gold, nord)∘τ for a τ moving a nord boundary (assert failure of the single-column law).

gold ← 100‿200‿300‿400‿500
nord ← 1‿0‿1‿1‿0
W ← {g‿n: g+1000×n}

# hand-computed anchor
! 1100‿200‿1300‿1400‿500 ≡ W gold‿nord

# diagonal action: permute BOTH columns by the same τ — the write commutes
taus ← ⟨4‿3‿2‿1‿0, 1‿2‿3‿4‿0, 2‿0‿1‿4‿3, 3‿0‿4‿1‿2, 0‿1‿2‿3‿4⟩
! ∧´ {(W ⟨𝕩⊏gold, 𝕩⊏nord⟩) ≡ 𝕩⊏W gold‿nord}¨ taus

# single-column action: permute gold alone across a nord boundary — the naive law fails
t ← 1‿0‿2‿3‿4                  # swaps entity 0 (nord) with entity 1 (not)
! ¬ (W ⟨t⊏gold, nord⟩) ≡ t⊏W gold‿nord   # 1200‿100‿… vs 200‿1100‿…
