# Derived tag, the representation-independence witness (s57): 131's stored bits and 132's as-derived equality mask are one tag. The twins pin identical post-states in separate files; this is the cross-file equality made executable — edit either registry's tag and this breaks.
# ano: Nord & Gold > 50 , Gold += 500     (131-derived-tag-a.ano ≡ 132-derived-tag-b.ano)
# Semantics: option A stores the tag as written bits, option B recomputes δ_Nord ∘ race against the live column at each use; downstream algebra cannot distinguish them, so one program over both worlds must land on one post-state.
# Invariant: the two masks agree cell for cell, and the two post-states are identical — the gold both twins pin, the rel untouched.

race ← ⟨"Nord", "Khajiit", "Nord", "Nord", "Orc"⟩
mA ← 1‿0‿1‿1‿0                  # 131: the stored bits
mB ← "Nord"⊸≡¨ race             # 132: the derived mask, recomputed from the live column
! mA ≡ mB

gold   ← 400‿600‿200‿150‿221
master ← 2‿¯1‿¯1‿4‿¯1
goldA ← gold + 500 × mA ∧ gold > 50
goldB ← gold + 500 × mB ∧ gold > 50
! goldA ≡ goldB
! goldA ≡ 900‿600‿700‿650‿221   # the pin both twins carry
! master ≡ 2‿¯1‿¯1‿4‿¯1         # the rel never moves
