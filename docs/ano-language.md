考案: Anghel (Anghel4d)

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

The predicate `Nord & TwoHanded > 60` is a reference, and the host resolves which entities satisfy it.

Generally, that looks something like this:
```haskell
source & predicate , effect
```

Selection on the left, effect on the right, comma between. `source` defaults to the live world.

## A Potential Comparison

```lua
local player = GetPlayer()
local playerFaction = player:GetFaction()
local cheeseCount = 122
local goldenAngle = math.pi * (3 - math.sqrt(5))  -- ~137.5 degrees

for _, npc in ipairs(GetAllActors()) do
  if npc:HasTunic("red")
    and (npc:GetRace() == "Nord" or npc:GetRace() == "Khajiit")
    and npc:GetFaction() == playerFaction then
    
    local center = npc:GetPosition()
    for i = 0, cheeseCount - 1 do
      local radius = math.sqrt(i) * 1.5
      local angle = i * goldenAngle
      local x = center.x + radius * math.cos(angle)
      local z = center.z + radius * math.sin(angle)
      SpawnObject("CheeseWheel", x, center.y, z)
    end
  end
end
```

Roughly 20 lines, and that's already assuming a clean API. In practice this is a coroutine or an event-hooked function, not a console line, because Lua has no notion of "select then broadcast an effect" as a primitive.

The Ano version:
```haskell
NPC & Tunic = :Red & (Nord | Khajiit) & Faction = Player.Faction ,
  spawn CheeseWheel * 122 at pos + phyllotaxis(index)
```

Every token is defined later in this document: `=` compares in the selector block and assigns in an effect clause. `spawn CheeseWheel * 122` binds `index` per copy, each copy reads its own NPC's `pos` (§17), and `phyllotaxis` is a callable defined in the host process and reached through the registry.

Ano is to a game world what q is to kdb+: the resident query-and-command language of a live column store. The FP/array sibling of Lua. The split with a Lua-class host is coroutines versus triggers. Cinematic sequencing, UI, and per-instance branching stay imperative. Everything statable as a condition over the world, a bulk effect, and a schedule is ano's territory. StarCraft 2's trigger editor shipped a whole campaign on that paradigm. Ano is that layer with a relational predicate language. The shape buys determinism and replay. A statement is a pure plan over a world snapshot: a predicate plus an effect buffer over pre-state. Performing the plan commits or refuses at one barrier. The same world and the same log give the same run. A mission is a text file that replays identically anywhere. Let's dive in.

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
Bandit & !Dead & Faction == :Bandit , Faction = :Hostile
```

or, scoped to a region:

```haskell
Merchant @ Whiterun , Gold += 5000
```

### 2. Targeting aliases

```apl
⊃ ⍸ tagged                   ⍝ first index where a tag holds (a unique resolve)
```

The cursor and the viewpoint are predicates with short names, resolved per evaluation. They sit beside component masks.

```haskell
^cursor , Health = 0
^observer , Faction = :Friendly
Player , Gold += 9999
^selected , Damage *= 2
```

`^cursor` is a pronoun, not a name. `Player` is a proper noun fixed at registration. `^cursor` is the word "you". Who it refers to is decided at the moment of speaking, by the engine, not the script. The registry supplies a resolver (a raycast from the mouse, the camera's focus), and every gather re-runs it. So `^cursor , Health = 0` kills whatever is under the mouse at that gather, and two statements mentioning `^cursor` may hit two different entities. That is why it carries a glyph in a language with no other sigils: the reader must know this name can move between statements. In C# it is an expression-bodied property, never a field. `Entity Cursor => Physics.Raycast(mouse)` re-raycasts on every read, like `DateTime.Now` against a stored timestamp. In Haskell it is `asks cursor` in a Reader: the script is a function of an environment the engine rebuilds each tick. Pointedly not a script-owned `IORef`: the host may replace or delete the alias between statement steps. In filesystem terms `Player` is `/home/pyrus` and `^cursor` is `./`. Against a component name the difference is arity. `Nord` is a mask over many rows. `^cursor` resolves to a referent, usable anywhere a selection or a mirror-read root goes (`^cursor.pos`).
Dynamic aliases occupy an overlay namespace keyed by the bare spelling. `^Whiterun` reads the live `Whiterun` alias target, such as another column or an existing mask, when one exists and otherwise falls through to bare `Whiterun`. Installing, rebinding, or deleting the alias changes only `^Whiterun`; bare `Whiterun` remains the registered column or binding. The caret promises only that the referent may change between statement steps.

### 3. Pattern-match selectors (Erlang)

```erlang
match(#entity{race = nord, two_handed = T}) when T > 60 -> ...
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

A relationship is a component whose value is another entity's ID. `rel.Comp` reads `rel` to get a target ID per entity, then gathers `Comp` at those IDs. One hop is one indexed read. A bare relationship in predicate position is its found mask: true exactly when the stored target resolves in the current world, not merely when an ID was assigned. The hop carries the same guard. An absent or dangling link fails the predicate, the left-join-null behavior.

```haskell
Nord & mentor.TwoHanded > 80 , Gold += 1000
Student & mentor.Dead , -Mentored
Soldier & faction.AtWar , Morale -= 20
```

`rel.Comp` is strictly functional: one ID, one indexed read. Applying the bare dot to a set-valued or inverse relationship is a compile error. The set hop is the postfix tick on the relationship name, or on a key-valued column, which reads as its inverse fibers (§13's value-level rel). It is k's each. `rel'` is the fiber at each selected entity as a per-source group. `rel'.Comp` gathers a column across it. In source position `sel.rel'` selects the image, the union of fibers. The set hop is the dot hop under each. A bare `rel'` outside a fold, quantifier, or source position is a compile error. In a boolean position it must sit under a fold. The quantifiers are the boolean folds applied to the fiber: `|/ rel'.Comp` is any, `&/ rel'.Comp` is all, `#/ (rel' & pred)` is count.

```haskell
Frenzy.targets' , +Frenzied               -- image of the fibers: each reached target, once
Plot & |/ neighbors'.Planted , +Watered   -- any: a planted neighbor exists
Pen & &/ livestock'.Healthy , +Certified  -- all: every animal in the pen healthy
```

The image is a selection and a selection is a mask. A target reachable from two sources appears once, and the effect applies once: set semantics, idempotent scatter. This is consistent with the predicate-is-the-reference stance, since masks have no multiplicity. In-degree is never silently summed into a value effect. To accumulate per in-edge, fold at the target over the inverse fiber: `Target , Hits += #/ attackers'`.

The keyed hop. A `unique` column is declared injectivity — pairwise-distinct, checked at load — and injectivity is precisely the license a hop needs, since an injective column is invertible on its image: the keyed hop is `rel ; unique⁻¹`, one index-of against the key column with one found-guard. A relationship declares its key column by writing it before the name (`rel id mentor -1 0 0 3 …`, reading type-annotation-first: mentor is a rel over id), and an undeclared rel stays keyed to the row index — zero churn for every existing world. Not-found is dangling is dead: a stored ID whose target despawned fails the found-guard and clears the mask bit, the same left-join-null, so staleness never grows a fault path (the storage mechanism is ano-ecs §2's generational compare; the diagnostic surface, never the semantics, is the debug layer's dead-link report). `DEAD` deliberately names every nonnegative stored target that fails the found-guard. Never-existed, despawned, and mistyped are not separate language states. The `-1` None sentinel stays silent. The inverse of a keyed rel is keyed automatically, and `unique` subsumes the id/keys name magic: it declares the key space. `unique` means exactly one thing: every element distinct, enforced at load and held across the run. Minting is spawn machinery, never a `unique` semantic, so several unique columns never compete for a mint; spawn fills each one fresh because the declared injectivity admits nothing else.


### 6. Named selections

```apl
Masters ← {(race[⍵]=`nord) ∧ skill[⍵] > 60}   ⍝ name a predicate, reuse it
```

A `def` names a selection. It composes into selections, scopes, and joins, and folds into the query plan at no runtime cost.

```haskell
def master = Human & Nord & TwoHanded > 60
def rich   = Gold > 10000

master , Gold += 1000
rich , Faction = :Hostile ; +Marked
master & rich , +Legendary
```

---

## Part II — Effects

### 7. Value effects (APL down a column)

```apl
gold +← 1000 × nord          ⍝ masked add: increment where the mask is 1
health -← 25 × bandit
gold ×← 1 + merchant         ⍝ masked double: ×2 on the mask, ×1 off it — 2 × merchant would zero every non-merchant
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
faction[⍸bandit] ← `hostile  ⍝ overwrite where the mask holds: ⍸ turns the mask into indices; faction[bandit] would index by the mask's 0s and 1s
```

