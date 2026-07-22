考案: Anghel (Anghel4d)

# ano examples

Ano is authoritative. BQN and q snippets show the neighboring array or relational operation.

## 1 - Canonical masked update

Select every Nord whose Two-Handed value exceeds 60 and add 1000 Gold.

```haskell
Nord & TwoHanded > 60 , Gold += 1000
```

```bqn
Gold +↩ 1000 × Nord ∧ TwoHanded>60
```

```q
update Gold:Gold+1000 from w where Nord, TwoHanded>60
```

## 2 - Core sentence form

Filter a source and apply an effect to the selected view.

```haskell
source & predicate , effect
```

```q
update effect from source where predicate
```

## 3 - Component masks

Bare component names are presence masks. `:Bandit` is a symbol atom.

```haskell
Nord , Gold += 100
Dragon , Health = 0
Bandit & !Dead & Faction == :Bandit , Faction = :Hostile
```

```bqn
gold +↩ 100×nord
health ↩ (0¨)⌾(dragon⊸/) health
faction ↩ (symHostile¨)⌾((bandit ∧ ¬dead ∧ faction=symBandit)⊸/) faction
```

```q
update Gold:Gold+100 from w where Nord
update Health:0 from w where Dragon
update Faction:`Hostile from w where Bandit,not Dead,Faction=`Bandit
```

## 4 - Scoped selection

`@` evaluates a selection within a declared scope.

```haskell
Merchant @ Whiterun , Gold += 5000
```

```bqn
Gold +↩ 5000 × Merchant ∧ Whiterun
```

```q
update Gold:Gold+5000 from w where Merchant,Region=`Whiterun
```

## 6 - Presence patterns

Presence may be constrained, unconstrained, or absent.

```haskell
(Nord, TwoHanded > 60) , Gold += 1000
(Nord, TwoHanded _) , +Trained
(Nord, !TwoHanded) , +Untrained
```

```q
update Gold:Gold+1000 from w where Nord,not null TwoHanded,TwoHanded>60
update Trained:1b from w where Nord,not null TwoHanded
update Untrained:1b from w where Nord,null TwoHanded
```

## 7 - Value predicates

Comparisons produce pointwise masks.

```haskell
Gold == 500 , Gold += 100
Gold > Health * 10 , +Greedy
Stamina < Weight , Speed *= 0.5
```

```bqn
Gold +↩ 100×Gold=500
Greedy ← Greedy ∨ Gold>Health×10
Speed ×↩ 1 - 0.5×Stamina<Weight
```

## 8 - Relationship hops

A functional relationship gathers the target column. Missing and dangling targets fail foundness.

```haskell
Nord & mentor.TwoHanded > 80 , Gold += 1000
Student & mentor.Dead , -Mentored
Soldier & faction.AtWar , Morale -= 20
```

```bqn
found ← (mentor≥0) ∧ (0⌈mentor)⊏exists
gold +↩ 1000×nord∧found∧80<(0⌈mentor)⊏twoHanded
```

## 9 - Named selections

A `def` names a predicate that is gathered at each statement.

```haskell
def master = Human & Nord & TwoHanded > 60
def rich = Gold > 10000

master , Gold += 1000
rich , Faction = :Hostile ; +Marked
master & rich , +Legendary
```

## 10 - Value effects

Value effects are masked column operations.

```haskell
Nord , Gold += 1000
Bandit , Health -= 25
Merchant , Gold *= 2
Wounded , Speed /= 2
```

```q
update Gold:Gold+1000 from w where Nord
update Health:Health-25 from w where Bandit
update Gold:Gold*2 from w where Merchant
update Speed:Speed%2 from w where Wounded
```

## 11 - Assignment

Assignment replaces selected values.

```haskell
Bandit , Faction = :Hostile
Dead , Loot = :Empty
```

```bqn
faction ↩ (hostile¨)⌾(bandit⊸/) faction
loot ↩ (empty¨)⌾(dead⊸/) loot
```

## 12 - Structural effects

Structural effects alter entity presence or component domains. The set hop selects the union of its fibers, so each reached target is affected once.

```haskell
Dead , ~
Frenzy.targets' , +Frenzied
Burdened , -Encumbered
```

```q
delete from w where Dead
update Frenzied:1b from w where id in raze exec targets from w where Frenzy
update Encumbered:0b from w where Burdened
```

## 13 - Sequenced effects

`;` batches effects against one pre-state and commits them at one barrier.

```haskell
^cursor , Knockback 5 ; Flash :Red ; -Shielded
Nord & TwoHanded > 60 , Gold += 1000 ; +Blessed
```

```q
update pos:knockback[pos;5],Flash:`Red,Shielded:0b from w where cursor
update Gold:Gold+1000,Blessed:1b from w where Nord,TwoHanded>60
```

## 14 - Reductions

A fold over a declared order is exact left accumulation. Regrouping requires associativity. Discarding traversal order also requires commutativity.

```haskell
+/ Gold @ Nord
*/ (1 - Resist) @ Hits
&/ Alive @ Party
|/ Burning @ Forest
|/ Threat @ Frontier
&/ Distance @ Route
#/ (Nord & TwoHanded > 60)
```

`|` is OR on masks and maximum on numbers. `&` is AND on masks and minimum on numbers. An empty fold yields its registered identity or no result row.

