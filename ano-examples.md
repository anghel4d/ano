考案: Anghel (Anghel4d)

# ano example verification

Working review artifact. Each section gives the operation, a Japanese surface, the ano surface, and a BQN/q reading. The BQN and q are semantic sketches, not final parser commitments.

---

## 1 - Canonical masked update

Select every Nord whose Two-Handed value exceeds 60 and add 1000 Gold.

```haskell
両手が六十を超える北に、金を千与える。
```

Ano:
```haskell
Nord & TwoHanded > 60 , Gold += 1000
```

Verification:
```bqn
Gold +↩ 1000 × Nord ∧ TwoHanded>60
```
```q
update Gold:Gold+1000 from w where Nord, TwoHanded>60
```

## 2 - Core sentence form

Evaluate a source, filter it by a predicate, then apply an effect to the filtered ordered view.

```haskell
源で条件を満たすものに、効果を行う。
```

Ano:
```haskell
source & predicate , effect
```

Verification:
```bqn
effect⌾(predicate⊸/) source
```
```q
update effect from source where predicate
```

## 3 - Component masks

Use component presence and boolean operations as masks over the entity view.

```haskell
北に、金を百与える。
竜に、体力を零にする。
死んでいない盗賊で派閥が盗賊である者に、派閥を敵対にする。
```

Ano:
```haskell
Nord , Gold += 100
Dragon , Health = 0
Bandit & !Dead & Faction == Bandit , Faction = Hostile
```

Verification:
```bqn
Gold +↩ 100×Nord
Health ↩ 0⌾(Dragon⊸/) Health
Faction ↩ Hostile⌾((Bandit∧¬Dead∧Faction=Bandit)⊸/) Faction
```
```q
update Gold:Gold+100 from w where Nord
update Health:0 from w where Dragon
update Faction:`Hostile from w where Bandit, not Dead, Faction=`Bandit
```

## 4 - Scoped selection

Restrict a component mask to a named region and update the selected column.

```haskell
ホワイトランで商人に、金を五千与える。
```

Ano:
```haskell
Merchant @ Whiterun , Gold += 5000
```

Verification:
```bqn
Gold +↩ 5000 × Merchant ∧ Whiterun
```
```q
update Gold:Gold+5000 from w where Merchant, Region=`Whiterun
```

## 5 - Target aliases

Resolve dynamic aliases as singleton or small masks and apply ordinary effects.

```haskell
この標的に、体力を零にする。
観測者に、派閥を友好にする。
プレイヤーに、金を九千九百九十九与える。
選択中のものに、威力を二倍にする。
```

Ano:
```haskell
@cursor , Health = 0
@observer , Faction = Friendly
Player , Gold += 9999
@selected , Damage *= 2
```

Verification:
```bqn
Health ↩ 0⌾(cursor⊸/) Health
Faction ↩ Friendly⌾(observer⊸/) Faction
Gold +↩ 9999×Player
Damage ×↩ 1+selected
```
```q
update Health:0 from w where cursor
update Faction:`Friendly from w where observer
update Gold:Gold+9999 from w where Player
update Damage:Damage*2 from w where selected
```

## 6 - Presence patterns

Distinguish component present with a constrained value, present with any value, and absent.

```haskell
両手が六十を超える北に、金を千与える。
両手を持つ北に、訓練済みを与える。
両手を持たない北に、未訓練を与える。
```

Ano:
```haskell
(Nord, TwoHanded > 60) , Gold += 1000
(Nord, TwoHanded _) , +Trained
(Nord, !TwoHanded) , +Untrained
```

Verification:
```bqn
sel ← Nord ∧ hasTwoHanded ∧ TwoHanded>60
Gold +↩ 1000×sel
```
```q
update Gold:Gold+1000 from w where Nord, not null TwoHanded, TwoHanded>60
update Trained:1b from w where Nord, not null TwoHanded
update Untrained:1b from w where Nord, null TwoHanded
```

## 7 - Value predicates

Compare columns pointwise and use the resulting masks for value or structural effects.

```haskell
金が五百である者に、金を百与える。
金が体力の十倍を超える者に、欲深いを与える。
持久力が重さ未満の者に、速度を半分にする。
```

