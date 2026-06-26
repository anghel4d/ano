# Ano (あの)

The Japanese distal demonstrative あの ("that one over there").
Address by description. The selection predicate is the entity reference.


## Intro

In Skyrim, gold goes to one entity, named by alias or by handle:

```text
player.additem f 1000
```

To reach a merchant instead, you click it, or you look up its FormID. To reach *every* Nord with a high Two-Handed skill across the whole map, you write a script with a loop.

> But what if I wanted to add that amount of gold to any arbitrary selection of Nords, with a specific trait, across the entire map?

In anolang:

```haskell
Nord & TwoHanded > 60 , Gold += 1000
```

No alias. No handle. No FormID. No loop. The predicate `Nord & TwoHanded > 60` is the reference, and the host resolves which entities satisfy it. One shape governs the whole static layer:

```haskell
source & predicate , effect
```

Selection on the left, effect on the right, comma between. `source` defaults to the live world. Let's dive in.

---

## Part I — Selection

### 1. Component masks

```apl
race = `nord                 ⍝ a boolean vector: which entities are Nord
(race=`nord) ∧ skill > 60    ⍝ AND of two boolean vectors
```

A bare component name is the set of entities carrying it. `&` `|` `!` build the mask.

```haskell
Nord , Gold += 100
Dragon , Health = 0
Bandit & !Dead & Faction == Bandit , Faction = Hostile
```

or, scoped to a region:

```haskell
Merchant @ cell(Whiterun) , Gold += 5000
```

### 2. Targeting aliases

```apl
⊃ ⍸ tagged                   ⍝ first index where a tag holds (a unique resolve)
```

The cursor and the viewpoint are predicates with short names, resolved per evaluation. They sit beside component masks.

```haskell
@cursor , Health = 0
@observer , Faction = Friendly
Player , Gold += 9999
@selected , Damage *= 2
```

### 3. Pattern-match selectors (Erlang)

```erlang
match(E) when race(E) =:= nord, two_handed(E) > 60 -> ...
```

The Erlang tuple-with-guard spells the selector positionally. Component presence has three states a flat tuple cannot tell apart.

```haskell
(Nord, TwoHanded > 60) , Gold += 1000   -- present, value constrained
(Nord, TwoHanded _) , +Trained          -- present, any value
(Nord, !TwoHanded) , +Untrained         -- absent
```

### 4. Value predicates

```apl
gold = 500                   ⍝ exact match
gold > health × 10           ⍝ cross-column comparison
```

```haskell
Gold == 500 , Gold += 100
Gold > Health * 10 , +Greedy
Stamina < Weight , Speed *= 0.5
```

### 5. Relationship joins (the dotted hop)

```apl
skill[mentor]                ⍝ gather: index one column by another's entity-IDs
dead[mentor[student]]        ⍝ two hops chained
```

A relationship is a component whose value is another entity's ID. `rel.Comp` reads `rel` to get a target ID per entity, then gathers `Comp` at those IDs. One hop is one indexed read. An absent or dangling link fails the predicate, the left-join-null behavior.

```haskell
Nord & mentor.TwoHanded > 80 , Gold += 1000
Student & mentor.Dead , -Mentored
Soldier & faction.AtWar , Morale -= 20
```

### 6. Named selections

```apl
Masters ← {(race[⍵]=`nord) ∧ skill[⍵] > 60}   ⍝ name a predicate, reuse it
```

A `def` names a selection. It composes into selections, scopes, and joins, and folds into the query plan at no runtime cost.

```haskell
def master = Human & Nord & TwoHanded > 60
def rich   = Gold > 10000

master , Gold += 1000
rich , Faction = Hostile ; +Marked
master & rich , +Legendary
```

---

## Part II — Effects

### 7. Value effects (APL down a column)

```apl
gold +← 1000 × nord          ⍝ masked add: increment where the mask is 1
health -← 25 × bandit
gold ×← 2 × merchant
```

A value effect is a masked array operation over a dense column.

```haskell
Nord , Gold += 1000
Bandit , Health -= 25
Merchant , Gold *= 2
Wounded , Speed /= 2
```

### 8. Assignment

```apl
faction[bandit] ← `hostile   ⍝ overwrite where the mask holds
```

```haskell
Bandit , Faction = Hostile
Dead , Loot = Empty
```

### 9. Structural effects (relational, archetype-changing)

```apl
t ,: row                     ⍝ insert (upsert a row)
t _: idx                     ⍝ delete rows
```

Structural effects change which rows exist or which columns a row occupies. They map onto insert, delete, and the kdb+ upsert that extends a row's schema. Adding a component migrates the entity to a new archetype.