```haskell
Bandit , Faction = :Hostile
Dead , Loot = :Empty
```

### 9. Structural effects (relational, archetype-changing)

```apl
t ,: row                     ⍝ insert (upsert a row)
t _: idx                     ⍝ delete rows
```

Structural effects change which rows exist or which columns a row occupies. They map onto insert, delete, and the kdb+ upsert that extends a row's schema. Adding a component migrates the entity to a new archetype.

する/なる mark voice; value vs structure is storage.

```haskell
Dead , ~                     -- despawn
Frenzy.targets' , +Frenzied  -- add component across the set hop's image
Burdened , -Encumbered       -- remove component
```

Spawn fill is a three-layer lookup: the proto's declared value, else the column's registered `default`, else the type's zero — num 0, bool 0, sym "", rel -1. A -1 rel is None under §5's left-join-null; kore renders it `/`, a rendering choice, never a language feature — if a surface literal ever lands it is `none` or `null`. The proto is the registered archetype, the missing noun: `def Marine soldier=1 hp=100` declares one in the registry, its first row-oriented named-value construct, and `spawn Marine` fills through it. A ghost is one bit plus the defaults; the proto is what knows Ghost.

### 10. Sequenced effects (one barrier)

```apl
A ⊢ B ⊢ C                    ⍝ all evaluated against the same pre-state
```

Every top-level statement is exactly one gather-effect-scatter barrier. `;` batches effects to the right of one comma into that barrier. All observe the statement's pre-state, and they must commute under the registered merge laws. The scatter commits at the end of the statement, and the next statement's gather observes it. Effects become visible across statements exactly at statement boundaries, in program order. `;` within one statement is the only barrier-sharing form. There is no multi-statement block barrier.

```haskell
^cursor , Knockback 5 ; Flash :Red ; -Shielded
Nord & TwoHanded > 60 , Gold += 1000 ; +Blessed
```

With the subject omitted, the block supplies ゼロが:

```haskell
spawn Wheat   --cursor , spawn Wheat
```

```haskell
Nord & Dead , spawn Ghost
~                              -- same subject, next barrier
```

A leading comma continues the antecedent explicitly, the same continuation with the effect spelled out:

```haskell
12 , offset = fib(index)
   , spawn Cheese at Player.pos + (offset, 0)   -- same subject, next barrier
```

A continuation line (`~`, a leading comma, or an elided-subject effect) is a new statement and a new barrier. It reuses the antecedent's saved selection mask, not its pre-state. Here the ghosts spawned by the first line survive the second, because the despawn applies the mask saved at the first gather, never a re-gather. The binding rule for the elided subject: `~` and the leading comma always bind the saved mask. A bare effect takes ゼロが: the antecedent's saved mask when one exists, the host default `^cursor` when the block opens cold.

### 11. Standing rules (the naru register)

```lisp
(defrule spread (plot ?p) => (assert (planted ?p)))   ; condition-action over working memory: the rule stands
```

The hinge changes, nothing else. `selection , effect` performs a command. `selection => effect` installs a standing rule. Same left side, same right side, same `;` batching. The comma is する, the performed voice. The arrow is なる, the becoming voice. The する/なる split §9 declares is carried by the hinge, and the hinge is the visible が only in the なる register (ano_nihongo.md, the する case frame). The lineage is the Datalog rule (`head :- body`, with ano's order head-final) and the production rule of OPS5 and CLIPS. A rule is named for retraction the way selections are named.

```haskell
Plot & !Planted & #/ (neighbors' & Planted) >= 2 , +Planted                  -- performed once
def spread = Plot & !Planted & #/ (neighbors' & Planted) >= 2 => +Planted    -- installed, standing
```

Schedule: once per tick, one barrier step. At the tick's ingest every installed rule re-gathers against the tick's pre-state and scatters once. No intra-tick cascading, no fixpoint. Totality comes from bounded demand. One step per tick is also the game-legible behavior: spreading crops advance one ring per tick. On-change evaluation is implementation lineage, not semantics. Rete and differential dataflow let the host fire only the rules whose footprints changed, observationally equivalent to the every-tick reading. Incrementality is an optimization the registry's read/write footprints already enable, never a semantic mode.

Conflicts: all standing rules active in a tick share one barrier, the `;` law lifted to the rule set. All observe the tick's pre-state. Overlapping writes to one cell are accepted only in three cases: the registered footprints are disjoint, the effect algebra proves a deterministic merge (additive increments commute), or complementary guard literals prove the writers' masks row-disjoint over the shared pre-state. Bloom selects under `!Planted`, wither under `Planted`, so the pair cannot touch one row (the Life pair; a column-level footprint check cannot see this, the guards can). Two rules whose overlapping writes admit none of the three are rejected at installation, statically, since the rule set is known. Rules never race commands. A tick runs ingest, then the rule barrier, then queued command statements in program order. The per-tick rule barrier is the one whole-program barrier, so the static layer needs no block form. The retraction surface is unsettled (Open Questions, Rule retraction).

The register scales from field rules to campaign logic. A quest is data: a stage column, objectives as entities, advancement as a standing rule per stage. This is the paradigm StarCraft 2's trigger editor shipped a whole campaign on: events, conditions, actions over live game state, here with a relational predicate language in place of the editor's condition list. Timers ride the host's monotonic tick counter. The statement log makes a mission a text file that replays identically anywhere. What stays with the host is the coroutine kingdom (cinematic sequencing, UI, per-instance branching) and little else.

```haskell
def stage3 = Quest & Id == :Liberation & Stage == 3 & #/ (objectives' & Complete) == 3 => Stage = 4 ; spawn Convoy at rally
```

The stage-4 rules stand inert until the data says otherwise. On advance, `stage3` must withdraw. The retraction question is load-bearing exactly here.

---

## Part III — Column expressions

A column expression is a typed column over the current query domain. If the current view has row domain `X`, every expression has the form `Col X V`: one value of `V` per row, with an optional validity mask. `Gold` is one after the query has gathered it onto `X`; a scalar is the constant column on `X`. These operators build other columns, and a column expression appears anywhere a component name appears: in a predicate, a fold, an effect, an ordering. Equal length never establishes alignment. The view carries lineage maps from `X` to the stored habitats from which its columns were gathered.

Ano's Greater/Lesser family is exactly q/kdb+'s convention over the carriers that Ano admits; this is not a claim of general q compatibility. `a | b` is OR on masks and pointwise maximum on numbers. `a & b` is AND on masks and pointwise minimum on numbers. Their folds and scans are the corresponding reductions and scans. A mask and a number never coerce into one another. Mixed application refuses. See KX's definitions of [Greater](https://code.kx.com/q/ref/greater/), [Lesser](https://code.kx.com/q/ref/lesser/), [max](https://code.kx.com/q/ref/max/), and [min](https://code.kx.com/q/ref/min/).

### 12. Reduction (`/`)

```apl
+/ 1 2 3 4        ⍝ 10        sum
×/ 1 2 3 4        ⍝ 24        product
⌈/ 3 1 4 1 5      ⍝ 5         max
∧/ alive          ⍝ all
```

Collapse a column to a scalar. On a declared traversal order, a fold is exact left accumulation and admits any carrier-compatible binary operation. An unordered fold that may regroup a fixed traversal requires associativity. A parallel/unordered fold that may also discard traversal order requires associativity and commutativity. An identity is required only if the empty scope is to yield a value. `@` scopes the fold to a selection, and a fold under `@` is always one scalar.

```haskell
+/ Gold @ Nord              -- total Nord gold
*/ (1 - Resist) @ Hits      -- combined damage multiplier
&/ Alive @ Party            -- whole party alive
|/ Burning @ Forest         -- any tile burning
|/ Threat @ Frontier        -- maximum threat
&/ Distance @ Route         -- minimum distance
#/ (Nord & TwoHanded > 60)  -- count of masters
```

Not every collapsing form is a raw reduction. The pairwise mean is not associative, and `#` is not a binary operator, so `avg/` and `#/` are derived fold-and-finish forms. `avg/` folds sum and count in one pass and divides at the end. `#/` is `+/` over the constant 1. The surface keeps the spellings, and the registry records them as fold-and-finish. That is what makes the empty case honest. A fold identity depends on the carrier. `|/` over masks yields false on empty input and `&/` yields true. Numeric `|/` and `&/` have no identity in Ano's finite float64 carrier, so an empty scope fails the row. `max/` and `min/` are bridge spellings for those numeric folds and obey the same law. `avg/` also fails an empty scope.

The slash attaches to a registered reducer name exactly as it attaches to an operator, one grammar row, and `fold(f)` is the long form of `f/`. `scan(f)` is the long form of `f\`; `scan2(f)` applies the same accumulator step over its declared second axis. The short and long spellings never select different semantics.

Operationally, Ano follows LINQ's unseeded `Aggregate` model. `f` denotes the accumulator step and is resolved through the registry: an operator selects its built-in entry and a name selects a registered reducer. On a nonempty ordered input, the first value starts the accumulator and each remaining value is applied from left to right. A fold returns the final accumulator. A scan returns the first value followed by every successive accumulator state, so it preserves input length. The registry entry supplies the step and, where applicable, its identity, finishing function, and algebraic witnesses. Haskell supplies the useful fold/scan and direction vocabulary; LINQ is the closer operational precedent because Ano dispatches the named accumulator through its registry.

Ano improves on LINQ because the registry can prove which execution strategies are legal:

- Ordered fold: any compatible registered accumulator.
- Unordered fold: requires associativity.
- Parallel/unordered fold: requires associativity and commutativity.
- Empty fold: returns the registered identity, or nothing when none exists.
- Scan: uses the same accumulator but returns every successive accumulator state.
- Empty scan: produces an empty column; it does not need an identity unless a seeded form explicitly emits the seed.

Here an unordered fold may regroup the fixed traversal but may not permute it; parallel/unordered evaluation may partition and merge without preserving traversal order. Ano has no seeded fold or scan form.

LINQ supplies query and accumulation as a library over a host language. Ano makes the query-and-accumulator model part of the language itself and joins it to the functional array calculus. The accumulator is therefore not an opaque callback: its registry entry can carry the laws that license execution freedom.

This generalization changes none of the established operators. Mask `|/` remains ANY, numeric `|/` remains maximum, `+\` remains running sum, `|\` remains running ANY or running maximum according to its carrier, and every other existing fold and scan keeps its dyad, identity, empty behavior, and prefix results. The broader contract only admits additional accumulator heads where the order and registered laws license them.

```haskell
threat/ Damage @ Enemies
fold(threat) Damage @ Enemies
```

### 13. Grouped fold (γ)

```q
select headcount: count i by pen from animal where cattle   / q: by groups, the aggregate collapses each group
```

A fold prefix over a tick-marked hop is the grouped fold: `fold/ rel'.Comp` for a gathered column, `fold/ (rel' & pred)` for a filtered fiber. A relationship is registered set-valued forward (`targets`, `neighbors`: each source maps to a set of targets) or as the inverse read of a functional relationship (`livestock`, the inverse of `pen : Animal -> Pen`). A key-valued column stands in relation position the same way. It has a functional relationship's exact shape, so `Col'` denotes its inverse fibers over the stable-id column: the value-level rel, a relation computed by the program instead of registered. For each entity in the current selection, `rel'` denotes the fiber at it, the target set for a forward relationship or the preimage for an inverse read. The fold collapses each fiber to one value per selected entity. The whole expression is a column aligned to the selection, written back under the ordinary alignment rule. This is γ: q's `by`, Datalog's grouped aggregation, expressed as fold-under-each over the fibers rather than a new clause.

```haskell
Pen , Headcount = #/ (livestock' & Cattle)    -- count per pen, over the inverse fiber
Plot , Moisture = avg/ neighbors'.Moisture    -- per-plot mean over the neighbor fiber
Target , Hits += #/ attackers'                -- in-degree, folded at the target
```

`fold/ col @ scope` remains the scoped-global fold and is always one scalar when the fold produces a row. `fold/ rel'…` is always per-source. The result-type split is lexical, never a registry lookup. After a fold, `@` yields one scalar, `'` yields a per-source column, and `@` never groups. Empty fibers use the carrier's identity when it has one. `#/` and `+/` give 0. Mask `|/` gives false and mask `&/` gives true. Numeric `|/`, numeric `&/`, `max/`, `min/`, and `avg/` fail the row. This is the §5 left-join-null rule extended from the dangling link to the empty fiber: the entity drops out of the selection and no write lands. A scoped-global identityless fold over nothing produces the same empty result as a predicate with no matches. A bare query prints nothing, never a placeholder scalar.

