
gold ← 100‿200‿300‿400‿500
nord ← 1‿0‿1‿1‿0
W ← {g‿n: g+1000×n}

! 1100‿200‿1300‿1400‿500 ≡ W gold‿nord

taus ← ⟨4‿3‿2‿1‿0, 1‿2‿3‿4‿0, 2‿0‿1‿4‿3, 3‿0‿4‿1‿2, 0‿1‿2‿3‿4⟩
! ∧´ {(W ⟨𝕩⊏gold, 𝕩⊏nord⟩) ≡ 𝕩⊏W gold‿nord}¨ taus

t ← 1‿0‿2‿3‿4
! ¬ (W ⟨t⊏gold, nord⟩) ≡ t⊏W gold‿nord