```haskell
Dead , ~                     -- despawn
Frenzy.targets , +Frenzied   -- add component
Burdened , -Encumbered       -- remove component
```

### 10. Sequenced effects (one barrier)

```apl
A ⊢ B ⊢ C                    ⍝ all evaluated against the same pre-state
```

`;` batches effects into one barrier. All observe the pre-state. The effects must commute.

```haskell
@cursor , Knockback 5 ; Flash Red ; -Shielded
Nord & TwoHanded > 60 , Gold += 1000 ; +Blessed
```

---

## Part III — Column expressions

A column expression is a vector-over-a-selection. `Gold` is one. These operators build others, and a column expression appears anywhere a component name appears: in a predicate, a fold, an effect, an ordering.

### 11. Reduction (`/`)

```apl
+/ 1 2 3 4        ⍝ 10        sum
×/ 1 2 3 4        ⍝ 24        product
⌈/ 3 1 4 1 5      ⍝ 5         max
∧/ alive          ⍝ all
```

Collapse a column to a scalar under an associative operator. `@` scopes to a selection.

```haskell
+/ Gold @ Nord              -- total Nord gold
*/ (1 - Resist) @ Hits      -- combined damage multiplier
&/ Alive @ Party            -- whole party alive
|/ Burning @ Forest         -- any tile burning
#/ (Nord & TwoHanded > 60)  -- count of masters
```

or, spelled for named reducers:

```haskell
reduce(threat) Damage @ Enemies
```

### 12. Scan (`\`)

```apl
+\ 1 2 3 4        ⍝ 1 3 6 10    running sum
⌈\ 3 1 4 1 5      ⍝ 3 3 4 4 5   running max
```

Accumulate a column along an ordered selection. Returns a column of equal length.

```haskell
+\ Weight @ path(A, B)       -- cumulative movement cost along a route
*\ Multiplier @ comboChain   -- running combo multiplier
max\ Height @ ray(Eye, fwd)  -- running peak along a sightline (occlusion)
```

or, with an explicit ordering:

```haskell
scan(+) Weight along pathCells
```

### 13. Grade and rank (`⍋ ⍒`)

```apl
⍋ 3 1 2           ⍝ 2 3 1     indices that sort ascending
V[⍋V]             ⍝ V sorted  grade used to reorder
⍋⍋ 3 1 2          ⍝ 3 1 2     rank of each element
```

Return the permutation that sorts a column, then reorder anything by it.

```haskell
Unit , Slot = grade(Initiative)        -- rank ascending into a component
top 5 (grade desc Threat) , +Targeted  -- the five highest-threat
```

or, as a pipeline:

```haskell
Enemy |> order by Threat desc |> take 5 , +Targeted
```

### 14. Outer product (`∘.`)

```apl
(⍳9) ∘.× ⍳9       ⍝ 9×9 multiplication table
∘.=⍨ ⍳4           ⍝ 4×4 identity (the diagonal)
2 | ∘.+⍨ ⍳8       ⍝ 8×8 checkerboard
```

Apply a binary operator to every pair from two sets. The relational reading is a dependent join over two generators.

```haskell
[ t & c , +InRange | t <- Tower, c <- Creep, dist(t, c) < 50 ]
[ a & b , Collide  | a <- Body,  b <- Body,  a < b, overlap(a, b) ]
```

or, the materialized matrix as a value:

```haskell
cross dist Tower Creep
```

### 15. Replicate (`/` dyadic)

```apl
3 / ⍳2            ⍝ 1 1 1 2 2 2   scalar replicate
counts / cells    ⍝ per-element multiplicity
```

A per-source count controls how many copies each row emits.

```haskell
Spawner , spawn Minion * Count       -- each spawner emits Count minions
Nest    , spawn Egg * Fertility      -- counts from a per-source column
```

or, exposing the flat-map:

```haskell
Spawner |> expand Count , spawn Minion at pos
```

### 16. Reshape (`⍴`)

```apl
3 3 ⍴ ⍳9          ⍝ 3×3 matrix from a flat vector
4 16 ⍴ army       ⍝ same army, 4 ranks of 16
```

Pour a flat selection into a spatial arrangement. The inverse of generation.

```haskell
arrange Soldier into grid 8 8    -- form into an 8×8 block
arrange Soldier into ranks 4     -- 4 rows, width inferred
arrange Archer into line 20      -- one rank of 20
```

or, the bare-grid shorthand:

```haskell
Soldier , pos = reshape 8 8
```

### 17. Named column transforms (`def` for fields)

```apl
Mean  ← +/ ÷ ≢                ⍝ fork: sum over count
Range ← ⌈/ - ⌊/              ⍝ max minus min
```

