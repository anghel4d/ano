# Ex48 — Zero subject default: the omitted left side is ゼロが, supplied by the host.
# ano: 小麦を生やす。
# ano: あの @cursor が spawn Wheat
# ano: spawn Wheat
# Semantics: with no subject the host's registered default supplies one — the cursor's raycast point — and spawn appends one row there; the retired @spawn verb form is spelled as the bare effect with the default subject.
# Invariant: the world gains exactly one row, typed wheat, positioned at the cursor, and every prior row is untouched.

WheatAt ← {⟨⟨"wheat",𝕩⟩⟩}      # one spawned row: type and position
world ← ⟨⟨"tree",0‿0⟩,⟨"rock",5‿1⟩,⟨"tree",2‿7⟩⟩
before ← world
cursor ← 3‿4
world ∾↩ WheatAt cursor
"ex48: one row appended"!(4=≠world)
"ex48: prior rows untouched"!(before≡3↑world)
"ex48: wheat at the cursor"!(⟨"wheat",3‿4⟩≡3⊑world)