Ano:
```haskell
Gold == 500 , Gold += 100
Gold > Health * 10 , +Greedy
Stamina < Weight , Speed *= 0.5
```

Verification:
```bqn
Gold +↩ 100×Gold=500
Greedy ← Greedy ∨ Gold>Health×10
Speed ×↩ 1 - 0.5×Stamina<Weight
```
```q
update Gold:Gold+100 from w where Gold=500
update Greedy:1b from w where Gold>Health*10
update Speed:Speed*0.5 from w where Stamina<Weight
```

## 8 - Relationship hops

Read another entity through a relationship component and predicate on the target's column.

```haskell
師匠の両手が八十を超える北に、金を千与える。
師匠が死んだ学生は、師事を失う。
派閥が戦争中である兵士に、士気を二十減らす。
```

Ano:
```haskell
Nord & mentor.TwoHanded > 80 , Gold += 1000
Student & mentor.Dead , -Mentored
Soldier & faction.AtWar , Morale -= 20
```

Verification:
```bqn
Gold +↩ 1000 × Nord ∧ 80 < mentor⊏TwoHanded
Mentored ← Mentored without Student∧mentor⊏Dead
Morale -↩ 20 × Soldier ∧ faction⊏AtWar
```
```q
m:update mentorTwoHanded:TwoHanded mentor from w
update Gold:Gold+1000 from m where Nord, mentorTwoHanded>80
update Mentored:0b from m where Student, mentorDead
update Morale:Morale-20 from m where Soldier, factionAtWar
```

## 9 - Named selections

Bind reusable predicates and inline them into later masks and effects.

```haskell
人間で北で両手が六十を超える者を達人と定める。
金が一万を超える者を富者と定める。
達人に、金を千与える。
富者に、派閥を敵対にし、印を与える。
達人で富者である者に、伝説を与える。
```

Ano:
```haskell
def master = Human & Nord & TwoHanded > 60
def rich   = Gold > 10000

master , Gold += 1000
rich , Faction = Hostile ; +Marked
master & rich , +Legendary
```

Verification:
```bqn
master ← Human ∧ Nord ∧ TwoHanded>60
rich ← Gold>10000
Gold +↩ 1000×master
```
```q
master:{[w] w where Human,Nord,TwoHanded>60}
rich:{[w] w where Gold>10000}
update Gold:Gold+1000 from w where Human,Nord,TwoHanded>60
```

## 10 - Value effects

Perform aligned masked arithmetic over dense columns.

```haskell
北に、金を千与える。
盗賊に、体力を二十五減らす。
商人に、金を二倍にする。
負傷者に、速度を二で割る。
```

Ano:
```haskell
Nord , Gold += 1000
Bandit , Health -= 25
Merchant , Gold *= 2
Wounded , Speed /= 2
```

Verification:
```bqn
Gold +↩ 1000×Nord
Health -↩ 25×Bandit
Gold ×↩ 1+Merchant
Speed ÷↩ 1+Wounded
```
```q
update Gold:Gold+1000 from w where Nord
update Health:Health-25 from w where Bandit
update Gold:Gold*2 from w where Merchant
update Speed:Speed%2 from w where Wounded
```

## 11 - Assignment

Replace selected values in an aligned component column.

```haskell
盗賊に、派閥を敵対にする。
死者に、戦利品を空にする。
```

Ano:
```haskell
Bandit , Faction = Hostile
Dead , Loot = Empty
```

Verification:
```bqn
Faction ↩ Hostile⌾(Bandit⊸/) Faction
Loot ↩ Empty⌾(Dead⊸/) Loot
```
```q
update Faction:`Hostile from w where Bandit
update Loot:`Empty from w where Dead
```

## 12 - Structural effects

Delete entities or change component domains, causing archetype migration.

```haskell
死者は、消える。
狂乱の標的に、狂乱を与える。
重荷を負う者は、過重を失う。
```

Ano:
```haskell
Dead , ~
Frenzy.targets , +Frenzied
Burdened , -Encumbered
```

Verification:
```bqn
world ↩ (¬Dead)/world
world ↩ regroup world with Frenzied on Frenzy⊏targets
```
```q
delete from w where Dead
update Frenzied:1b from w where id in raze targets from w where Frenzy
update Encumbered:0b from w where Burdened
```

## 13 - Sequenced effects