A `def` whose body is a number-over-entities is a derived column, usable wherever a raw column is.

```haskell
def threat = Damage * Speed / Range
def dps    = Damage / Cooldown

Enemy , Priority = threat            -- as an effect target
Enemy & threat > 100 , +Dangerous    -- inside a predicate
+/ threat @ Enemy                    -- folded
order by threat desc                 -- as an ordering key
```

Composed, the array operations chain:

```haskell
+/ threat @ (order by dps desc |> take 10)
```

---

## Part IV — Space

Space is the same calculus with the raggedness removed. A lattice is dense and rectangular, so every operator above applies, and most are more natural here. `grid w h` carries `x y` per cell. `region(...)`, `path(...)`, `ray(...)` are spatial sources.

### 18. Patterns from the coordinate lattice (outer product)

```apl
2 | ∘.+⍨ ⍳8       ⍝ 8×8 checkerboard (parity of coordinate sums)
∘.=⍨ ⍳8           ⍝ the diagonal
(⍳8) ∘.≤ ⍳8       ⍝ upper-triangular mask
```

A grid is `⍳` crossed with `⍳`. Every regular pattern is a predicate on the coordinate columns. Change the operator, change the pattern.

```haskell
grid 8 8 & (x + y) % 2 == 0 , spawn Wheat  at (x, y)   -- checkerboard
grid 8 8 & (x + y) % 2 == 1 , spawn Barley at (x, y)
grid 8 8 & x == y , spawn Pillar at (x, y)              -- diagonal
grid 8 8 & x + y < 8 , +Buildable                       -- triangle
grid 8 8 & x % 3 == 0 , spawn Fence at (x, y)           -- stripes
```

### 19. Generation along a computed lattice (Fibonacci, spiral)

```apl
+\ fib            ⍝ cumulative fibonacci offsets
n × 137.5         ⍝ golden-angle per index
```

`fib n` and `range n` are scalar index sources. A coordinate transform turns the index into a position. `Player.pos +` lifts into world-space.

```haskell
fib 12 , spawn Cheese at Player.pos + (n, 0)                 -- fib gaps along a line
fib 12 , spawn Cheese at Player.pos + polar(n, n * 137.5)    -- phyllotaxis spiral
range 8 , spawn Pillar at Player.pos + (n * 2, 0)           -- evenly spaced row
```

or, reshape an existing set onto the computed lattice:

```haskell
arrange Coin onto spiral(137.5)
arrange Seed onto fib_line
```

### 20. Reduction and scan over space

```apl
+/ , elevation    ⍝ total elevation (ravel then sum)
⌈/ , terrain      ⍝ highest point on the map
+\ slope          ⍝ cumulative climb along a path
```

```haskell
+/ Elevation @ grid 64 64           -- total elevation of the map
max/ Threat @ region(Frontier)      -- hottest cell in a zone
+\ Cost @ grid 64 64                -- summed-area table over a cost field
max\ Height @ ray(Eye, north)       -- occlusion test along a sightline
```

### 21. Grade over space (best cells)

```apl
⍒ , safety        ⍝ cells ordered by safety, descending
```

```haskell
top 8 (grade desc Safety @ grid 64 64) , spawn Sentry at pos   -- 8 safest cells
grid 64 64 |> order by Resource desc |> take 5 , +MiningNode    -- 5 richest tiles
```

### 22. Replicate over space (density fields)

```apl
counts / cells    ⍝ per-cell multiplicity → a population
```

```haskell
grid 64 64 , spawn Tree * Density                    -- per-cell count from a field
grid 64 64 & Fertility > 0 , spawn Crop * Fertility  -- richer cells grow more
```

### 23. Named fields (`def` over coordinates)

```apl
Ridge ← {(1○ ⍵÷8) + (1○ ⍵÷8)}    ⍝ a heightmap as a function of position
```

A field is a derived column over `x y`, or over a cell's value and its neighbor.

```haskell
def ridge = sin(x / 8) + sin(y / 8)        -- a heightmap field
def basin = ridge < 0                       -- a derived spatial predicate
def slope = abs(Height - neighbor.Height)   -- local gradient via the neighbor

grid 64 64 & ridge > 0.5 , spawn Peak at (x, y)   -- spawn on the ridges
grid 64 64 & basin , Water = 100                  -- flood the basins
grid 64 64 & slope > 30 , +Cliff                  -- steep cells become cliffs
```

### 24. Source code that looks like the result (board literal)

```apl
⍉ 8 8 ⍴ glyphs    ⍝ reshape a flat glyph string into a board
```

