
WheatAt ← {⟨⟨"wheat",𝕩⟩⟩}
world ← ⟨⟨"tree",0‿0⟩,⟨"rock",5‿1⟩,⟨"tree",2‿7⟩⟩
before ← world
cursor ← 3‿4
world ∾↩ WheatAt cursor
"ex48: one row appended"!(4=≠world)
"ex48: prior rows untouched"!(before≡3↑world)
"ex48: wheat at the cursor"!(⟨"wheat",3‿4⟩≡3⊑world)