Batch multiple effects against the same pre-state selection and merge the deltas at one barrier.

```haskell
この標的に、五の押し戻しを行い、赤く光らせ、盾を失わせる。
両手が六十を超える北に、金を千与え、祝福を与える。
```

Ano:
```haskell
@cursor , Knockback 5 ; Flash Red ; -Shielded
Nord & TwoHanded > 60 , Gold += 1000 ; +Blessed
```

Verification:
```bqn
Knockback⌾(cursor⊸/) ⋄ Flash⌾(cursor⊸/) ⋄ Shielded removed under cursor
Gold +↩ 1000×sel ⋄ Blessed added under sel
```
```q
update pos:knockback[pos;5], Flash:`Red, Shielded:0b from w where cursor
update Gold:Gold+1000, Blessed:1b from w where Nord, TwoHanded>60
```

## 14 - Reductions

Collapse a selected vector to a scalar using an associative reducer.

```haskell
北の金の総和。
命中の耐性を引いたものの積。
隊の生存の全て。
森の炎上のいずれか。
両手が六十を超える北の数。
```

Ano:
```haskell
+/ Gold @ Nord
*/ (1 - Resist) @ Hits
&/ Alive @ Party
|/ Burning @ Forest
#/ (Nord & TwoHanded > 60)
```

Verification:
```bqn
+´ Nord/Gold
×´ Hits/(1-Resist)
∧´ Party/Alive
∨´ Forest/Burning
+´ Nord∧TwoHanded>60
```
```q
select sum Gold from w where Nord
select prd 1-Resist from w where Hits
select all Alive from w where Party
select any Burning from w where Forest
select count i from w where Nord, TwoHanded>60
```

## 15 - Named reducer form

Apply a registered reducer to a column over a selected source.

```haskell
敵の損害を脅威として畳む。
```

Ano:
```haskell
reduce(threat) Damage @ Enemies
```

Verification:
```bqn
ThreatReduce´ Enemies/Damage
```
```q
select threat Damage from w where Enemies
```

## 16 - Scans

Accumulate along the current ordered view and return a same-length column.

```haskell
経路で、重さの累積和。
連鎖で、倍率の累積積。
視線で、高さの累積最大。
```

Ano:
```haskell
+\ Weight @ (↕steps |> route A B)
*\ Multiplier @ comboChain
max\ Height @ (Eye + ↕n * fwd)
```

Verification:
```bqn
+` route/Weight
×` comboChain/Multiplier
⌈` ray/Height
```
```q
sums exec Weight from route
prds exec Multiplier from comboChain
maxs exec Height from ray
```

## 17 - Explicit scan source

Name the ordered selection rather than deriving it inline.

```haskell
経路セルに沿って、重さを足しながら走査する。
```

Ano:
```haskell
scan(+) Weight along pathCells
```

Verification:
```bqn
+` pathCells/Weight
```
```q
sums exec Weight from w where pathCells
```

## 18 - Grade and rank

Write rank back as an aligned component; use grade plus top-k to select highest threat.

```haskell
単位に、主導権の順位をスロットにする。
脅威の降順で上位五つに、標的を与える。
```

Ano:
```haskell
Unit , Slot = grade(Initiative)
top 5 (grade desc Threat) , +Targeted
```

Version B:
```haskell
Unit , Slot = rank(Initiative)
top 5 (grade desc Threat) , +Targeted
```

Verification follows Version B.

Verification:
```bqn
Slot ↩ ⍋⍋ Unit/Initiative
Targeted ← Targeted ∨ (↕≠Threat)∊5↑⍒Threat
```
```q
update Slot:iasc iasc Initiative from w where Unit
update Targeted:1b from 5 sublist `Threat xdesc select from w
```

## 19 - Pipeline top-k

Use a legible pipeline alias for ordered top-k selection.

```haskell
敵を脅威の降順に並べ、五つ取り、標的を与える。
```

Ano:
```haskell
Enemy |> order by Threat desc |> take 5 , +Targeted
```