### 14. Scan (`\`)

```apl
+\ 1 2 3 4        ⍝ 1 3 6 10    running sum
⌈\ 3 1 4 1 5      ⍝ 3 3 4 4 5   running max
```

Accumulate a column along an ordered selection. Returns a column of equal length. Unordered selections need `along`.

```haskell
+\ Weight @ (til steps |> route A B) -- cumulative movement cost along a route
*\ Multiplier @ comboChain           -- running combo multiplier
|\ Height @ (Eye + til n * fwd)      -- running maximum along a sightline
&\ Depth @ Descent                   -- running minimum along a descent
max\ Height @ Ray                    -- numeric bridge for |\
min\ Depth @ Descent                 -- numeric bridge for &\
```

or, with an explicit ordering:

```haskell
scan(+) Weight along pathCells
```

### 15. Grade and rank (`⍋ ⍒`)

```apl
⍋ 3 1 2           ⍝ 2 3 1     indices that sort ascending
V[⍋V]             ⍝ V sorted  grade used to reorder
⍋⍋ 3 1 2          ⍝ 3 1 2     rank of each element
```

Return the permutation that sorts a column, then reorder anything by it.

A grade carries an ordering witness. Stable ties use a declared input order; dense and fractional rank are value-only alternatives. Any rank used by an effect must state its tie policy, because index, keys, bindings, relationships, and declared orders are observable structures in Ano; there is no universal permutation-invariance law for records.

```haskell
Unit , Slot = rank(Initiative)         -- value-only rank ascending into a component
top 5 (grade desc Threat) , +Targeted  -- the five highest-threat
```

or, as a pipeline:

```haskell
Enemy |> order by Threat desc |> take 5 , +Targeted
```

### 16. Outer product (`∘.`)

```apl
(⍳9) ∘.× ⍳9       ⍝ 9×9 multiplication table
∘.=⍨ ⍳4           ⍝ 4×4 identity (the diagonal)
2 | ∘.+⍨ ⍳8       ⍝ 8×8 checkerboard
```

Apply a binary operator to every pair from two sets. The relational reading is a filtered cross join over two generators: σ_p(A × B), the θ-join. A dependent join would mean the second generator's domain is a function of the first, `b <- f(a)`, which this is not.

```haskell
[ t & c , +InRange | t <- Tower, c <- Creep, dist(t, c) < 50 ]
[ a & b , Collide  | a <- Body,  b <- Body,  a < b, overlap(a, b) ]
```

or, the materialized matrix as a value:

```haskell
cross dist Tower Creep
```

### 17. Replicate (`/` dyadic)

```apl
3 / ⍳2            ⍝ 1 1 1 2 2 2   scalar replicate
counts / cells    ⍝ per-element multiplicity
```

A per-source count controls how many copies each row emits.

```haskell
Spawner , spawn Minion * Count       -- each spawner emits Count minions
Nest    , spawn Egg * Fertility      -- counts from a per-source column
```

The replicate binds `index` on each copy, 0 up to the count, restarting at every source. A copy reads its source's columns, so per-copy position math needs no loop. It is the same `index` the bare shape binds (§21), one name for the row's ordinal in whatever minted the row. Under a replicate the copy number is the innermost binding and shadows any outer one. The intro's cheese line is this rule: each wheel takes its own NPC's `pos` and its own copy number.

```haskell
Nest , spawn Egg * Fertility at pos + polar(index, index * 137.5)   -- each nest's clutch spirals around it
```

or, exposing the flat-map:

```haskell
Spawner |> expand Count , spawn Minion
```

### 18. Reshape (`⍴`)

```apl
3 3 ⍴ ⍳9          ⍝ 3×3 matrix from a flat vector
4 16 ⍴ army       ⍝ same army, 4 ranks of 16
```

Array reshape changes a value's shape. Exact equal-cardinality reshape is a reindexing; APL's cycling or truncating reshape is an output-to-input gather map, not an equivalence. The Ano form `pos = to shape` is neither: it derives one coordinate per row of the current query domain and writes those coordinates through the effect's destination lineage. It does not reshape the entity habitat, and it does not license a stored field to change rank. The surface spells this placement combinator `to`, the allative; the spelling of first-class value reshape remains open.

