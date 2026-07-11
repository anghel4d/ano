# Quantifiers over the neighbor fiber (decision record setHop; spec §5, §9). Split from s05d-set-hop.bqn.
# ano: Plot & |/ neighbors'.Planted , +Sprouting
# ano: Plot & !Planted & #/ (neighbors' & Planted) >= 2 , +Planted
# Semantics: |/ over the fiber is the any-quantifier; #/ over the filtered fiber is the count. Both folds read the statement's pre-state: crops spread one ring per tick.

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