A string literal becomes a glyph lattice carrying a `char` per cell. A registered lookup maps each glyph to a spawn type.

```haskell
board "RNBQKBNR/PPPPPPPP/......../......../......../......../pppppppp/rnbqkbnr"
  , spawn (pieceOf char) at (x, y)
```

---

## Part V — The two habitats

```haskell
                  over entities                 over space
reduce      +/ Gold @ Nord              +/ Elevation @ grid 64 64
scan        +\ Damage @ sorted          +\ Cost @ path(A,B)
grade       top 5 (grade Threat)        top 8 (grade Safety @ grid)
outer       [f | a<-A, b<-B]            grid w h & (x+y)%2==0
replicate   spawn Minion * Count        grid & spawn Tree * Density
reshape     arrange Unit into grid      arrange Soldier into ranks 4
def         def threat = Dmg*Spd/Rng    def ridge = sin(x/8)+sin(y/8)
```

Over entities the operation joins across ragged components and leans on the archetype index. Over space it runs on a dense lattice with no join. The grammar `source & predicate , effect` holds across both.

---

## A farm interlude

```haskell
grid 16 16 & (x + y) % 2 == 0 , spawn Wheat at (x, y)        -- checkerboard field
Pen , Headcount = #/ (livestock & Cattle)                   -- count cattle per pen
Cow & Weight < avg/ Weight @ Cow , +Marked                  -- below-average weight
Cow , pos.x = grade(Milk) * spacing                         -- line the herd by yield
Plot & !Planted & #/(neighbors & Planted) >= 2 , +Planted   -- crops spread
Plot , Moisture = avg/ Moisture @ neighbors                 -- irrigation diffusion
Crop & Growth >= 100 , spawn Produce at pos ; ~             -- harvest the ripe
Farm & Acreage < avg/ Acreage @ Farm , Gold += 500          -- subsidy to small holdings
Gold , Gold = Gold * 1.05                                   -- 5% interest, world-wide
```

---

## Technical Explanation

A staging language for entity-component systems. FP / LISP / APL lineage, ASCII
surface, intended as a Lua-class embeddable scripting layer.

A script states predicates over registered components. The host engine resolves
which entities satisfy them. The selection predicate is the entity reference.
The name あの is the distal demonstrative ("that one over there").

Architecture. Scripts stage calls to host-registered functions, compile to
bytecode, JIT the predicate-and-emit hot path, and return an effect buffer
describing the work. The host interprets the buffer. The execution model is
eBPF-shaped: the script invokes only registered functions. Component types and
read/write footprints reside in the registry, declared once at registration and
referenced by name in scripts.

### Data model

Components are dense columns; an entity is a view, a key into those columns. The phrase "the entity's components" denotes the columns holding an entry under that key. Within a column the data is dense; raggedness arises only at the cross-column cut, since different keys appear in different columns. This mirrors q, where a table is stored as a collection of column lists, operations on columns are vector operations, and atomic, aggregate, and uniform functions reduce to direct memory addressing.

### Two access patterns

Down a column is array calculus, a dense vector operation with no presence test. Across columns is a relational join, an intersection over which keys appear in which columns. The count of distinct components in a selector predicts the cost: zero or one (or pure space) is a dense column op; two or more is a cross-column join. The archetype index materializes "which keys appear in this exact column set," so the join evaluates presence once per archetype rather than once per key.

### Entity as view

Named views over the column store, differing only in binding time. An archetype is a view frozen at ship time; a `def` is a view frozen by the author; a bare predicate is a view computed at evaluation. Registration is early-bound selection.

### The relationship hop

A relationship component stores another entity's ID. `rel.Comp` is the indexed gather `Comp[rel]`, a foreign-key join, total under absent or dangling links by failing the predicate. This is the relational-model ECS that Flecs and the Bevy relations work have independently converged on.

### Lineage

| Layer | Tradition | Contribution |
|---|---|---|
| Surface flavor | Haskell, Erlang | Equational read, guard idiom |
| Front door | Cortex Roleplay console | Verb-legible command ergonomics |
| Selection | q/kdb+, SQL, Datalog | Target-by-description, join, sparseness |
| Action | APL, q/kdb+ | Masked column arithmetic, the array calculus |
| Core | Lisp | Homoiconic staging, registry, macros |

### The maths

The selection sublanguage is relational algebra: selection (σ) by predicate, join (⋈) by relationship, projection (π) by component access. The column sublanguage is the array calculus: reduce, scan, grade, outer product, replicate, reshape over dense vectors. A column store unifies them, since "set of rows" and "array of values" are the same bytes viewed along two axes. The comprehension correspondence (Wadler) identifies the monad comprehension with the relational query, so the double-generator form and the join are one object, and the array operations are the map/filter/reduce that the comprehension desugars to. Totality follows from finite combinators rather than a general fixpoint: every static statement terminates because iteration is bounded by the finite entity set.