```haskell
Soldier , pos = to 8 8           -- form into an 8×8 block
Soldier , pos = to 4 _           -- 4 rows, width inferred
Archer  , pos = to 20            -- one rank of 20
```

### 19. Named column transforms (`def` for fields)

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
+/ threat @ (Enemy |> order by dps desc |> take 10)
```

---

## Part IV — Space

A habitat is a nominal finite index type `H`: the identity of the places at which a column may vary. A stored field is a total function `f : H → V`. Its physical column is a ravelled buffer plus a layout bijection `ℓ : Fin(n) ≅ H`; the buffer alone does not identify `H`. An ECS component is different: it is a partial column on the entity habitat `E`, represented by a presence subobject `P_C ↪ E` and a total column `c : P_C → V`. Two habitats remain foreign when cardinality, rank, shape, carrier, coordinates, and physical layout all happen to agree.

A space is not merely something 2D. It is a habitat carrying spatial structure. For `shape : Fin(r) → Nat`, `Box(shape) = ∏ⱼ Fin(shape(j))`. A named lattice habitat `H` has a nominal site type `D_H`, a product presentation `b_H : Box(shape_H) ≅ D_H`, and a chart `κ_H : D_H → Λ_H` into a free integer module `Λ_H ≅ ℤʳ`, with a bounded partial lookup back to `D_H`. The product presentation supplies semantic axes; the physical layout supplies buffer order. Neither identifies another equal-shaped habitat. The rank and shape belong to the registered presentation of `D_H`, hence to every stored field on it. A field update preserves that habitat. A filter may expose a subdomain; a generator or reshape may produce a derived domain; neither silently changes the rank or identity of the stored field.

A frame `F` is nominal and supplies distinct carriers `Point<F>` and `Vector<F>`. A position component is still entity-indexed, for example `pos : P_pos → Point<World3>` with `P_pos ↪ E`; the frame belongs to each value while the component habitat remains `P_pos`. A `Point<MarsLocal>` is not a `Point<World3>`, a `Vector<PlayerPlane2>` is not a world point, and a `CellRef<Ground>` is neither an entity key nor a flat buffer offset. Matching item dimensions never establish any of those conversions.

Placement is optional and separate. To put a lattice into a game world, declare an ambient affine frame `F`, a point `o : Point<F>`, and a linear map `β : Λ_H → Vector<F>`. Then `χ(d) = o + β(κ_H(d))` gives a placement `χ : D_H → Point<F>`. Placement transforms values covariantly; it is not row lineage, does not reindex a field, and need not be injective or invertible. Changing `χ` leaves `f : D_H → V`, `D_H`, rank, and shape unchanged.

The return direction is always explicit. A registered locator has type `Point<F> ⇀ D_H` and proves its declared choice law; an interpolator returns a finite typed support in `D_H`; a frame map relates exact nominal frames; a registered lineage map relates exact row habitats. A `situated(H,F)` capability is the registry evidence naming the admitted placement and any admitted return bridges for that exact habitat-frame pair. No bridge is inferred from equal dimensions, equal shapes, equal cardinalities, a component called `pos`, or compatible physical buffers. Placement alone never manufactures a locator.

Surface support is another registered partial bridge, not an affine consequence. It consumes a `Point<F>` and one frozen world snapshot, then returns a typed hit only when that hit satisfies the declared incidence, ray, interval, normal, admissibility, and `Best` laws. `Best` uses a semantic cost and stable semantic tie key, never candidate enumeration or buffer row order; a policy without a unique certified choice refuses. The returned hit point is contact geometry, not necessarily an object's origin, so a prototype supplies a `RestingPose` or collider support offset before spawn placement.

The denotational signatures below are illustrative; registry and surface spelling remain open.

```text
pos             : P_Player → Point<World3>
groundAt        : Point<World3> ⇀ CellRef<Ground>
groundSample    : Point<World3> ⇀ weighted CellRef<Ground>
playerPlane     : Point<World3> × Vector<PlayerPlane2> → Point<World3>
supportBelow    : Snapshot × Point<World3> ⇀ SurfaceHit<CollisionWorld,World3>

Ground[4096] + MarsSurface[4096]       -- rejected: equal length is not lineage
Point<MarsLocal> + Vector<World3>       -- rejected: foreign frames
layout(Ground) as CellRef<Ground>       -- rejected: physical order is not a site reference
placement(Ground,World3)⁻¹              -- rejected unless a locator or affine equivalence was separately registered
```

A bare numeric shape, where retained by the surface, denotes a fresh anonymous lattice value. It never aliases a registered field merely because their buffers have equal length. Named stored fields are reached through their declared habitat. The registry spelling for habitats, frames, situated capabilities, placements, locators, interpolators, and support projectors is open; the semantic separation and authorization boundary are not.

### 20. Patterns from a coordinate lattice (outer product)

```apl
2 | ∘.+⍨ ⍳8       ⍝ 8×8 checkerboard
∘.=⍨ ⍳8           ⍝ diagonal relation
```

The coordinate columns are derived columns `axisⱼ : D → Fin(nⱼ)`. Every regular pattern is a predicate on those columns.

```haskell
Ground & (x + y) % 2 == 0 , Wheat = 1
Ground & x == y , Pillar = 1
```

Both effects target fields already declared on `Ground`. Spawning entities is different: the selected cell view supplies placement lineage, while allocation creates fresh entity keys.

### 21. Derived lines and computed placement

A finite line has habitat `Fin(n)` and its index is an ordinary column. A host callable may derive Fibonacci offsets or polar coordinates from it.

```haskell
12 , offset = fib(index)
   , spawn Cheese at Player.pos + (offset, 0)
Coin , pos = Player.pos + polar(index, index * 137.5)
```

The leading comma remains a new statement and barrier. The first statement materializes a derived line; the second consumes it. `prev` is a parallel shift on a declared order, not a recurrence.

### 22. Reduction and scan

A reduction maps a column on `X` to a scalar. A declared order licenses exact left accumulation with any compatible registered step. An unordered fold that may regroup that traversal requires associativity. A parallel/unordered fold that may discard traversal order requires associativity and commutativity. A registered identity supplies the empty result but is not required for nonempty input. `avg/` reduces the sufficient statistic `(sum, count)` and then finishes by division. A scan requires a declared order or axis, applies the same left-accumulator law, and returns a column on the same domain.

```haskell
+/ Elevation @ Ground
max/ Threat @ Frontier
scan2(+) Cost @ Ground
```

A fold does not alter the source field. Its result is a scalar or a separately declared derived value.

### 23. Grade

Grade returns an ordering witness for the current query domain. `top` may select a subdomain through that witness. A later write is legal only through the subdomain's retained lineage.

```haskell
top 8 (grade desc Safety @ Ground) , spawn Sentry
```

Ties require an explicit policy whenever their order can affect an effect.

### 24. Replicate and spawn

Replicate with per-row counts constructs the dependent sum `C = Σ(x : X). Fin(count(x))` and retains the source projection `C → X`. Spawn allocates one fresh entity key per member of `C`; placement and copied values gather through that projection.

```haskell
Ground , spawn Tree * Density
Ground & Fertility > 0 , spawn Crop * Fertility
```

A cell index is not an entity parent. A `parent` component may be filled from the source only when the source view carries entity lineage.

An exact spatial spawn evaluates against one frozen pre-state. “The player” must resolve to exactly one source row or the statement refuses; for that source, 51 copies form `Fin(51)`, while a general selected source `X` forms `Σ(x:X).Fin(51)` and means 51 per source. Each copy retains both source and copy index, derives its local phyllotaxis offset, crosses into `World3` only through the source's registered player-plane map, and asks the registered support projector for one certified hit.

Exact means all before any. Every required hit and `RestingPose` is validated before fresh allocation, then one injective allocation map supplies the entity keys shared by every spawned component effect. If all 51 validate, the commit creates exactly 51 fresh wheels for the unique player, writes only frame-correct supported positions, and preserves every fixed field. If one projection misses, one tie remains unresolved, or one resting pose is invalid, the statement returns the exact input world; there is no 37-wheel prefix. Geometric overlap between otherwise valid wheels is separate from effect-destination collision and is checked only when the declared policy requests clearance.

```haskell
Player , spawn CheeseWheel * 51 at supportBelow(playerPlane(Player.pos, phyllotaxis(index)))
```

This line is an explanatory reading, not a ruling on the eventual surface spelling of the frame map or support projector. The ruled content is unique-source resolution, copy lineage, frozen-snapshot projection, validate-before-allocate, collider-aware resting placement, and atomic success or refusal.

### 25. Fields, neighborhoods, and boundaries

A named field declares its habitat. A derived field is a pure column expression on one current query domain.

```haskell
def ridge = sin(x / 8) + sin(y / 8)
def basin = ridge < 0
Ground & ridge > 0.5 , Peak = 1
Ground & basin , Water = 100
```

A neighborhood is a relation `N ↪ D × D`, not a magic shift. A functional neighbor is a partial map; a general stencil remains a fiber view until a quantifier or fold consumes it. Boundary policy is structure, not syntax sugar: shrink is partial, constant extends the field, clamp and reflect are maps, and wrap requires modular or quotient structure. No boundary is the universal default.

### 26. Board literals and value reshape

A board literal creates an anonymous value habitat. Exact reshape requires equal cardinality and changes only its presentation; cycling or truncating reshape carries an explicit output-to-input gather map.

```haskell
"RNBQKBNRPPPPPPPP................................pppppppprnbqkbnr"
  to 8 8 , spawn (pieceOf char)
