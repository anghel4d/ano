# ex31 - Spatial top-k (demoed at 8x8; the docs' 64 64 scales without change)
# ano: top 8 (grade desc Safety @ 64 64) , spawn Sentry
#      64 64 & top 5 (grade desc Resource) , +MiningNode
# semantics: the lattice is 64 cell-entities addressed by flat index k (x = 8|k, y = floor k/8); grade a per-cell column descending, take the k best cells, spawn into them / set their mask.
# invariant: exactly the top-k cells by the graded column are hit; grade-desc ties resolve stably to the lower cell index; spawn adds one Sentry per chosen cell, +MiningNode is a mask set.

# world: 8x8 lattice as flat columns
k ← ↕64
x ← 8|k
y ← ⌊k÷8

# Safety peaks at the keep (2,3): 100 minus squared distance
safety ← 100 - ((x-2)×(x-2)) + (y-3)×(y-3)
# Resource grows toward the far corner
resource ← x×y

sentries   ← 64⥊0
miningNode ← 64⥊0

# top 8 (grade desc Safety) , spawn Sentry
top8 ← 8↑⍒safety
sentries +↩ (↕64) ∊ top8
# top 5 (grade desc Resource) , +MiningNode
top5 ← 5↑⍒resource
miningNode ∨↩ (↕64) ∊ top5

# by hand: d2=0 cell 26; d2=1 tie cells 18 25 27 34 in stable index order; d2=2 tie cells 17 19 33 35, stable grade keeps 17 19 33
! 26‿18‿25‿27‿34‿17‿19‿33 ≡ top8
# by hand: 49 at k63; 42 at k55,k62 (stable: 55 first); 36 at k54; 35 at k47,k61 (stable: 47)
! 63‿55‿62‿54‿47 ≡ top5

# expected post-state: one sentry on each of the 8 safest cells, none elsewhere
! ((↕64) ∊ 26‿18‿25‿27‿34‿17‿19‿33) ≡ sentries
! 8 ≡ +´sentries
! ((↕64) ∊ 63‿55‿62‿54‿47) ≡ miningNode
! 5 ≡ +´miningNode