Verification:
```bqn
Targeted ← Targeted ∨ 5↑⍒Enemy/Threat
```
```q
update Targeted:1b from 5 sublist `Threat xdesc select from w where Enemy
```

## 20 - Outer product comprehension

Construct pair domains, filter them, and emit pairwise effects.

```haskell
塔とクリープの全ての組で距離が五十未満なら、射程内を与える。
重体の全ての組で一方が他方より小さく重なるなら、衝突させる。
```

Ano:
```haskell
[ t & c , +InRange | t <- Tower, c <- Creep, dist(t, c) < 50 ]
[ a & b , Collide  | a <- Body,  b <- Body,  a < b, overlap(a, b) ]
```

Verification:
```bqn
(Tower dist⌜ Creep) < 50
(Body overlap⌜ Body) ∧ <⌜˜Body
```
```q
select from ([]t:Tower) cross ([]c:Creep) where dist[t;c]<50
select from ([]a:Body) cross ([]b:Body) where a<b, overlap[a;b]
```

## 21 - Outer product as value

Materialize or lazily represent the pairwise distance table.

```haskell
塔とクリープの距離表。
```

Ano:
```haskell
cross dist Tower Creep
```

Verification:
```bqn
Tower dist⌜ Creep
```
```q
update d:dist[t;c] from ([]t:Tower) cross ([]c:Creep)
```

## 22 - Replicate spawn

Use a per-source count to emit multiple rows per selected source.

```haskell
発生器ごとに、数だけミニオンを生成する。
巣ごとに、肥沃度だけ卵を生成する。
```

Ano:
```haskell
Spawner , spawn Minion * Count
Nest    , spawn Egg * Fertility
```

Verification:
```bqn
world ∾↩ Count/MinionRows
world ∾↩ Fertility/EggRows
```
```q
w,:raze Count#'MinionRows
w,:raze Fertility#'EggRows
```

## 23 - Expand alias

Expose replicate as a flat-map stage before spawning.

```haskell
発生器を数だけ広げ、ミニオンを生成する。
```

Ano:
```haskell
Spawner |> expand Count , spawn Minion
```

Verification:
```bqn
Count/Spawner
```
```q
raze Count#'Spawner
```

## 24 - Reshape to spatial positions

Pour ordered selections into shaped position layouts.

```haskell
兵士に、位置を八八の形へ置く。
兵士に、位置を四行の形へ置く。
弓兵に、位置を二十の列へ置く。
```

Ano:
```haskell
Soldier , pos = to 8 8
Soldier , pos = to 4 _
Archer  , pos = to 20
```

Verification:
```bqn
pos ↩ φ¨(↕8‿8) assigned to Soldier
pos ↩ φ¨(↕4‿inferred) assigned to Soldier
pos ↩ φ¨↕20 assigned to Archer
```
```q
update pos:shape[8 8;count i] from w where Soldier
update pos:shape[4 0N;count i] from w where Soldier
update pos:shape[20;count i] from w where Archer
```

## 25 - Derived columns

Define derived columns and reuse them as effect sources, predicates, folds, and ordering keys.

```haskell
損害と速度を掛け、距離で割ったものを脅威と定める。
損害を冷却で割ったものを秒間損害と定める。
敵に、優先度を脅威にする。
脅威が百を超える敵に、危険を与える。
敵の脅威の総和。
脅威の降順で並べる。
```

Ano:
```haskell
def threat = Damage * Speed / Range
def dps    = Damage / Cooldown

Enemy , Priority = threat
Enemy & threat > 100 , +Dangerous
+/ threat @ Enemy
order by threat desc
```

Verification:
```bqn
threat ← Damage×Speed÷Range
dps ← Damage÷Cooldown
Priority ↩ threat⌾(Enemy⊸/) Priority
+´ Enemy/threat
```
```q
update threat:Damage*Speed%Range,dps:Damage%Cooldown from w
update Priority:threat from w where Enemy
update Dangerous:1b from w where Enemy, threat>100
select sum threat from w where Enemy
`threat xdesc w
```

## 26 - Fold over ordered top-k

Order a source by DPS, take the first ten, then fold their threat.

```haskell
敵を秒間損害の降順に並べ、十を取り、その脅威を総和する。
```

Ano:
```haskell
+/ threat @ (order by dps desc |> take 10)
```

Version B:
```haskell
+/ threat @ (Enemy |> order by dps desc |> take 10)
```

Verification follows Version B.

