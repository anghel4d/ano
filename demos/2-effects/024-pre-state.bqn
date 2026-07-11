# One barrier, one pre-state (spec §10): batched effects gather once, scatter once.
# ano: Trickster , Gold = Silver ; Silver = Gold
# Semantics: gather, effect, scatter — both batched effects read the statement's pre-state, so the pair is a swap; the merged delta (disjoint write columns, so it commutes) scatters at the statement's end.
# Invariant: the buffered result swaps the two columns on-mask; naive sequential application, where the second effect reads the first's write, differs.

trickster ← 0‿1‿1‿0
gold      ← 10‿20‿30‿40
silver    ← 1‿2‿3‿4

# buffered: both effects read pre-state (gold, silver as gathered)
goldB   ← (trickster/silver)⌾(trickster⊸/) gold
silverB ← (trickster/gold)⌾(trickster⊸/) silver

! goldB   ≡ 10‿2‿3‿40
! silverB ≡ 1‿20‿30‿4

# naive sequential: the second effect reads the first's write
goldN   ← (trickster/silver)⌾(trickster⊸/) gold
silverN ← (trickster/goldN)⌾(trickster⊸/) silver

! silverN ≡ 1‿2‿3‿4          # the swap is lost: silver reads its own old values back
! ¬silverB ≡ silverN         # buffered and sequential genuinely differ

# off-mask cells untouched
! ((¬trickster)/goldB)   ≡ (¬trickster)/gold
! ((¬trickster)/silverB) ≡ (¬trickster)/silver
