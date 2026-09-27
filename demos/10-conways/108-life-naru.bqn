
S ← {(»𝕩)+(«𝕩)+(»˘𝕩)+(«˘𝕩)+(»»˘𝕩)+(»«˘𝕩)+(«»˘𝕩)+(««˘𝕩)}
Life ← {n ← S 𝕩 ⋄ (3=n) ∨ 𝕩 ∧ 2=n}

p0 ← 5‿5⥊0‿0‿0‿0‿0‿ 0‿0‿0‿0‿0‿ 0‿1‿1‿1‿0‿ 0‿0‿0‿0‿0‿ 0‿0‿0‿0‿0
next ← Life p0

n ← S p0
birth ← (¬p0) ∧ 3=n
death ← p0 ∧ ¬ (2=n) ∨ 3=n
! next ≡ (p0 ∨ birth) ∧ ¬death
! next ≡ (p0 ∧ ¬death) ∨ birth

Bloom  ← {𝕩 ∨ (¬𝕩) ∧ 3 = S 𝕩}
Wither ← {n ← S 𝕩 ⋄ 𝕩 ∧ (2=n) ∨ 3=n}
ring ← Wither Bloom p0
lone ← Bloom Wither p0
! ring ≡ 5‿5⥊0‿0‿0‿0‿0‿ 0‿0‿1‿0‿0‿ 0‿1‿0‿1‿0‿ 0‿0‿1‿0‿0‿ 0‿0‿0‿0‿0
! lone ≡ 5‿5⥊0‿0‿0‿0‿0‿ 0‿0‿0‿0‿0‿ 0‿0‿1‿0‿0‿ 0‿0‿0‿0‿0‿ 0‿0‿0‿0‿0
! next ≢ ring
! next ≢ lone
! ring ≢ lone

! 5‿5 ≡ ≢next
! 5‿5 ≡ ≢ring
! 5‿5 ≡ ≢lone
