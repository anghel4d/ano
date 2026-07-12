# Spec §12 (pre-renumbering §11) — The honest fold contract: raw reduction requires an associative operator with identity; avg is a derived fold-and-finish, count likewise.
# ano:  avg/ Weight @ Cow
#       #/ Cow
# Semantics: fold/ col @ scope collapses under an associative reducer; avg is not a raw fold — it is the sum-fold divided by the count-fold, one finishing step — and having no identity it fails the empty scope, the §5 left-join-null rule extended to the empty fiber.
# Invariant: pairwise mean is not associative (witness), no fold direction of it recovers the true mean, yet fold+finish gives the exact mean; count is the fold of ones; the empty scope fails.

cow    ← 1‿1‿0‿1‿1‿0
weight ← 400‿500‿50‿600‿900‿70
w ← cow/weight               # 400 500 600 900

# fold+finish: sum-fold, count-fold, one finishing divide
! 2400 ≡ +´ w
! 4    ≡ +´ 1¨w
! 600  ≡ (+´w) ÷ +´1¨w
! 4    ≡ +´ cow              # #/ Cow — count is the sum-fold of the mask

# associativity failure witness: no pairwise operator computes avg by raw reduction
M ← {(𝕨+𝕩)÷2}                # the candidate pairwise mean
! ¬ ((400 M 500) M 900) ≡ 400 M (500 M 900)   # 675 vs 550: not associative
foldR ← M´ w                 # right association over the column
foldL ← M˜´ ⌽w               # left association over the column
! 512.5 ≡ foldR
! 712.5 ≡ foldL
! ¬ foldR ≡ foldL            # the raw reduction is ill-defined ...
! ¬ 600 ≡ foldR              # ... and neither association is the mean
! ¬ 600 ≡ foldL

# raw folds have identities on the empty scope; avg has none and fails
! 0 ≡ +´⟨⟩ ⋄ ! 1 ≡ ×´⟨⟩ ⋄ ! 1 ≡ ∧´⟨⟩ ⋄ ! 0 ≡ ∨´⟨⟩
Avg ← {!0<≠𝕩 ⋄ (+´𝕩)÷≠𝕩}
! 600 ≡ Avg w
! "row fails" ≡ Avg⎊("row fails"˙) ⟨⟩
