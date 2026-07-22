
k ← ↕64
x ← 8|k
y ← ⌊k÷8

safety ← 100 - ((x-2)×(x-2)) + (y-3)×(y-3)
resource ← x×y

sentries   ← 64⥊0
miningNode ← 64⥊0

top8 ← 8↑⍒safety
sentries +↩ (↕64) ∊ top8
top5 ← 5↑⍒resource
miningNode ∨↩ (↕64) ∊ top5

! 26‿18‿25‿27‿34‿17‿19‿33 ≡ top8
! 63‿55‿62‿54‿47 ≡ top5

! ((↕64) ∊ 26‿18‿25‿27‿34‿17‿19‿33) ≡ sentries
! 8 ≡ +´sentries
! ((↕64) ∊ 63‿55‿62‿54‿47) ≡ miningNode
! 5 ≡ +´miningNode