Verification:
```bqn
+´ threat⊏˜10↑⍒Enemy/dps
```
```q
select sum threat from 10 sublist `dps xdesc select from w where Enemy
```

## 27 - Coordinate lattice patterns

Generate a finite lattice and filter by coordinate predicates.

```haskell
八八の升で、xとyの和が偶数の升に小麦を生成する。
八八の升で、xとyの和が奇数の升に大麦を生成する。
八八の升で、xがyと同じ升に柱を生成する。
八八の升で、xとyの和が八未満の升に建築可を与える。
八八の升で、xが三で割り切れる升に柵を生成する。
```

Ano:
```haskell
8 8 & (x + y) % 2 == 0 , spawn Wheat
8 8 & (x + y) % 2 == 1 , spawn Barley
8 8 & x == y , spawn Pillar
8 8 & x + y < 8 , +Buildable
8 8 & x % 3 == 0 , spawn Fence
```

Verification:
```bqn
0=2|+⌜˜↕8
1=2|+⌜˜↕8
=⌜˜↕8
8>+⌜˜↕8
0=3|↕8
```
```q
cells:([]x:til 8) cross ([]y:til 8)
select from cells where 0=(x+y) mod 2
select from cells where x=y
```

## 28 - Computed line generation

Generate rank-1 lines and map the index or recurrence-derived offsets into world positions.

```haskell
十二の列で、前の二つのオフセットの和をオフセットとし、チーズをプレイヤー位置にその分ずらして生成する。
十二の列で、索引と黄金角から極座標を作り、チーズを生成する。
八の列で、索引の二倍だけずらして柱を生成する。
```

Ano:
```haskell
12 , offset = prev.offset + prev.prev.offset
   , spawn Cheese at Player.pos + (offset, 0)
12 , spawn Cheese at Player.pos + polar(index, index * 137.5)
8  , spawn Pillar at Player.pos + (index * 2, 0)
```

Version B:
```haskell
12 , offset = fib(index)
   , spawn Cheese at Player.pos + (offset, 0)
12 , spawn Cheese at Player.pos + polar(index, index * 137.5)
8  , spawn Pillar at Player.pos + (index * 2, 0)
```

Version A assumes boundary values for the first cells; Version B hides the boundary values inside `fib`.

Verification follows Version A for the recurrence and the shared lines for both versions.

Verification:
```bqn
offset ↩ »offset + »»offset
pos ↩ PlayerPos + Polar¨(↕12)∾(↕12)×137.5
pos ↩ PlayerPos + 2×↕8
```
```q
line12:([]index:til 12)
update offset:prev[offset]+prev2[offset] from line12
update pos:PlayerPos+polar[index;index*137.5] from line12
update pos:PlayerPos+2*index from ([]index:til 8)
```

## 29 - Assign spiral to existing entities

Use the selected entity order as an index source and assign computed positions.

```haskell
硬貨に、索引と黄金角の極座標をプレイヤー位置に足した位置を与える。
```

Ano:
```haskell
Coin , pos = Player.pos + polar(index, index * 137.5)
```

Verification:
```bqn
pos ↩ PlayerPos + Polar¨(↕≠Coin)∾(↕≠Coin)×137.5
```
```q
update pos:PlayerPos+polar[index;index*137.5] from update index:i from w where Coin
```

## 30 - Space reductions and scans

Fold spatial fields, scan cost fields, and scan sightlines.

```haskell
六十四六十四の標高の総和。
辺境の脅威の最大。
六十四六十四の費用の累積和。
目から北への視線で高さの累積最大。
```

Ano:
```haskell
+/ Elevation @ 64 64
max/ Threat @ Frontier
+\ Cost @ 64 64
max\ Height @ (Eye + ↕n * north)
```

Version B:
```haskell
+/ Elevation @ 64 64
max/ Threat @ Frontier
scan2(+) Cost @ 64 64
max\ Height @ (Eye + ↕n * north)
```

As written, `+\` is a leading-axis scan; a full summed-area table would be a two-axis scan over the lattice.

Verification follows Version B for the cost line and the shared lines for both versions.

Verification:
```bqn
+´⥊Elevation
⌈´Frontier/Threat
+` Cost
⌈` ray/Height
```
```q
sum Elevation
max exec Threat from cells where Frontier
sums Cost
maxs exec Height from ray
```

## 31 - Spatial top-k

Grade finite spatial fields and select the best cells.

```haskell
六十四六十四で安全の降順の上位八つに、番兵を生成する。
六十四六十四で資源の降順の上位五つに、採掘点を与える。
```

Ano:
```haskell
top 8 (grade desc Safety @ 64 64) , spawn Sentry
64 64 & top 5 (grade desc Resource) , +MiningNode
```

Verification:
```bqn
8↑⍒Safety
5↑⍒Resource
```
```q
8 sublist `Safety xdesc cells
5 sublist `Resource xdesc cells
```

## 32 - Density fields

Use spatial scalar fields as per-cell spawn counts.

```haskell
六十四六十四で、密度だけ木を生成する。
六十四六十四で肥沃度が零を超える升に、肥沃度だけ作物を生成する。
```

Ano:
```haskell
64 64 , spawn Tree * Density
64 64 & Fertility > 0 , spawn Crop * Fertility
```

Verification:
```bqn
world ∾↩ Density/TreeRows
world ∾↩ (Fertility>0)/Fertility/CropRows
```
```q
w,:raze Density#'TreeRows
w,:raze Fertility#'CropRows where Fertility>0
```

## 33 - Spatial derived fields

Define coordinate and neighbor-derived fields and use them in spatial predicates.

```haskell
xを八で割った正弦とyを八で割った正弦の和を尾根と定める。
尾根が零未満であることを盆地と定める。
高さと隣の高さの差の絶対値を傾斜と定める。
六十四六十四で尾根が零点五を超える升に、峰を生成する。
六十四六十四で盆地である升に、水を百にする。
六十四六十四で傾斜が三十を超える升に、崖を与える。
```

Ano:
```haskell
def ridge = sin(x / 8) + sin(y / 8)
def basin = ridge < 0
def slope = abs(Height - neighbor.Height)

