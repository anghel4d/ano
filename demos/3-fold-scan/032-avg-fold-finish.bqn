
cow    ← 1‿1‿0‿1‿1‿0
weight ← 400‿500‿50‿600‿900‿70
w ← cow/weight

! 2400 ≡ +´ w
! 4    ≡ +´ 1¨w
! 600  ≡ (+´w) ÷ +´1¨w
! 4    ≡ +´ cow

M ← {(𝕨+𝕩)÷2}
! ¬ ((400 M 500) M 900) ≡ 400 M (500 M 900)
foldR ← M´ w
foldL ← M˜´ ⌽w
! 512.5 ≡ foldR
! 712.5 ≡ foldL
! ¬ foldR ≡ foldL
! ¬ 600 ≡ foldR
! ¬ 600 ≡ foldL

! 0 ≡ +´⟨⟩ ⋄ ! 1 ≡ ×´⟨⟩ ⋄ ! 1 ≡ ∧´⟨⟩ ⋄ ! 0 ≡ ∨´⟨⟩
Avg ← {!0<≠𝕩 ⋄ (+´𝕩)÷≠𝕩}
! 600 ≡ Avg w
! "row fails" ≡ Avg⎊("row fails"˙) ⟨⟩
