# ano:  +/ Readings@row, max/ Readings@row over the fibers ⟨3,5⟩ ⟨⟩ ⟨2,4,9⟩ ⟨⟩
# Semantics: a per-row fold over a fiber column applies its helper once per row. A helper seeded with a registered identity is total, so every row answers; an unseeded one has no answer on an empty fiber and the row must drop through the validity channel instead of aborting the program.
# Invariant: the seeded fold answers all four rows; the unseeded one aborts if applied bare, and answers exactly the two nonempty rows once guarded.

readings ← ⟨3‿5, ⟨⟩, 2‿4‿9, ⟨⟩⟩

LeftSum ← {+˜´⌽(0∾𝕩)}
LeftMax ← {⌈˜´⌽𝕩}
LeftMin ← {⌊˜´⌽𝕩}

! 8‿0‿15‿0 ≡ LeftSum¨ readings

# bare, the unseeded fold has nothing to return on an empty fiber and BQN aborts
! "caught" ≡ ({LeftMax¨𝕩}⎊{"caught"}) readings

# the validity channel: compute under a total placeholder, then keep only the valid rows
Guard ← {(0<≠¨𝕩) / {0=≠𝕩 ? 0 ; LeftMax 𝕩}¨𝕩}
! 5‿9 ≡ Guard readings
! 3‿2 ≡ (0<≠¨readings) / {0=≠𝕩 ? 0 ; LeftMin 𝕩}¨readings

# no infinity is ever manufactured to stand in for the missing rows
! 0 = ≠ (0<≠¨⟨⟨⟩⟩) / {0=≠𝕩 ? 0 ; LeftMax 𝕩}¨⟨⟨⟩⟩