64 64 & ridge > 0.5 , spawn Peak
64 64 & basin , Water = 100
64 64 & slope > 30 , +Cliff
```

Version B:
```haskell
def ridge = sin(x / 8) + sin(y / 8)
def basin = ridge < 0
def slope = abs(Height - neighbor(clamp).Height)

64 64 & ridge > 0.5 , spawn Peak
64 64 & basin , Water = 100
64 64 & slope > 30 , +Cliff
```

Version A assumes a declared default stencil and boundary rule; Version B makes the boundary policy visible.

Verification follows Version B for the slope line and the shared lines for both versions.

Verification:
```bqn
ridge ← (1○x÷8)+1○y÷8
basin ← ridge<0
slope ← |Height-«Height
```
```q
update ridge:sin[x%8]+sin[y%8] from cells
update basin:ridge<0 from cells
update slope:abs Height-neighborHeight from cells
```

## 34 - Board literal

Reshape a glyph string into a board and map each glyph to a spawned piece.

```haskell
盤面文字列を八八へ置き、文字から駒を生成する。
```

Ano:
```haskell
"RNBQKBNR/PPPPPPPP/......../......../......../......../pppppppp/rnbqkbnr"
  to 8 8 , spawn (pieceOf char)
```

Verification:
```bqn
PieceOf¨ 8‿8⥊glyphs
```
```q
update piece:pieceOf each char from board:shape[8 8;glyphs]
```

## 35 - Two habitats table

Summarize the same operations over ordered entity vectors and rank-2 spatial lattices.

```haskell
実体では一階の鍵、空間では二階の升として同じ計算を読む。
```

Ano:
```haskell
                  over entities (rank 1)        over space (rank 2)
