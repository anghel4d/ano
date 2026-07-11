# Outer product as value (ex21): materialize the pairwise distance table.
# ano: cross dist Tower Creep
# Semantics: the outer product as a value — an nT x nC matrix, dist applied to every pair, no filter and no effect.
# Invariant: the matrix matches the hand-computed table, and filtering its flattening reproduces ex20's theta-join pairs — the comprehension is exactly sigma_p over this value.

# Same world as ex20.
tp ← ⟨0‿0, 200‿0, 50‿50⟩          # Tower positions
cp ← ⟨10‿10, 90‿10, 50‿80, 0‿49⟩  # Creep positions
d2 ← tp {+´×˜𝕨-𝕩}⌜ cp             # cross dist Tower Creep, squared to stay integral

# Hand-computed, one row per tower.
! d2 ≡ 3‿4 ⥊ 200‿8200‿8900‿2401‿36200‿12200‿28900‿42401‿3200‿3200‿900‿2501
! 3‿4 ≡ ≢d2                        # shape is |Tower| x |Creep|

# ex20 is a filter over this value: same pairs, no recomputation.
! ⟨0‿0, 0‿3, 2‿2⟩ ≡ (⥊d2<2500) / ⥊ (↕3) ⋈⌜ ↕4