```

This sketch is legal only if `to 8 8` is defined as value reshape for the literal and the 64 output cells retain lineage to its 64 characters. It does not mutate any registered lattice's shape.

---

## Part V — Habitats, views, and columns

There are not two universal habitats. A world schema names many habitats: the live entity habitat, component-presence habitats, lattices, relation edges, event batches, history partitions, and derived query domains. What unifies them is the same column interface.

A query constructs a finite row domain `X`. Each consumed value is a `Col X V`, implemented as an aligned buffer and optional validity bitmap. Each stored source contributes a lineage map from `X` back to its own habitat. Selection restricts `X`; reindexing gathers along a map; a relationship contributes a span; grouping exposes its fibers; folds reduce fibers; effects scatter along an explicit destination map.

```haskell
Nord & TwoHanded > 60 , Gold += 1000
Ground & Water > 0 , Water -= 1
Plot , Moisture = avg/ neighbors'.Moisture
```

The first line gathers partial ECS components onto an entity query domain. The second updates one fixed field through a subdomain inclusion. The third reduces a neighborhood relation's fibers back onto the plot domain. All three lower to columnar kernels, but none may align columns by length alone.

Shape-changing reads are ordinary derived views. Stored writes are domain-preserving unless an operation explicitly creates a new stored object with a new habitat. Spawn and despawn change the live entity set, so a performed statement has type `World Σ → Result(World Σ × Output)`: the schema `Σ` is fixed while the finite entity habitat inside the world may change. Fixed field habitats named by `Σ` do not.

---

## A farm interlude

```haskell
Ground & (x + y) % 2 == 0 , spawn Wheat                      -- Ground supplies the lattice and placement
Pen , Headcount = #/ (livestock' & Cattle)                   -- count cattle per pen, over the inverse fiber
Cow & Weight < avg/ Weight @ Cow , +Marked                   -- below-average weight (scoped fold: one scalar, broadcast)
Cow , pos.x = rank(Milk) * spacing                           -- line the herd by yield (value-only rank)
Plot & !Planted & #/ (neighbors(wrap)' & Planted) >= 2 , +Planted  -- boundary is explicit
Plot , Moisture = avg/ neighbors(wrap)'.Moisture             -- per-plot mean over a nonempty wrapped fiber
Crop & Growth >= 100 , spawn Produce ; ~                     -- harvest the ripe
Farm & Acreage < avg/ Acreage @ Farm , Gold += 500           -- subsidy to small holdings
Gold , Gold = Gold * 1.05                                    -- 5% interest, world-wide
```

The two folds differ at the glyph. `@` after a fold is scoped-global, one scalar broadcast into the comparison. The tick is γ, a per-source column over each fiber. The explicit wrapped neighborhood gives every plot a nonempty fiber. A pen with no animals keeps `Headcount` at `#/`'s identity 0. Performed, the crops-spread line advances one ring per statement. Installed with the arrow (`def spread = … => +Planted`, §11), it advances one ring per tick.

---

## Technical Explanation

A staging language for entity-component systems. FP / APL lineage, ASCII surface. An embedded query-and-command engine with console ergonomics, in the SQL, Datalog, and production-rules class. Beside a Lua-class host the split is coroutines versus triggers. Sequencing and UI stay imperative. Conditions over the world with bulk effects on a schedule, missions included, are ano's territory (§11).

A script states predicates over registered columns and relations. The host resolves the finite query domain satisfying them. The predicate is the entity reference only when that domain carries entity lineage; the same calculus also ranges over fields, edges, events, and derived views.

What the model buys is determinism and replay. Compilation produces a pure plan from one snapshot: gather aligned columns, emit an effect buffer, then commit or refuse at the barrier. A performed statement has type `World Σ → Result(World Σ × Output)`. The same schema, world, input, and statement log give the same run.

Architecture. Scripts stage calls to host-registered functions, compile to a domain-and-lineage IR, lower column kernels to bytecode or a JIT target, and return an effect buffer describing the work. The host interprets the buffer. The staging and registration mechanics are eBPF-shaped, but totality comes from the grammar: no unbounded loop, no general recursion, registered callables, declared footprints.

The registry is ano's entire contact surface with the host. Every stored column declares a value carrier, a semantic habitat, a physical layout, mutability, and refinements. Every relationship declares its endpoint habitats and cardinality facts. Every frame declares its point and vector carriers. Every spatial bridge declares exact habitat and frame endpoints. Every effect declares a destination map and collision algebra.

- A mutable ECS component is a partial column on the entity habitat. A mutable field is a total column on one named field habitat. They share a column interface, not an index type.
- A position component carries `Point<F>` for one nominal frame `F`; its row habitat remains its component-presence habitat. The name `pos` and a three-number item shape confer no spatial authority.
- A registered lineage, frame map, locator, interpolator, situated capability, or support projector is an admitted bridge with exact endpoints and laws. An arbitrary function, placement, equal shape, and physical layout are not substitutes.
- A readonly column has an empty write footprint. Host mutation enters only at the tick's ingest boundary, so scripts still observe one snapshot.
- A callable function declares the habitats of its arguments and result together with its footprint. A callable may derive a column or dispatch an opaque value; neither is separate.
- A sigiled alias such as `^cursor` is re-resolved per evaluation. Sigiled lookup reads the live alias overlay first and falls through to the bare binding when no alias exists. Rebinding or deleting `^name` never changes bare `name`.
- An explicit binding such as `Player` or `Whiterun` has a declared denotation. A same-stem sigiled alias is a separate dynamic lookup and never changes that bare denotation.

The namespace is flat. Sentence position fixes syntactic use, while the registry fixes denotation and habitat. Provenance is tooling metadata, never a glyph.

The registry also carries accumulator entries and algebraic witnesses. A reducer supplies a carrier-compatible step; identity, finish, associativity, and commutativity are independent optional capabilities. Identity licenses a value for the empty fold, associativity licenses unordered regrouping over a fixed traversal, and associativity plus commutativity license parallel/unordered execution that may discard traversal order. Exact left accumulation on a declared order needs only the compatible step. A merge operation owes the law that makes collision fibers deterministic. A locator owes its choice law; a support projector owes candidate, `Best`, and refusal laws under a stable semantic tie policy. These are local capabilities of operations on carriers. Ground entries are trusted facts about storage and host functions; sky entries are laws the optimizer may use only with the required witness. `ano-sky.md` develops the witness question.

### Data model

A world schema `Σ` names habitats and columns. The current entity keys form a finite habitat `E`. A component `C` has a presence habitat `P_C`, an inclusion `P_C ↪ E`, and a dense value column `P_C → V_C`. A total field has a distinct habitat `H` and column `H → V`. A relation is represented by an edge habitat `R` with source and target maps, a span `A ← R → B`.

Physical storage remains columnar. Every finite semantic habitat has a layout bijection to `Fin(n)`, so a column lowers to a contiguous buffer and masks, maps, and edge endpoints lower to integer vectors. The semantic habitat and layout descriptor prevent two equal-length buffers from being confused.

### Query view

A query constructs a row habitat `X` and lineage maps to the stored habitats it touches. Every expression consumed by a kernel is aligned on `X`. Selection creates a subobject of `X`; scalar broadcast creates the constant column; a functional hop gathers along a partial map; a general relationship keeps an edge or fiber view until a quantifier or fold consumes it.

For one statement, let the source view have row domain `X`:

```text
X = source domain
p : X → Bool
S = {x ∈ X | p(x)}
i : S ↪ X
```

Equivalently, `S = p⁻¹({true})`, and `i` is the canonical subdomain inclusion. Here `p` is the final total predicate mask after presence, validity, and foundness guards; a failed partial read contributes false. Every predicate mask is a column on `X`, so pointwise `&` combines masks on that common domain and never short-circuits one conjunct through another. After the comma, an effect-side source column `c : X → V` is restricted to `c ∘ i : S → V`. Diagnostics follow the same domains: predicate-side `RELATION` and `FIBER` crossings range over `X`, effect-side crossings range over `S`, and distinct source crossings remain distinct trace events.

This is the missing bridge between relational selection and array execution. The compiler need not box rows or abandon structure-of-arrays storage. It passes an aligned buffer bundle plus masks and lineage vectors.

### Effects

An effect carries a target column, a destination map `d : S ⇀ H`, values aligned on `S`, and a merge policy. Plain assignment requires an injective destination map unless an explicit conflict resolver is registered. Additive, min, max, and similar effects may accept collisions only by reducing each destination fiber with a certified commutative merge. A masked field write restricts source columns along `i : S ↪ X` and scatters through the destination map.

Spawn is structural. Counts on `S` form `Σ(x : S). Fin(count(x))`; allocation maps that derived habitat to fresh entity keys. Despawn and spawn may change `E`, but not the schema or the habitat of a registered field.

### The relationship hop

A functional relationship component gives a partial map from the current entity view to a target entity habitat; `rel.Comp` is its indexed gather and dangling links invalidate the row. A set-valued relationship is a span. Its forward or inverse hop returns an edge/fiber view, not an arbitrarily flattened column. Boolean adjacency matrices are one physical representation of the same relation.

Array:
```bqn
mentor⊏twoHanded
```

Relational:
```sql
select e.* from Entity e join Entity m on e.mentor = m.id where m.twoHanded > 80
```

ECS:
```haskell
Nord & mentor.TwoHanded > 80 , Gold += 1000
```

### Lineage

| Layer | Tradition | Contribution |
|---|---|---|
| Surface flavor | Haskell, Erlang | Equational read, guard idiom |
| Front door | Cortex Roleplay console | Verb-legible command ergonomics |
| Address by description | Inform 7 | "now every closed door is open" — first-class intensional descriptions with bulk declarative effect, shipped in a game-authoring language |
| Selection | q/kdb+, SQL, Datalog | Target-by-description, join, sparseness |
| Standing rules | OPS5, CLIPS, Drools | Condition-action over working memory is precisely predicate-comma-effect; Rete is the known answer to incremental re-evaluation |
| Campaign logic | StarCraft 2 trigger editor | Events-conditions-actions over live game state scripted whole campaigns; the なる register is that layer with a relational predicate language |
| Action | APL, q/kdb+ | Masked column arithmetic, the array calculus |
| Spatial rules | PuzzleScript | Pattern-rewrite over a declared grid habitat |
| Effects | ECS command buffers (Flecs, Bevy) | The deferred effect buffer, committed at a sync point |

### The maths

The selection sublanguage is relational algebra with grouping: selection (σ) by predicate, join (⋈) by relationship, projection (π) by component access, and grouped aggregation (γ) over relationship fibers. The column sublanguage is array calculus over a typed finite row domain: reindex, reduce, scan, grade, outer product, replicate, and reshape. The common substrate is not an identity between sets of rows and arrays of bytes; it is a bundle of aligned buffers, validity masks, layout descriptors, and lineage maps. The comprehension correspondence identifies a filtered product with a θ-join. The combinator core is structural recursion over finite columns, so totality follows from the grammar. A compiled statement is a pure plan; performance is `World Σ → Result(World Σ × Output)`. Effects inside one barrier compose only under their certified collision laws, while statements compose sequentially at barriers. The formal obligations live in `proofs/foundations.md`.

---
# Habitats and capabilities


The Pious Hierarchy here is Mathematics > Denotation > domain-and-lineage IR > Grammar > Surface > Lowering. Syntax may infer evidence already present in the registry; it may not invent it from length, rank, or spelling.

## Habitat capabilities

A habitat begins as a finite index type. Operations demand only the additional structure they use.

| Capability | Witness | Licenses |
|---|---|---|
| finite | layout `Fin(n) ≅ H` | dense storage, masks, parallel map |
| ordered | total order or axis order | shift, scan, stable grade |
| product | `H ≅ A × B` | axes, transpose, rank-aware reshape |
| lattice | chart into a free integer module | integer offsets, stencils |
| affine placement | `o` and `β : Λ → T` | world coordinates and spawn placement |
| metric | distance law | radius and nearest queries |
| cell complex | cells and boundary maps | incidence, contour, topology |
| relation | span `A ← R → B` | hops, inverse fibers, grouped folds |
| carrier algebra | operation and laws | reduce, scan, collision merge |
| host dispatch | registered signature and footprint | operations absent from the native kernel set |

These capabilities compose. `Ground` may be a 2D lattice with affine placement and a metric. An inventory may be an ordered entity relation with no geometry. An adjacency matrix may be a boolean field on `Node × Node` and also present a relation. The carrier and its habitat remain distinct.

## Domain preservation

A stored column has a declared habitat `H`. An ordinary update returns another value in `H → V`; it may change values but not `H`. A masked update restricts to `X ↪ H` and scatters through that inclusion. A derived operation may return `Y → W` for another domain `Y`, but writing that result into a column on `H` requires an explicit destination map `Y ⇀ H` satisfying the target effect's collision law.

A lattice chart is regenerable; field state is not. Recreating `↕shape` recreates coordinates, not the correspondence between persistent values and their cells. Therefore filter, fold, grade, replicate, and reshape are free as reads into derived domains, while write-back is lawful only through retained lineage. Pressing `n`, ticking twice, or spawning entities cannot change a registered field's rank.

## Symmetry is local evidence

Permutation equivariance is useful for an operation that claims to ignore labels and order. It is not a law of all entity programs. Ano exposes unique keys, `index`, explicit bindings, relationships, order witnesses, and structural effects, so the full symmetric group does not act invisibly on every query. A compiler rewrite may use equivariance only when the operation's signature and input view certify it.

Stable grade, a fixed relationship, or a declared order deliberately breaks maximal symmetry. That is additional structure, not a type error. Ties remain explicit wherever their resolution can affect state.

## Opaque carriers

A host value such as a behavior tree, navmesh, or engine handle may expose no native Ano algebra. It is still a column on a declared habitat and may still be selected, reindexed, stored readonly, or passed to a registered callable according to its signature. Host implementation is an execution capability, not a third habitat.

Naturality in the carrier characterizes genuinely parametric column maps: a family that works uniformly for every `V` is a reindexing. It does not prove that every operation on an opaque carrier is parametric. The registry says which operations exist and what they may inspect.

```haskell
Node , OutDeg = +/ Adj@row
Hostile , shortestPath via Navmesh
^cursor , runBehaviorTree
```

The first is a native boolean/numeric fold. The others are registered dispatch. All arguments and results still carry habitats and footprints.

## Carrier refinements and laws

A carrier may accumulate compatible evidence: `bool`, `nat`, `int`, `range`, `unique`, `ordered`, `monoid(op,id)`, quantized. Evidence has set-intersection semantics, never inheritance. `+/` demands the appropriate monoid; `max/` is a semigroup and refuses an empty fiber unless an identity is registered; an unordered parallel fold demands commutativity. An effect merge owes the same laws over each collision fiber.

Carrier refinements are orthogonal to habitats. `bool` admits `{0,1}`, `nat` the naturals through 2^53, and `int` their signed twin; `range <col> <lo> <hi>` adds bounds. The loader checks data at rest, the barrier retracts committed writes to the carrier, and Kore repairs interactive edits. These are carrier facts orthogonal to habitats.


That's Ano.

---

## Appendix — Grammar

### Precedence

Fourteen levels, loosest to tightest. Everything else in the document is a consequence of this table.

