# ano: avg/ and avg\ over Cow, #/ and #\ over the selection, then avg/ over None
# Semantics: mean state is (ordered sum,count), projected at each prefix and at finish. It is not repeated binary averaging. Count has empty result 0; mean has no empty result row.

cow    ← 1‿1‿0‿1‿1‿0
none   ← 6⥊0
weight ← 400‿500‿50‿600‿900‿70
x ← cow/weight

sumPrefixes ← +`x
countPrefixes ← +`1¨x
means ← sumPrefixes÷countPrefixes
LeftSum ← {+˜´⌽(0∾𝕩)}
Mean ← {(LeftSum 𝕩)÷≠𝕩}
NoMean ← {0=≠𝕩 ? ⟨⟩ ; ⟨Mean 𝕩⟩}

! 400‿450‿500‿600 ≡ means
! 600 ≡ Mean x
! (Mean x) ≡ ¯1⊑means
! 4 ≡ +´cow
! 1‿2‿2‿3‿4‿4 ≡ +`cow
! 2400 ≡ LeftSum x

M ← {(𝕨+𝕩)÷2}
! 712.5 ≡ M˜´⌽x
! ¬ 600 ≡ M˜´⌽x
! 0 = ≠ NoMean none/weight