```q
select sum Gold from w where Nord
select all Alive from w where Party
select any Burning from w where Forest
select max Threat from w where Frontier
select min Distance from w where Route
```

## 15 - Named reducer form

The registry resolves the accumulator step and its available identity, finish, and laws.

```haskell
threat/ Damage @ Enemies
fold(threat) Damage @ Enemies
```

## 16 - Scans

A scan returns every successive accumulator state on the declared order.

```haskell
+\ Weight @ (til steps |> route A B)
*\ Multiplier @ comboChain
|\ Height @ Ray
&\ Depth @ Descent
```

The operand carrier selects running OR or maximum for `|\` and running AND or minimum for `&\`. An empty scan is an empty column.

```q
sums exec Weight from route
prds exec Multiplier from comboChain
maxs exec Height from ray
mins exec Depth from descent
```

## 17 - Explicit scan source

The long form names the accumulator and order.

```haskell
scan(+) Weight along pathCells
```

```bqn
+` pathCells⊏Weight
```

## 18 - Grade and rank

Grade creates an ordering witness. Rank creates aligned values.

```haskell
Unit , Slot = rank(Initiative)
top 5 (grade desc Threat) , +Targeted
```

```q
update Slot:(asc distinct Initiative)?Initiative from w where Unit
update Targeted:1b from 5 sublist `Threat xdesc select from w
```

## 20 - Outer product comprehension

A comprehension constructs a pair domain and filters it before the effect.

```haskell
[ t & c , +InRange | t <- Tower, c <- Creep, dist(t, c) < 50 ]
[ a & b , Collide | a <- Body, b <- Body, a < b, overlap(a, b) ]
```

```q
select from ([]t:Tower) cross ([]c:Creep) where dist[t;c]<50
select from ([]a:Body) cross ([]b:Body) where a<b,overlap[a;b]
```

## 22 - Replicate spawn

A per-source count creates that many effect rows for each selected source.

```haskell
Spawner , spawn Minion * Count
Nest , spawn Egg * Fertility
```

```q
w,:raze Count#'MinionRows
w,:raze Fertility#'EggRows
```

## 23 - Expand alias

The pipeline form exposes replicate as a flat-map stage.

```haskell
Spawner |> expand Count , spawn Minion
```

```bqn
Count/Spawner
```

## 25 - Derived columns

A derived column composes into effects, predicates, folds, and orderings.

```haskell
def threat = Damage * Speed / Range
def dps = Damage / Cooldown

Enemy , Priority = threat
Enemy & threat > 100 , +Dangerous
+/ threat @ Enemy
order by threat desc
```

```q
update threat:Damage*Speed%Range,dps:Damage%Cooldown from w
update Priority:threat from w where Enemy
select sum threat from w where Enemy
```

## 26 - Fold over ordered top-k

The pipeline constructs the ordered fold input.

```haskell
+/ threat @ (Enemy |> order by dps desc |> take 10)
```

```bqn
+´ (Enemy/threat) ⊏˜ 10↑⍒Enemy/dps
```

```q
select sum threat from 10 sublist `dps xdesc select from w where Enemy
```

## 38 - Entity-domain examples

Entity queries preserve lineage for scalar reads and aligned writes.

```haskell
Nord & TwoHanded > 60 , Gold += 1000
+/ Gold @ Nord
Unit , Rank = rank(Gold)
```

## 39 - Host-dispatch examples

Native carriers use native algebra. Opaque carriers pass through registered host routines.

```haskell
Node , OutDeg = +/ Adj@row
Hostile , shortestPath via Navmesh
^cursor , runBehaviorTree
```

## 40 - Japanese tokenizer skin

The Japanese and ASCII readers produce the same core statement.

```haskell
北 と 両手 六十 より 、 金 に 千 たす
```

```haskell
あの Nord & TwoHanded > 60 が Gold += 1000
```

```haskell
Nord & TwoHanded > 60 , Gold += 1000
```

## 41 - Japanese dotted hop

`の` reads the relationship gather.

```haskell
北 と 師匠 の 両手 八十 より 、 金 に 千 たす
```

```haskell
Nord & mentor.TwoHanded > 80 , Gold += 1000
```

## 42 - Japanese scoped selection

`で` reads the locative scope.

```haskell
商人 ホワイトラン で 、 金 に 五千 たす
```

```haskell
Merchant @ Whiterun , Gold += 5000
```

## 43 - Relative-clause selection

A full clause may precede the selected noun.

```haskell
両手が六十を超える北に、金を千与える。
```

```haskell
Nord & TwoHanded > 60 , Gold += 1000
```

## 45 - Japanese fold noun phrase

`の` gathers and `総和` reduces.

```haskell
北全員の金の総和。
```

```haskell
+/ Gold @ Nord
```

## 48 - Zero subject default

An effect without a subject uses the antecedent saved in the block or the host default `^cursor` when the block opens cold.

```haskell
小麦を生やす。
```

```haskell
spawn Wheat
```

## 49 - Block-local anaphora

A continuation is a new barrier over the saved selection mask. It does not re-gather the predicate.

```haskell
死んだ北が、幽霊を生み、消える。
```

```haskell
Nord & Dead , spawn Ghost
~
```

The first barrier spawns the ghosts. The second despawns only the entities in the saved mask.