- 1 (loosest) — `,` and `=>`: the hinge; everything left is selection, everything right is effect (`,` performs, `=>` installs). The comma inside a parenthesized presence tuple `(Nord, !TwoHanded)` is bracketed and does not compete.
- 2 — `;`: effect batching within the statement's one barrier.
- 3 — `|>`: pipeline stages (`order by`, `take`, `expand`).
- 4 — effect verbs and assignment: `= += -= *= /=` (`=` assigns only in effect position; the equals glyph, below), `+Comp -Comp ~ spawn`, the locatives `at` and `to`, replicate `*` in `spawn X * n`.
- 5 — `|`: Greater, OR on masks and maximum on numbers.
- 6 — `&`: Lesser, AND on masks and minimum on numbers.
- 7 — `!`: mask not, prefix on one mask term.
- 8 — comparison: `== != < <= > >=`, and `=` in selection position (the equals glyph, below).
- 9 — fold and scan prefixes: `f/ f\` with f an operator or a registered reducer name, `fold(f)`, `scan(f) … along`, `grade`, `top k`.
- 10 — additive arithmetic: `+ -`.
- 11 — multiplicative arithmetic: `* / %`.
- 12 — `@` scope: locative on a mask (`Cheese @ cellar`) and fold scope (`Gold @ Nord`), one meaning: evaluate within this scope. A placed spatial callable may take `mask @ frame(args) at origin`; `at` supplies the origin of that declared placement, never an ambient frame inferred by `@`.
- 13 — hops: the dot `.` and the set-hop tick `'`, the gather, the tightest operator.
- 14 (tightest) — atoms: names, the `^alias` sigil (lexical, part of the identifier), colon symbols (`:Sym`), counter-typed numerals (`3mo`), parens and comprehension brackets.

Resolution, worked: `Cheese @ cellar & Aged > 3mo` parses as `(Cheese @ cellar) & (Aged > 3mo)`, since `@` (12) binds its scope before `&` (6), and `>` (8) binds before `&`. `Cow & Weight < avg/ Weight @ Cow` parses as `Cow & (Weight < (avg/ (Weight @ Cow)))`, since the fold prefix (9) outbinds the comparison (8).

### Fold and scan permutations

One table governs the whole level-9 family. Ano's `|/` and `&/` are q's folds. The operand carrier selects Greater or Lesser.

| f | `f/` fold | `f\` scan | empty-scope identity |
|---|---|---|---|
| `+` | sum | running sum | 0 |
| `*` | product | running product | 1 |
| `&` | ALL on masks, minimum on numbers | still-all on masks, running minimum on numbers | mask true, number none → row drops |
| `\|` | ANY on masks, maximum on numbers | ever-any on masks, running maximum on numbers | mask false, number none → row drops |
| `#` | count | running count | 0 |
| `max` | numeric bridge for `\|/` | numeric bridge for `\|\` | none → row drops |
| `min` | numeric bridge for `&/` | numeric bridge for `&\` | none → row drops |
| `avg` | fold-and-finish mean | running mean | none → row drops |
| `-` | ordered left subtraction; unordered refused | running subtraction | none → row drops |
| `/` (divide) | `fold(/)` is ordered left division; unordered refused; `//` remains unlexable | `scan(/)` is running division | none → row drops |