---

## Open Questions, Next Steps

```markdown
/\/\/\/\/\/\/\/\
```

- Registered functions vs. bindings vs. derived columns. The surface currently
  uses `def` for both selections and derived columns, distinguished by body
  type. A clearer lexical or sigil scheme to separate host-registered
  primitives, user bindings, and derived columns is wanted. The `@spawn`
  verb-form is retired pending this.

- Sigil overload. `@` serves both the registered-selector namespace (`@cursor`)
  and the fold scope (`+/ Gold @ Nord`). Context disambiguates; a rename of one
  removes the overload.

- Outer product as a value. The double-generator comprehension covers the
  filtered-pairs case. Whether ano lets a script hold a materialized N×M matrix
  (`cross f A B`) as a first-class value is open.

- Coordinate frames. Sources declare a frame (cell, scalar, world); `at`
  consumes it. Enforcement that a world-space `at` cannot silently consume a
  cell-space coordinate is specified but not mechanized.

- Scan ordering. Scan requires an ordered selection. The rule for an unordered
  selection (error vs. implicit `along`) is open.

- Effect commutativity. `;`-batched effects must commute for the one-barrier
  semantics. Whether the compiler verifies commutativity from registered
  footprints or trusts the author is open.

- Bootstrap host. The first compiler may be written in APL, Haskell, or OCaml
  before the self-hosted toolchain. The bytecode VM and JIT target are fixed;
  the front-end implementation language is not.



The name あの is the distal demonstrative ("that one over there").

---

# Addendum

## Table 1. The canonical task across mentioned languages

Canonical task: +1000 gold to every entity that is Nord and has Two-Handed > 60.
Secondary tasks where defined: Fold (sum gold across all Nords, returns a scalar) and Despawn (remove all Dead entities, structural effect).

### Skyrim console (Canonical)
```text
player.additem f 1000
```
Operates on the player alias. Set-wide selection is not expressible.

### Papyrus (Canonical)
```papyrus
int i = 0
while i < n
  if a[i].GetRace() == Nord && a[i].GetAV("TwoHanded") > 60
    a[i].Gold += 1000
  endif
  i += 1
endwhile
```

### APL (Canonical)
```apl
gold +← 1000 × nord ∧ 60 < twoH
```

### APL (Fold)
```apl
+/ gold × nord
```

### APL (Despawn)
```apl
entities ↓⍨← dead
```

### BQN (Canonical)
```bqn
gold +↩ 1000 × nord ∧ 60 < twoH
```

### BQN (Fold)
```bqn
+´ gold × nord
```

### あの, kanji (Canonical)
```text
北 と 両手 六十 より 、 金 に 千 たす
```
ano's `predicate , effect` form transliterated. 北 Nord, と &, 両手 Two-Handed, 六十 より is > 60, 、 the comma, 金 に 千 たす is Gold += 1000. Kanji are ideographic, so tokens read as themselves.

### あの, hiragana (Canonical)
```text
のるど と りょうて ろくじゅう より 、 きん に せん たす
```
Same form, phonetic kana. Particles carry the operators: と &, より >, に the += target, 、 the comma.

### あの, katakana (Canonical)
```text
ノルド ト リョウテ ロクジュウ ヨリ 、 キン ニ セン タス
```
Same, katakana.

### あの, no spacing (Canonical)
```text
のるどとりょうてろくじゅうより、きんにせんたす
```
Spaces dropped, the way APL runs glyphs together.

### J (Canonical)
```j
gold =: gold + 1000 * nord *. 60 < twoH
```

### J (Fold)
```j
+/ gold * nord
```

### J (Despawn)
```j
entities #~ -. dead
```

### Futhark (Canonical)
```futhark
map3 (\g n t -> g + 1000 * i32.bool (n && t > 60)) gold nord twoH
```
Pure data-parallel map. The same expression compiles to CPU or GPU.

### Futhark (Fold)
```futhark
i32.sum (map2 (\g n -> g * i32.bool n) gold nord)
```

### Futhark (Despawn)
```futhark
map (.0) (filter (\(_, d) -> !d) (zip entities dead))
```

### q / kdb+ (Canonical)
```q
update gold:gold+1000 from p where race=`nord, twohanded>60
```

### q / kdb+ (Fold)
```q
select sum gold from p where race=`nord
```

### q / kdb+ (Despawn)
```q
delete from p where dead
```

