# ano: 照準 beside ^照準
# Semantics: non-ASCII alias keys are exact-byte after the registry's ASCII-only case fold. A live native stem can shadow its bare namesake without changing it.

bare ← 0‿1‿0‿0‿1‿0
live ← 1‿0‿0‿0‿0‿1
hero ← 0‿1‿0‿1‿0‿1
hit  ← 6⥊0
faction ← 0‿1‿0‿1‿0‿1

hit +↩ bare
selected ← live ∧ hero
hit ↩ (9¨)⌾(selected⊸/) hit
faction ↩ (2¨)⌾(selected⊸/) faction

! 0‿1‿0‿0‿1‿9 ≡ hit
! 0‿1‿0‿1‿0‿2 ≡ faction
! 0‿1‿0‿0‿1‿0 ≡ bare