generator   n            (↕n)             w h          (↕ w‿h)
reduce      +/ Gold @ Nord               +/ Elevation @ 64 64
scan        +\ Damage @ graded           +\ Cost @ 64 64
grade       top 5 (grade Threat)         top 8 (grade Safety @ 64 64)
outer       [f | a<-A, b<-B]             64 64 & (x+y)%2==0
replicate   spawn Minion * Count         spawn Tree * Density
reshape     to Unit                      to Soldier 4 _
recurrence  prev.X       (shift »)       neighbor.X   (shift «/»)
def         def threat = Dmg*Spd/Rng     def ridge = sin(x/8)+sin(y/8)
```

Verification:
```bqn
↕n ⋄ ↕w‿h
```
```q
([]i:til n)
([]x:til w) cross ([]y:til h)
```

## 36 - Farm interlude

Combine lattice generation, grouped counts, ranking, stencil growth, diffusion, structural harvest, subsidy, and global interest.

```haskell
十六十六で市松模様の升に、小麦を生成する。
囲いに、家畜で牛であるものの数を頭数にする。
牛で体重が牛の平均体重未満のものに、印を与える。
牛に、乳量の順位に間隔を掛けたものを位置のxにする。
植わっていない区画で、植わった隣が二つ以上なら、植えたものにする。
区画に、隣の水分の平均を水分にする。
成長が百以上の作物に、産物を生成し、消える。
面積が農場平均未満の農場に、金を五百与える。
金に、金を一・〇五倍にする。
```

Ano:
```haskell
16 16 & (x + y) % 2 == 0 , spawn Wheat
Pen , Headcount = #/ (livestock & Cattle)
Cow & Weight < avg/ Weight @ Cow , +Marked
Cow , pos.x = grade(Milk) * spacing
Plot & !Planted & #/(neighbors & Planted) >= 2 , +Planted
Plot , Moisture = avg/ Moisture @ neighbors
Crop & Growth >= 100 , spawn Produce ; ~
Farm & Acreage < avg/ Acreage @ Farm , Gold += 500
Gold , Gold = Gold * 1.05
```

Version B:
```haskell
16 16 & (x + y) % 2 == 0 , spawn Wheat
Pen , Headcount = #/ (livestock & Cattle)
Cow & Weight < avg/ Weight @ Cow , +Marked
Cow , pos.x = rank(Milk) * spacing
Plot & !Planted & #/(neighbors & Planted) >= 2 , +Planted
Plot , Moisture = avg/ Moisture @ neighbors
Crop & Growth >= 100 , spawn Produce ; ~
Farm & Acreage < avg/ Acreage @ Farm , Gold += 500
Gold , Gold = Gold * 1.05
```

Verification follows Version B for the herd line and the shared lines for both versions.

Verification:
```bqn
0=2|+⌜˜↕16
Headcount ↩ +´ Cattle/livestock
Marked ← Weight < (+´÷≠) Cow/Weight
pos.x ↩ (⍋⍋Milk)×spacing
Planted ← Planted ∨ (¬Planted)∧2≤+´neighbors∧Planted
Moisture ↩ (+´÷≠) neighbor⊏Moisture
Gold ×↩ 1.05
```
```q
update Headcount:count each livestock where Cattle from Pen
update Marked:1b from w where Cow, Weight<avg Weight
update pos.x:spacing*iasc iasc Milk from w where Cow
update Gold:Gold+500 from w where Farm, Acreage<avg Acreage
update Gold:Gold*1.05 from w
```

## 37 - Tier 1 space examples

Show space as generated ground, projection, and ordered scan.

```haskell
八八の偶数升に小麦を生成する。
六十四六十四の標高の総和。
六十四六十四の費用の累積和。
```

Ano:
```haskell
8 8 & (x + y) % 2 == 0 , spawn Wheat
+/ Elevation @ 64 64
+\ Cost @ 64 64
```

Verification:
```bqn
0=2|+⌜˜↕8
+´⥊Elevation
+`Cost
```
```q
select from cells where 0=(x+y) mod 2
sum Elevation
sums Cost
```

## 38 - Tier 2 record examples

Show record-aligned value write, scalar read, and rank scattered back to identities.

```haskell
両手が六十を超える北に、金を千与える。
北の金の総和。
単位に、金の順位を順位にする。
```

Ano:
```haskell
Nord & TwoHanded > 60 , Gold += 1000
+/ Gold @ Nord
Unit , Rank = grade(Gold)
```

Version B:
```haskell
Nord & TwoHanded > 60 , Gold += 1000
+/ Gold @ Nord
Unit , Rank = rank(Gold)
```

Verification follows Version B for the rank line and the shared lines for both versions.

Verification:
```bqn
Gold +↩ 1000×Nord∧TwoHanded>60
+´Nord/Gold
Rank ↩ ⍋⍋Gold
```
```q
update Gold:Gold+1000 from w where Nord, TwoHanded>60
select sum Gold from w where Nord
update Rank:iasc iasc Gold from w where Unit
```