### k (Canonical)
```k
p:@[p;&(race=`nord)&th>60;{x+1000}@gold]
```

### Haskell, comprehension (Canonical)
```haskell
[ e{gold = gold e + 1000} | e <- world, race e == Nord, twoHanded e > 60 ]
```

### Haskell, lens (Canonical)
```haskell
world & each . filtered p . gold +~ 1000
```

### Haskell (Fold)
```haskell
sum [ gold e | e <- world, race e == Nord ]
```

### Haskell (Despawn)
```haskell
filter (not . dead) world
```

### OCaml, fold (Canonical)
```ocaml
List.map (fun e -> if cond e then { e with gold = e.gold + 1000 } else e) world
```

### OCaml, pipe (Fold)
```ocaml
world |> List.filter (fun e -> e.race = Nord) |> List.fold_left (fun a e -> a + e.gold) 0
```

### OCaml, Seq (Fold)
```ocaml
World.to_seq |> Seq.filter is_nord |> Seq.map gold |> Seq.fold_left (+) 0
```

### OCaml (Despawn)
```ocaml
List.filter (fun e -> not e.dead) world
```

### F# (Canonical)
```fsharp
world |> Seq.map (fun e -> if cond e then { e with Gold = e.Gold + 1000 } else e)
```

### Erlang, comprehension + guard (Canonical)
```erlang
[ give(E, 1000) || E <- World, race(E) == nord, two_handed(E) > 60 ].
```

### Erlang qlc (Canonical)
```erlang
qlc:q([ E || E <- mnesia:table(ents), race(E) == nord, two_handed(E) > 60 ]).
```

### Elixir, comprehension (Canonical)
```elixir
for e <- world, e.race == :nord, e.two_handed > 60, do: %{e | gold: e.gold + 1000}
```

### Elixir, Ecto (Canonical)
```elixir
from e in W, where: e.race == ^:nord and e.two_handed > 60, update: [inc: [gold: 1000]]
```

### LINQ, method (Canonical)
```csharp
World.Where(e => e.Race == Nord && e.TwoHanded > 60).ToList().ForEach(e => e.Gold += 1000);
```

### LINQ, query (Canonical)
```csharp
from e in World where e.Race == Nord && e.TwoHanded > 60 select Bump(e)
```

### SQL (Canonical)
```sql
UPDATE ents SET gold = gold + 1000 WHERE race = 'nord' AND twohanded > 60;
```

### SQL (Fold)
```sql
SELECT sum(gold) FROM ents WHERE race = 'nord';
```

### SQL (Despawn)
```sql
DELETE FROM ents WHERE dead;
```

### Datalog (Canonical)
```datalog
master(E) :- race(E, nord), skill(E, twoHanded, V), V > 60.
```
The rule body is the selection. The effect is applied via an extension over the derived predicate `master`.

### Differential dataflow (Canonical)
```rust
ents.filter(|e| e.nord && e.th > 60).map(|e| (e.id, 1000))
```

### Differential dataflow (Despawn)
```rust
ents.filter(|e| !e.dead).consolidate()
```

### Prolog (Canonical)
```prolog
forall((ent(E), race(E, nord), th(E, V), V > 60), add_gold(E, 1000)).
```

### Common Lisp, loop (Canonical)
```lisp
(dolist (e world)
  (when (and (nordp e) (> (two-handed e) 60))
    (incf (gold e) 1000)))
