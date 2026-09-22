
adj ← >⟨0‿1‿1‿0‿0
        0‿0‿1‿0‿0
        1‿0‿0‿1‿1
        0‿0‿0‿0‿0
        0‿1‿0‿0‿1⟩
hostile ← 0‿1‿0‿1‿1
cursor  ← 0‿0‿0‿1‿0
btree   ← ⟨"seek", "flee", "guard", "patrol", "idle"⟩

outdeg ← +´˘adj
! 2‿1‿3‿0‿2 ≡ outdeg

env ← /hostile
! 1‿3‿4 ≡ env
! adj ≡ >⟨0‿1‿1‿0‿0, 0‿0‿1‿0‿0, 1‿0‿0‿1‿1, 0‿0‿0‿0‿0, 0‿1‿0‿0‿1⟩

! ⟨3⟩ ≡ /cursor
! "patrol" ≡ ⊑(/cursor)⊏btree
! btree ≡ ⟨"seek", "flee", "guard", "patrol", "idle"⟩