## 39 - Tier 3 opaque examples

Use an exposed matrix view for native algebra; hand true opaque values to host routines.

```haskell
節点に、隣接行の総和を出次数にする。
敵対者に、隣接グラフで最短経路を求める。
この標的に、行動木を走らせる。
```

Ano:
```haskell
Node , OutDeg = +/ Adj@row
Hostile , shortestPath via Adj
@cursor , runBehaviorTree
```

Verification:
```bqn
OutDeg ↩ +´ AdjRow
# graph view has no native expression; dispatch to host
```
```q
update OutDeg:sum each AdjRows from nodes
shortestPath[Hostile;Adj]
runBehaviorTree each select from w where cursor
```

---

# ano_nihongo.md examples

## 40 - Japanese tokenizer skin

Show that the Japanese sentence and the ASCII sentence emit the same token stream modulo reader surface.

```haskell
北 と 両手 六十 より 、 金 に 千 たす
```

Ano:
```haskell
Nord & TwoHanded > 60 , Gold += 1000
```

Verification:
```bqn
Gold +↩ 1000×Nord∧TwoHanded>60
```
```q
update Gold:Gold+1000 from w where Nord, TwoHanded>60
```

## 41 - Japanese dotted hop

Use の as the relationship gather.

```haskell
北 と 師匠 の 両手 八十 より 、 金 に 千 たす
```

Ano:
```haskell
Nord & mentor.TwoHanded > 80 , Gold += 1000
```

Verification:
```bqn
Gold +↩ 1000×Nord∧80<mentor⊏TwoHanded
```
```q
update Gold:Gold+1000 from mentorJoin where Nord, mentorTwoHanded>80
```

## 42 - Japanese scoped selection

Use で as the locative scope marker.

```haskell
商人 ホワイトラン で 、 金 に 五千 たす
```

Ano:
```haskell
Merchant @ Whiterun , Gold += 5000
```

Verification:
```bqn
Gold +↩ 5000×Merchant∧Whiterun
```
```q
update Gold:Gold+5000 from w where Merchant, Region=`Whiterun
```

## 43 - Relative-clause selection

Use a full clause before a noun as the selector.

```haskell
両手が六十を超える北に、金を千与える。
```

Ano:
```haskell
Nord & TwoHanded > 60 , Gold += 1000
```

Verification:
```bqn
Gold +↩ 1000×Nord∧TwoHanded>60
```
```q
update Gold:Gold+1000 from w where Nord, TwoHanded>60
```

## 44 - Fallen master

Read the mentor through the relationship hop and remove a component when the mentor is dead.

```haskell
師匠が死んだ北は、訓練を失う。
```

Ano:
```haskell
Nord & mentor.Dead , -Trained
```

Verification:
```bqn
Trained removed where Nord∧mentor⊏Dead
```
```q
update Trained:0b from mentorJoin where Nord, mentorDead
```

## 45 - Japanese fold noun phrase

Use の as gather and 総和 as reduction.

```haskell
北全員の金の総和。
```

Ano:
```haskell
+/ Gold @ Nord
```

Verification:
```bqn
+´Nord/Gold
```
```q
select sum Gold from w where Nord
```

## 46 - Japanese spatial recurrence

Use で for row scope, は for distributed topic, and 前 for the shift.

```haskell
升目の列で、各升の高さは前の二升の和となる。
```

Ano:
```haskell
Cell @ row , Height = prev.Height + prev.prev.Height
```

Version B:
```haskell
Cell @ row , Height = fib(index)
```

Version A assumes row order and boundary values for the first cells; Version B hides the boundary values inside `fib`.

Verification follows Version A.

Verification:
```bqn
Height ↩ »Height + »»Height
```
```q
update Height:prev[Height]+prev2[Height] from row
```

## 47 - Counters and becoming

Use 三ヶ月 as a typed duration literal and なる for the resulting value change.

```haskell
蔵で三ヶ月より熟成したチーズは、値が倍になる。
```

Ano:
```haskell
Cheese @ cellar & Aged > 3mo , Price *= 2
```

Verification:
```bqn
Price ×↩ 1 + Cheese∧cellar∧Aged>3mo
```
```q
update Price:Price*2 from w where Cheese, cellar, Aged>3mo
```