```

### Common Lisp, loop (Fold)
```lisp
(loop for e in world when (nordp e) sum (gold e))
```

### Common Lisp, reduce (Fold)
```lisp
(reduce #'+ (mapcar #'gold (remove-if-not #'nordp world)))
```

### Common Lisp (Despawn)
```lisp
(remove-if #'deadp world)
```

### Clojure, seq (Canonical)
```clojure
(map #(update % :gold + 1000) (filter master? world))
```

### Clojure, thread (Fold)
```clojure
(->> world (filter nord?) (map :gold) (reduce +))
```

### Clojure, transduce (Fold)
```clojure
(transduce (comp (filter nord?) (map :gold)) + 0 world)
```
Transducers fuse filter and map into a single pass with no intermediate allocation, matching the structure of the ano planner's fused column scan.

### Clojure (Despawn)
```clojure
(remove :dead world)
```

### Factor (Canonical)
```factor
world [ master? ] filter [ 1000 + ] map-gold
```

### Lean 4 (Canonical)
```lean
world.filter master |>.map (fun e => { e with gold := e.gold + 1000 })
```

### Agda (Canonical)
```agda
map (λ e → record e { gold = gold e + 1000 }) (filter master world)
```

### Lua (Canonical)
```lua
for _, e in ipairs(world) do
  if master(e) then e.gold = e.gold + 1000 end
end
```

### Starlark (Canonical)
```python
[bump(e) for e in world if e.race == "nord" and e.th > 60]
```

### Egison (Canonical)
```egison
match world (multiset ent)
  { [<cons (& nord (th (& $v ?(> 60)))) _> ...] }
```

---

## Table 2. Candidate ano surfaces

Same canonical task. Each register is a surface over the shared staged-effect core. Variants given per register: Canonical, Fold (sum Nord gold to a scalar), Join (gold to a Nord whose mentor has Two-Handed > 80). All ano code fenced as `haskell` per working convention.

### selector , effect — the original hypothesis (Canonical)
```haskell
Nord & TwoHanded > 60 , Gold += 1000
```

### selector , effect (Fold)
```haskell
+/ Gold @ Nord
```

### selector , effect (Join)
```haskell
Nord & mentor.TwoHanded > 80 , Gold += 1000
```

### qSQL register (Canonical)
```haskell
update Gold += 1000 from Nord where TwoHanded > 60
```

### qSQL register (Fold)
```haskell
select +/Gold from Nord
```

### qSQL register (Join)
```haskell
update Gold += 1000 from Nord where mentor.TwoHanded > 80
```

### qSQL terse (Canonical)
```haskell
Gold+:1000 / Nord, TwoHanded>60
```

### qSQL terse (Fold)
```haskell
+/Gold / Nord
```

### Erlang tuple-match (Canonical)
```haskell
(Nord, TwoHanded > 60) , Gold += 1000
```

### Erlang tuple-match (presence vs value)
```haskell
(Nord, TwoHanded _) , +Trained
```
`TwoHanded _` matches an entity that carries the Two-Handed component with any value. `!TwoHanded` matches an entity lacking the component. Omission of the component imposes no constraint. Component presence corresponds to archetype membership and resolves before any value read.

### Erlang tuple-match (Join)
```haskell
(Nord, mentor:(TwoHanded > 80)) , Gold += 1000
```

### Guard clause (Canonical)
```haskell
give(E) when nord(E), two_handed(E) > 60 -> Gold(E) += 1000
```

### Guard clause (Join)
```haskell
give(E) when nord(E), mentor(E, M), th(M) > 80 -> Gold(E) += 1000
```

### Pipe (Canonical)
```haskell
world |> Nord |> TwoHanded > 60 |> Gold += 1000
```

### Pipe (Fold)
```haskell
world |> Nord |> +/Gold
```

### Pipe + verbs (Canonical)
```haskell
world |> where(Nord & TwoHanded > 60) |> each(Gold += 1000)
```

### Pipe + verbs (Fold)
```haskell
world |> where(Nord) |> sum(Gold)
```

### Monad comprehension (Canonical)
```haskell
[ e.Gold += 1000 | e <- world, Nord e, TwoHanded e > 60 ]
```

### Monad comprehension (Fold)
```haskell
+/ [ Gold e | e <- world, Nord e ]
```

### Monad comprehension (Join)
```haskell
[ e.Gold += 1000 | e <- world, Nord e, m <- mentor e, TwoHanded m > 80 ]
```
The join is expressed as a second generator. The comprehension bind desugars to a relational join.

### Yield comprehension (Canonical)
```haskell
for e in world where Nord, TwoHanded > 60 yield Gold += 1000
```

### Yield comprehension (Join)
```haskell
for e in world, m in mentor e where TwoHanded m > 80 yield Gold += 1000
```

### Rule / Datalog (Canonical)
```haskell
Gold(e) += 1000 <- Nord(e), TwoHanded(e) > 60.
```

### Rule / Datalog (Join)
```haskell
Gold(e) += 1000 <- Nord(e), mentor(e, m), TwoHanded(m) > 80.
```

### Reactive rule (Canonical)
```haskell
rule: when Nord & TwoHanded > 60 -> Gold += 1000
```
Evaluates incrementally on the delta when the body's truth value changes, rather than scanning every frame.

### Reactive rule (Fold)
```haskell
track: +/Gold @ Nord
```

### Optic / point-free (Canonical)
```haskell
world . eachEntity . where(Nord & TwoHanded > 60) . gold %~ +1000
```

### Optic / point-free (Fold)
```haskell
world . eachEntity . where(Nord) . gold & sum
```

### Tacit ASCII (Canonical)
```haskell
(Nord & 60 < TwoHanded) ~> Gold +1000
```

### Tacit ASCII (Join)
```haskell
(Nord & 80 < mentor.TwoHanded) ~> Gold +1000
```

### Array-mask (Canonical)
```haskell
Gold +: 1000 * (Nord & TwoHanded > 60)
```
The selection is a boolean mask. Mutation is a masked add over the column.

### Array-mask (Fold)
```haskell
+/ Nord * Gold
```

### s-expr core, Lisp (Canonical)
```lisp
(each (and Nord (> TwoHanded 60)) (incr Gold 1000))
```
The homoiconic core. Other registers desugar to this form.

### s-expr core, Lisp (Fold)
```lisp
(sum Gold (where Nord))
```

### s-expr core, Lisp (Join)
```lisp
(each (and Nord (> (. mentor TwoHanded) 80)) (incr Gold 1000))
```

### s-expr sugar'd, Lisp + reader macros (Canonical)
```lisp
(give-gold 1000 (& Nord (> TwoHanded 60)))
```

### s-expr sugar'd, Lisp + reader macros (Join)
```lisp
(give-gold 1000 (& Nord (mentor> TwoHanded 80)))
```

### Concatenative (Canonical)
```haskell
Nord [ TwoHanded 60 > ] filter [ Gold 1000 +! ] each
```
Quotations supply the predicate and the effect. No bound names appear. Refactoring requires rewriting the stack flow.

### Set-builder math (Canonical)
```haskell
{ e.Gold += 1000 : e in World, Nord(e), TwoHanded(e) > 60 }
```

### Set-builder math (Fold)
```haskell
Sum{ Gold(e) : Nord(e) }
```

### Match block (Canonical)
```haskell
match Nord & TwoHanded > 60 { Gold += 1000 }
```

### Match block (Fold)
```haskell
fold Nord { +/ Gold }
```

### Selection-as-value (Canonical)
```haskell
let masters = Nord & TwoHanded > 60 in masters.Gold += 1000
```
The selected set is a first-class value, bound to `masters`, then acted upon. Supports passing and re-selection.

### Selection-as-value (Fold)
```haskell
let ns = Nord in +/ ns.Gold
```

### Dataflow DAG (Canonical)
```haskell
richest <- Nord |> maxby Gold ; richest.Gold += 1000
```
A fold result binds to `richest` and scopes a subsequent map. The script forms a two-node dataflow graph. The `<-` binding introduces this capability and requires an intra-script evaluation order.

### Dataflow DAG (Join)
```haskell
disciples <- Nord ⋈ mentor where m.TwoHanded > 80 ; disciples.Gold += 1000
```

### Demonstrative, あの-literal (Canonical)
```haskell
ano Nord, TwoHanded > 60 : Gold += 1000
```
`ano` marks a declarative selection. `dore` marks a query. The two keywords correspond to demonstrative and interrogative deixis. This register places the name in the grammar.

### Demonstrative, あの-literal (Fold)
```haskell
dore +/Gold : Nord
```

### Infix natural (Canonical)
```haskell
every Nord with TwoHanded > 60 gets Gold + 1000
```
Prose-shaped surface with high parsing ambiguity.

### Capability-call (Canonical)
```haskell
Gold.give(1000) @ (Nord & TwoHanded > 60)
```
The verb is a registered capability. The selection following `@` supplies its target scope.

### Capability-call (Fold)
```haskell
Gold.sum() @ Nord
```

### Sigil-mask (Canonical)
```haskell
&Gold +1000 ?[Nord TwoHanded>60]
```
`?[...]` delimits the selector. Sigils prefix the effect target and the columns read.

### OCaml match / guard (Canonical)
```ocaml
match e with _ when nord e && twohanded e > 60 -> { e with gold = gold e + 1000 }
```
The ML `when` guard carries the same selection semantics as the Erlang guard and the Datalog body. The entity is bound to `e` and visible in the effect.

### OCaml match / guard (Join)
```ocaml
match e with _ when nord e && twohanded (mentor e) > 80 -> { e with gold = gold e + 1000 }
```

### OCaml pipe + labelled verbs (Canonical)
```ocaml
world |> select ~p:(Nord & TwoHanded > 60) |> iter ~f:(Gold += 1000)
```

### OCaml pipe + labelled verbs (Fold)
```ocaml
world |> select ~p:Nord |> sum ~f:Gold
```

### OCaml PPX, ano-in-OCaml (Canonical)
```ocaml
[%ano Nord & TwoHanded > 60 => Gold += 1000]
```
ano embedded as a host EDSL via a PPX rewriter. The surface resides in OCaml source, rewrites to staged calls at host build time, and typechecks against the registry as ordinary OCaml. For an OCaml host, the bytecode path is bypassed and the plan is monomorphised at build time.

### OCaml PPX, ano-in-OCaml (Fold)
```ocaml
[%ano +/ Gold @ Nord]
```