The identity column restates the §12/§13 law. A fold with an identity yields it on the empty scope. A fold without one fails the row, so an identityless scoped-global fold is an empty result and a bare query prints nothing. The γ column-form (`f/ rel'.Comp`) inherits the carrier-specific law per fiber. A named reducer (`threat/`) uses its registered identity or fails the empty scope. Its scan (`threat\`) is length-preserving, so empty input yields an empty column without consulting an identity. Steel's query emitter still substitutes `0` after a guarded identityless global fold. Removing that placeholder is pending in `todo/18-empty-result-output.md`.

Ano adopts q's operations directly over its admitted carriers. `|` is OR on masks and maximum on numbers. `&` is AND on masks and minimum on numbers. Their folds and scans follow from the same dyads. Boolean OR stays `|`. There is no `||`. `max/`, `max\`, `min/`, and `min\` remain numeric bridges. `>` remains a comparison, so `>/` stays rejected.

Steel does not yet implement the carrier overload. It emits `|` and `&` only as mask operations. Numeric `|`, numeric `&`, numeric `|/`, numeric `&/`, numeric `|\`, and numeric `&\` are pending in `todo/17-greater-lesser.md`. The bridge `max\` is live. `min\`, running mean, and running count remain pending in `todo/12-unbuilt-scans.md`. The γ column-form takes operator folds. A named reducer over fibers (`threat/ livestock'.Weight`) is still refused.
The long-form head in `fold(f)`, `scan(f)`, and `scan2(f)` follows the LINQ accumulator model: it may be an operator or a registered reducer name, and the registry resolves the step, identity, finish, and laws. This is one callable-head policy, not a special case for `fold(+)`. A declared order admits any compatible step under exact left accumulation; unordered regrouping requires associativity; parallel/unordered execution that may discard traversal order requires associativity and commutativity. Steel's `fold(f)` parser still accepts names only, and the emitters still maintain incompatible operation tables. Long-form parity and ordered subtraction/division are pending in `todo/12-unbuilt-scans.md`.

### Desugarings

Every surface form is the one form `source & predicate , effect`. Sugar only elides or supplies a slot.

```haskell
Nord , Gold += 100                         -- world & Nord , Gold += 100: source elided, the live world
Merchant @ Whiterun , Gold += 5000         -- world & (Merchant @ Whiterun) , …: the locative is part of the predicate
^cursor , Health = 0                       -- world & ^cursor , …: an alias is a named predicate with a unique resolve
spawn Wheat                                -- ^cursor , spawn Wheat: subject elided, ゼロが supplies the antecedent
~                                          -- saved mask , ~: continuation, a new statement and barrier over the antecedent's saved mask (§10)
, spawn Cheese at Player.pos + (offset, 0) -- saved mask , effect: the leading-comma continuation, same rule spelled with an effect (§10)
def master = Human & Nord                  -- names a predicate; folds into any selection slot at plan time (§6)
Enemy |> order by Threat desc |> take 5 , +Targeted   -- the pipeline builds an ordered, truncated source view; the effect half is unchanged
top 5 (grade desc Threat) , +Targeted      -- the same view, fold-prefix spelling (§15)
[ t & c , +InRange | t <- Tower, c <- Creep, dist(t, c) < 50 ]   -- the comprehension's generators and filter are the source: σ_p(Tower × Creep) , +InRange (§16)
Spawner |> expand Count , spawn Minion     -- replicate: each source row emits Count effect rows (§17)
12 , offset = fib(index)                   -- a numeric shape in source position is the generator (↕12), predicate empty (§21)
8 8 & x == y , Pillar = 1                  -- a fresh anonymous rank-2 value; it aliases no stored field (§20)
"…" to 8 8 , spawn (pieceOf char)          -- reshape pours the literal into a lattice source (§26)
Oil @ blast(5) at Firebolt.pos , +Fire     -- blast declares a placed habitat; `at` supplies its origin (Part IV)
eval "Nord , Gold += 100"                  -- the quotation splice: a literal one-statement string, re-lexed and parsed in place at compile time (Open Questions, Staging)
def spread = Plot & p => +Planted          -- the naru hinge: same form, installed instead of performed, re-gathered once per tick (§11)
```

### The enum sigil

The colon is the atom literal: `:Bandit` is the enum value, `Bandit` the component mask. The rule is lexical, never a registry lookup. A bare name in a value position denotes a column (component or derived). A colon name is a symbol atom. `Col == :Sym` is a pointwise mask against the constant. `Col == Col2` is a pointwise column comparison. A bare name that resolves to no registered column is a compile error, never a silent symbol. The lineage is the atom of Erlang and Ruby and the Lisp keyword.

```haskell
Bandit & !Dead & Faction == :Bandit , Faction = :Hostile
```

### The equals glyph

One glyph, position decides, the SQL rule. Left of the hinge (`,` or `=>`) `=` is comparison, identical to `==`. Right of it, assignment. The first `=` after `def name` is definitional, and a def body is selection position, so any further `=` inside it compares. The readings never collide: assignment cannot parse in a predicate, and a comparison nested inside an effect's right-hand expression is spelled `==`. `==` stays legal everywhere.

```haskell
NPC & Tunic = :Red & Faction = Player.Faction , Gold = 0   -- two comparisons, one assignment
```

### Dot and `@`

Dot is gather under declared structure. A functional relationship hop `rel.Comp` gathers through a partial entity map. A mirror-read such as `Player.pos` is the arity-one case. `pos.x` projects a registered compound value without changing habitat. `prev` requires an order witness; a lattice neighbor requires a lattice chart and explicit boundary policy. A set hop such as `neighbors(wrap)'.Moisture` retains the fan-out edge habitat until the tick's fiber fold consumes it. Dot never establishes alignment from length and never groups by itself.

`@` means evaluate within a declared scope. On a selection it restricts the current query view; after a fold or scan it supplies the scoped input and does not group. A scope does not create an affine frame. The form `mask @ frame(args) at origin` is meaningful only when `frame` is registered as a constructor of a placed habitat with a declared ambient space and linear part; `at` then supplies its origin. `+/ Gold @ Nord` and `+/ Elevation @ Ground` are scoped-global folds, one scalar each. Grouping is the tick.

The alias sigil `^` is not an operator: `^` glued to a name is one identifier (`^cursor`, `^observer`, `^world`), the こそあど deixis, re-resolved per evaluation. Sigiled lookup reads the dynamic alias overlay first and falls through to the bare binding when no alias exists. `^` appears nowhere else in the grammar, so `^name` never needs disambiguating.

### `!` and `^`

| sigil | is | precedence | binds to | resolved |
|---|---|---|---|---|
| `!` | mask NOT, a prefix operator | level 7 | one mask term | at evaluation, pointwise |
| `^` | not an operator: lexically part of the identifier; the alias/deixis sigil (`^cursor`, `^observer`, `^world`), こそあど | atom, level 14 | the name it is glued to | live alias first, then the bare binding; re-resolved per evaluation |

They never compete: `!` negates a mask, `^` names a dynamic alias. `Whiterun` is the bare registered column or binding, `!Whiterun` negates its mask, and `^Whiterun` reads the live alias of that name. Without a live alias, `^Whiterun` falls through to bare `Whiterun`. With one, sigiled lookup shadows the bare binding without replacing it. Deleting the alias restores the fallback. `!^cursor` is "not the thing under the cursor": the sigils compose, they do not overlap. Implementation is pending in `todo/16-dynamic-alias-overlay.md`.

## Open Questions, Next Steps

```markdown
/\/\/\/\/\/\/\/\
```

- Explicit bindings, the denotation. The registry binds a bare proper noun as a constant, and the surface reads identically whether that constant is an entity key (`Player`), a pre-baked selection (`Whiterun`), or an archetype. The options are not exclusive. A binding could declare its denotation at registration and resolve by context: the mirror-read `Player.pos` demands a unique entity, the scope `@ Whiterun` demands a region, `at rally` demands a point. Whether one name may carry several faces or must declare exactly one is open. Context dispatch fits the language's inference stance. One face per name is easier to check and easier to read.

- Recurrences. `offset = prev.offset + prev.prev.offset` is not a recurrence under the one evaluation rule. The comma is gather-effect-scatter and every read observes pre-state, so `prev` is a parallel shift and the statement is one stencil step. No statement can carry a value along the line it is writing, and iterating it gives k stencil steps, never the order-carried sequence. A true recurrence is a scan whose step need not be associative, and §14's scan is a read-side column expression over a declared order. A scan that feeds its own column's scatter breaks the barrier by construction, so scans cannot live behind the barrier. Options, each with a cost. Keep recurrences host-side as registered functions (`fib(index)`, the current canonical form): the host runs the recursion and hands back a column, which a statement may key positions off in the same breath. Or admit a sequential `scan(f) along order` with non-associative f, legal only where its write footprint does not intersect its read footprint. Or admit a generator subclause: corecursion consumed under bounded demand, the lazy-list/Python-generator shape, total because the take is finite even when the definition is not, and read-side by construction so it never touches the barrier. A fixpoint/iterate form stays rejected, since totality comes from bounded demand, not a general fixpoint. One boundary note. Under a fixed-tickrate host the game loop is itself the scan, state[t+1] = F(state[t]), one barrier per tick. So recurrences across ticks are already expressible, and a stage counter advanced by standing rules is one running. Only the within-statement form is open.

- Identity. The intensional/extensional boundary: how does a script say "the same bandit as last tick"? A predicate re-resolves per evaluation, so the surface is intensional at the statement level. Yet relationship components already store entity IDs, extensional handles, so the data level is extensional, and the surface cannot reach what the data already holds. The saved-mask continuation rule gives one statement of extension inside a script. Nothing spans ticks. The options pull against each other. A surface form that holds a resolved selection across ticks is a handle, which breaks the predicate-is-the-reference stance. A component that freezes the match (`+Marked`) keeps the stance but makes identity state the script must manage and retract. A third option rides the clock. A host with tick-stamped history (Anoptic plans a monotonic tick counter at a fixed rate) makes "the same bandit as last tick" an as-of join against the t−1 partition, q's `aj`, an extensional read recovered through the time axis, with no handle on the surface. Identity becomes an indexing question. Where the boundary between intensional statements and extensional data sits is the design. Unresolved.

- Rule retraction. A standing rule is named (`def spread = … => …`) so it can be withdrawn, but the retraction form is unspecified: an `undef spread`, a scope that expires (`=> … @ scene`), or a component guard the rule itself reads. A handle-shaped verb sits awkwardly beside predicate-is-the-reference. A guard makes rule state the script must manage and retract. Campaign logic makes this entry load-bearing: a mission stage is a standing rule that must withdraw itself on advance (§11). Unresolved.

- Staging and quotation. Nothing in the current design is homoiconic: there is no quote form and no macro, and "registry" is Lua vocabulary, not Lisp. The nihongo koto nominalizer, the clause-as-thing form, points at where it would enter. `eval "…"` is ⍎ constrained to a literal one-statement quotation, spliced at parse time, reflection priced as program text, the footprint still visible to the §11 conflict check. A runtime string has an opaque footprint and waits on the interpreter. Templates with holes, the koto form, are where homoiconicity would be earned. Open past the literal splice.

- Outer product as a value. The double-generator comprehension covers the filtered-pairs case. Whether ano lets a script hold a materialized N×M matrix (`cross f A B`) as a first-class value is open.

- Habitat and spatial spelling. The denotation: a registered field has one nominal habitat; equal shape never aligns foreign habitats; a bare shape creates an anonymous derived habitat; selection retains lineage; exact reshape is an equivalence; cycling or truncation is a gather map; `Point<F>` and `Vector<F>` carry nominal frames; placement is a covariant point map and never lineage or localization; locators, interpolators, frame maps, situated capabilities, and support projectors are separately registered partial bridges with exact endpoints and laws; an exact spatial spawn validates every proposal before allocation and refuses atomically. The remaining surface questions are how habitats, layouts, frames, placements, locators, interpolators, projectors, boundaries, and exact spawn policy are declared, whether first-class value reshape uses `to` or `⥊`, and whether the allative placement `to` deserves a distinct spelling. Unresolved.

- The Sky Registry, the witness obligation. A ground entry is a trusted storage or host fact and a sky entry is a law the compiler may rewrite by; their failure modes are not symmetric. Candidate rungs remain trusted, property-tested against generated Steel/Kore worlds, and machine-checked against the obligations in `proofs/foundations.md`. BQN twins may remain explanatory witnesses but are neither semantic nor differential oracles. Whether the rung varies per law kind remains open. `ano-sky.md` carries the development. Unresolved.

- The machine as a world. The hypothesis closing `ano-sky.md`: an x64 machine is columnar data plus a step function, a code column, a register column, memory, which is the standard formalization (Sail, the K framework, ACL2). An out-of-order core already runs the evaluation model: rename is the pre-state gather, the store buffer is the effect buffer, retirement is the barrier. Registering the machine itself as a world would make compilation an ano query, gather λ-terms and scatter instructions, the first Futamura projection with the evaluator written in ano. What is provable is dependence and layout, since barrier semantics makes intra-statement aliasing statically absent. What is not is latency, data-dependent and the Itanium lesson. Literal self-modifying code stays dead in the pipeline, so JIT synthesis into fresh columns is the usable form. First falsifiable step: the Haskell embedding (`ano-sky.md`, the prototype path). A hypothesis, not a roadmap. Unresolved.

- Bootstrap host: Rust. Steel is the reference compiler and Kore is the interactive world. CBQN is Steel's backend and BQN is not a semantic authority. The bytecode VM and JIT target stay fixed. A future embedded implementation derives from verified Steel semantics and the domain-and-lineage IR, never the inverse.

- Reap ownership and granularity. Generational tombstoning: the gen bump plus presence clear is the mark, free-list reuse at tick seal is the deferred reap, and the mask-level meaning of `~` never changes — only storage reclamation is policy, exposed as the registry option `reap seal|host`. The remaining knob is the option's grain: world-wide, per archetype, or per column. Unresolved.



The name あの is the distal demonstrative ("that one over there").
