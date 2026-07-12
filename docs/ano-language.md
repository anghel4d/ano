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

Ano is to a game world what q is to kdb+: the resident query-and-command language of a live column store. The FP/array sibling of Lua. The split with a Lua-class host is coroutines versus triggers. Cinematic sequencing, UI, and per-instance branching stay imperative. Everything statable as a condition over the world, a bulk effect, and a schedule is ano's territory. StarCraft 2's trigger editor shipped a whole campaign on that paradigm. Ano is that layer with a relational predicate language. The shape buys determinism and replay. A statement is a pure function of world state: a predicate plus an effect buffer over pre-state. The same world and the same log give the same run. A mission is a text file that replays identically anywhere. Let's dive in.

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

`^cursor` is a pronoun, not a name. `Player` is a proper noun: the referent was chosen once, at registration, and never moves. `^cursor` is the word "you". Who it refers to is decided at the moment of speaking, by the engine, not the script. The registry supplies a resolver (a raycast from the mouse, the camera's focus), and every gather re-runs it. So `^cursor , Health = 0` kills whatever is under the mouse at that gather, and two statements mentioning `^cursor` may hit two different entities. That is why it carries a glyph in a language with no other sigils: the reader must know this name can move between statements. In C# it is an expression-bodied property, never a field. `Entity Cursor => Physics.Raycast(mouse)` re-raycasts on every read, like `DateTime.Now` against a stored timestamp. In Haskell it is `asks cursor` in a Reader: the script is a function of an environment the engine rebuilds each tick. Pointedly not an `IORef`, since nothing can store or write it. In filesystem terms `Player` is `/home/pyrus` and `^cursor` is `./`, the same spelling landing somewhere different depending on where you stand. Against a component name the difference is arity. `Nord` is a mask over many rows. `^cursor` resolves to a referent, usable anywhere a selection or a mirror-read root goes (`^cursor.pos`).

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

A relationship is a component whose value is another entity's ID. `rel.Comp` reads `rel` to get a target ID per entity, then gathers `Comp` at those IDs. One hop is one indexed read. An absent or dangling link fails the predicate, the left-join-null behavior.

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

The keyed hop (ruled 2026-07-11). A `unique` column is declared injectivity — pairwise-distinct, checked at load — and injectivity is precisely the license a hop needs, since an injective column is invertible on its image: the keyed hop is `rel ; unique⁻¹`, one index-of against the key column with one found-guard. A relationship declares its key column by writing it before the name (`rel id mentor -1 0 0 3 …`, reading type-annotation-first: mentor is a rel over id), and an undeclared rel stays keyed to the row index — zero churn for every existing world. Not-found is dangling is dead: a stored ID whose target despawned fails the found-guard and clears the mask bit, the same left-join-null, so staleness never grows a fault path (the storage mechanism is ano-ecs §2's generational compare; the diagnostic surface, never the semantics, is the debug layer's dead-link report). The inverse of a keyed rel is keyed automatically, and `unique` subsumes the id/keys name magic: it declares what the emitter used to guess. `unique` means exactly one thing (ruled 2026-07-11): every element distinct, enforced at load and held across the run. Minting is spawn machinery, never a `unique` semantic, so several unique columns never compete for a mint; spawn fills each one fresh because the declared injectivity admits nothing else.

The undeclared world keeps its meaning after rows shift (the surfaced no-id choice, taken 2026-07-11). The fixture row identity is itself a key space: materialized once at fixture time as a hidden column, thinned by despawns and carried forward by spawns, so an idx-keyed read after a `~` resolves through it — the same algebra, not a second semantics. The alternative, refusing structural effects in worlds with no unique column, was considered and set aside: left-join-null already gives those programs meaning. The anoc perimeter around `unique` — spawn mints 1+max, `default` and proto fields on a key column refuse at load, effect assignment to one is a compile error — is surfaced implementation decision, not ruling; the ruled content of `unique` is distinctness alone.

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

Spawn fill is a three-layer lookup (ruled 2026-07-11): the proto's declared value, else the column's registered `default`, else the type's zero — num 0, bool 0, sym "", rel -1. A -1 rel is None under §5's left-join-null; kore renders it `/`, a rendering choice, never a language feature — if a surface literal ever lands it is `none` or `null`. The proto is the registered archetype, the missing noun: `def Marine soldier=1 hp=100` declares one in the registry, its first row-oriented named-value construct, and `spawn Marine` fills through it. A ghost is one bit plus the defaults; the proto is what knows Ghost.

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

The hinge changes, nothing else. `selection , effect` performs a command now. `selection => effect` installs a standing rule. Same left side, same right side, same `;` batching. The comma is する, the performed voice. The arrow is なる, the becoming voice. The する/なる split §9 declares is carried by the hinge, and the hinge is the visible が only in the なる register (ano_nihongo.md, the する case frame). The lineage is the Datalog rule (`head :- body`, with ano's order head-final) and the production rule of OPS5 and CLIPS. A rule is named for retraction the way selections are named.

```haskell
Plot & !Planted & #/ (neighbors' & Planted) >= 2 , +Planted                  -- performed once, now
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

A column expression is a vector-over-a-selection. `Gold` is one. These operators build others, and a column expression appears anywhere a component name appears: in a predicate, a fold, an effect, an ordering.

### 12. Reduction (`/`)

```apl
+/ 1 2 3 4        ⍝ 10        sum
×/ 1 2 3 4        ⍝ 24        product
⌈/ 3 1 4 1 5      ⍝ 5         max
∧/ alive          ⍝ all
```

Collapse a column to a scalar. The contract for a raw `fold/` is strict: an associative binary operator, with a registered identity if the empty scope is to mean anything. `@` scopes the fold to a selection, and a fold under `@` is always one scalar.

```haskell
+/ Gold @ Nord              -- total Nord gold
*/ (1 - Resist) @ Hits      -- combined damage multiplier
&/ Alive @ Party            -- whole party alive
|/ Burning @ Forest         -- any tile burning
#/ (Nord & TwoHanded > 60)  -- count of masters
```

Not every collapsing form is a raw reduction. The pairwise mean is not associative, and `#` is not a binary operator, so `avg/` and `#/` are derived fold-and-finish forms. `avg/` folds sum and count in one pass and divides at the end. `#/` is `+/` over the constant 1. The surface keeps the spellings, and the registry records them as fold-and-finish. That is what makes the empty case honest. A fold with an identity yields it (`+/` and `#/` give 0, `|/` false, `&/` true). A reducer with no identity over finite component values (`avg/`, `max/`, `min/`) fails the empty scope and the row drops, the left-join-null rule again.

or, spelled for named reducers — the slash attaches to a registered reducer name exactly as it attaches to an operator, one grammar row, and `fold(f)` is the long form. Scans come free: `threat\`.

```haskell
threat/ Damage @ Enemies
fold(threat) Damage @ Enemies
```

### 13. Grouped fold (γ)

```q
select headcount: count i by pen from animal where cattle   / q: by groups, the aggregate collapses each group
```

A fold prefix over a tick-marked hop is the grouped fold: `fold/ rel'.Comp` for a gathered column, `fold/ (rel' & pred)` for a filtered fiber. A relationship is registered set-valued forward (`targets`, `neighbors`: each source maps to a set of targets) or as the inverse read of a functional relationship (`livestock`, the inverse of `pen : Animal -> Pen`). A key-valued column stands in relation position the same way. It has a functional relationship's exact shape, so `Col'` denotes its inverse fibers over the stable-id column: the value-level rel, a relation computed by the program instead of registered (the wand demos' w3-c). For each entity in the current selection, `rel'` denotes the fiber at it, the target set for a forward relationship or the preimage for an inverse read. The fold collapses each fiber to one value per selected entity. The whole expression is a column aligned to the selection, written back under the ordinary alignment rule. This is γ: q's `by`, Datalog's grouped aggregation, expressed as fold-under-each over the fibers rather than a new clause.

```haskell
Pen , Headcount = #/ (livestock' & Cattle)    -- count per pen, over the inverse fiber
Plot , Moisture = avg/ neighbors'.Moisture    -- per-plot mean over the neighbor fiber
Target , Hits += #/ attackers'                -- in-degree, folded at the target
```

`fold/ col @ scope` remains the scoped-global fold and is always one scalar. `fold/ rel'…` is always per-source. The result-type split is lexical, never a registry lookup. After a fold, `@` yields one scalar, `'` yields a per-source column, and `@` never groups. Empty fiber: the fold's registered identity when it has one (`#/` and `+/` give 0, `|/` false, `&/` true). A reducer with no identity (`avg/`, `max/`, `min/`) fails the row. This is the §5 left-join-null rule extended from the dangling link to the empty fiber: the entity drops out of the selection and no write lands.

### 14. Scan (`\`)

```apl
+\ 1 2 3 4        ⍝ 1 3 6 10    running sum
⌈\ 3 1 4 1 5      ⍝ 3 3 4 4 5   running max
```

Accumulate a column along an ordered selection. Returns a column of equal length. Unordered selections need `along`.

```haskell
+\ Weight @ (til steps |> route A B) -- cumulative movement cost along a route
*\ Multiplier @ comboChain           -- running combo multiplier
max\ Height @ (Eye + til n * fwd)    -- running peak along a sightline (occlusion)
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

Ties are stable in the grade, which is an ordering. A rank written back into a component is value-only, dense or fractional, because stable ties break by index, and leaking index information into a record is exactly what the Tier-2 write law forbids.

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

Pour a flat selection into a spatial arrangement. Generation makes keys. Reshape repositions existing values. The surface spells it `to`, the allative.

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

Space is the same calculus with the raggedness removed. A lattice is dense and rectangular, so every operator above applies, and most are more natural here. There is no spatial keyword. A numeric shape in the source slot is the generator, `w h` carrying `x y` per cell. Acting on cells that already exist is a predicate on the position column. Generating cells that hold nothing is the shape, the one job a predicate cannot do, since a filter cannot invent a key. A named region is a scope (`@ Frontier`). A path or sightline is an ordered index line (`til` plus arithmetic).

Space may denote infinity. A statement demands a finite window or a symbolic field clipped by `@scope`. Coordinates enter world-space by the affine column `pos = φ(k) = o + S·k`. The `@` scope fixes the frame `(o, S)`. The origin defaults to the cursor's raycast, and the こそあど deixis names it: `^cursor` proximal, `^world` distal. The anchored form `mask @ frame(args) at origin` spells the frame in full. The same locative `at` that places a spawn fills o, with a mirror-read (`at Firebolt.pos`) or a bound point (`at impact`). The registered frame fn is the predicate, run per cell as `Fn ⟨cell, origin, args⟩` (demos 11-noita n5/n6). The counter check is the unit consistency of φ.

### 20. Patterns from the coordinate lattice (outer product)

```apl
2 | ∘.+⍨ ⍳8       ⍝ 8×8 checkerboard (parity of coordinate sums)
∘.=⍨ ⍳8           ⍝ the diagonal
(⍳8) ∘.≤ ⍳8       ⍝ upper-triangular mask
```

The lattice is `⍳` crossed with `⍳`. Every regular pattern is a predicate on the coordinate columns. Change the operator, change the pattern.

```haskell
8 8 & (x + y) % 2 == 0 , spawn Wheat     -- checkerboard
8 8 & (x + y) % 2 == 1 , spawn Barley
8 8 & x == y , spawn Pillar              -- diagonal
8 8 & x + y < 8 , +Buildable             -- triangle
8 8 & x % 3 == 0 , spawn Fence           -- stripes
```

### 21. Generation along a computed lattice (Fibonacci, spiral)

```apl
+\ fib            ⍝ cumulative fibonacci offsets
n × 137.5         ⍝ golden-angle per index
```

A line of `n` is the bare shape `n`. Its index becomes a position under a coordinate transform, and `Player.pos +` lifts into world-space. A computed sequence like Fibonacci comes from a registered host function over the index. The recurrence runs inside `fib`, outside the calculus.

```haskell
12 , offset = fib(index)                                      -- one statement, one barrier
   , spawn Cheese at Player.pos + (offset, 0)                 -- continuation: next barrier over the saved line, sees the committed offset
12 , spawn Cheese at Player.pos + polar(index, index * 137.5) -- phyllotaxis spiral
8  , spawn Pillar at Player.pos + (index * 2, 0)              -- evenly spaced row
```

The second cheese line is a leading-comma continuation under §10: a new statement and a new barrier over the line's saved mask, whose gather observes the committed `offset`. It cannot share the first line's barrier, since it reads what that line writes.

The tempting spelling `offset = prev.offset + prev.prev.offset` is not a recurrence. The comma is gather-effect-scatter and every read observes pre-state, so `prev` is a shift, not a carry. The statement is one parallel stencil step `new[i] = old[i-1] + old[i-2]` over the pre-state column, undefined-or-zero on a freshly minted line, so it cannot generate Fibonacci. Iterating it gives k stencil steps, never the order-carried sequence. `fib(index)` is the honest form. Whether a true recurrence should ever be admitted is open (Open Questions, Recurrences).

or, assign the computed positions onto an existing set:

```haskell
Coin , pos = Player.pos + polar(index, index * 137.5)   -- spiral, assigned from the iota
```

### 22. Reduction and scan over space

```apl
+/ , elevation    ⍝ total elevation (ravel then sum)
⌈/ , terrain      ⍝ highest point on the map
+\ slope          ⍝ cumulative climb along a path
```

```haskell
+/ Elevation @ 64 64                -- total elevation of the map
max/ Threat @ Frontier              -- hottest cell in a zone
scan2(+) Cost @ 64 64               -- summed-area table over a cost field
max\ Height @ (Eye + til n * north) -- occlusion test along a sightline
```

`+\` is a leading-axis scan. The summed-area table needs the two-axis `scan2`.

### 23. Grade over space (best cells)

```apl
⍒ , safety        ⍝ cells ordered by safety, descending
```

```haskell
top 8 (grade desc Safety @ 64 64) , spawn Sentry          -- 8 safest cells
64 64 & top 5 (grade desc Resource) , +MiningNode         -- 5 richest tiles
```

### 24. Replicate over space (density fields)

```apl
counts / cells    ⍝ per-cell multiplicity → a population
```

```haskell
64 64 , spawn Tree * Density                    -- per-cell count from a field
64 64 & Fertility > 0 , spawn Crop * Fertility  -- richer cells grow more
```

### 25. Named fields (`def` over coordinates)

```apl
Ridge ← {(1○ ⍺÷8) + (1○ ⍵÷8)}    ⍝ a heightmap as a function of position: x Ridge y mixes both coordinates
```

A field is a derived column over `x y`, or over a cell's value and its neighbor.

```haskell
def ridge = sin(x / 8) + sin(y / 8)                -- a heightmap field
def basin = ridge < 0                               -- a derived spatial predicate
def slope = abs(Height - neighbor(clamp).Height)    -- local gradient via the neighbor

64 64 & ridge > 0.5 , spawn Peak     -- spawn on the ridges
64 64 & basin , Water = 100          -- flood the basins
64 64 & slope > 30 , +Cliff          -- steep cells become cliffs
```

A bare `neighbor` assumes the registered default stencil and boundary rule. `neighbor(clamp)` makes the boundary policy visible.

### 26. Source code that looks like the result (board literal)

```apl
⍉ 8 8 ⍴ glyphs    ⍝ reshape a flat glyph string into a board
```

A string literal becomes a glyph lattice carrying a `char` per cell. A registered lookup maps each glyph to a spawn type.

```haskell
"RNBQKBNRPPPPPPPP................................pppppppprnbqkbnr"
  to 8 8 , spawn (pieceOf char)   -- exactly 64 glyphs, no separators: the shape must consume the literal
```

---

## Part V — The two habitats

```haskell
                  over entities (rank 1)        over space (rank 2)
generator   n            (↕n)             w h          (↕ w‿h)
reduce      +/ Gold @ Nord               +/ Elevation @ 64 64
scan        +\ Damage @ graded           +\ Cost @ 64 64
grade       top 5 (grade Threat)         top 8 (grade Safety @ 64 64)
outer       [f | a<-A, b<-B]             64 64 & (x+y)%2==0
replicate   spawn Minion * Count         spawn Tree * Density
reshape     pos = to 20                  pos = to 4 _
shift       prev.X       (shift »)       neighbor.X   (shift «/»)
def         def threat = Dmg*Spd/Rng     def ridge = sin(x/8)+sin(y/8)
```

Over entities the operation joins across ragged components and leans on the archetype index. Over space it runs on a dense lattice with no join. The grammar `source & predicate , effect` holds across both. It is one generator read at two ranks: the entity key at rank 1, the cell index at rank 2.

---

## A farm interlude

```haskell
16 16 & (x + y) % 2 == 0 , spawn Wheat                       -- checkerboard field
Pen , Headcount = #/ (livestock' & Cattle)                   -- count cattle per pen, over the inverse fiber
Cow & Weight < avg/ Weight @ Cow , +Marked                   -- below-average weight (scoped fold: one scalar, broadcast)
Cow , pos.x = rank(Milk) * spacing                           -- line the herd by yield (value-only rank)
Plot & !Planted & #/ (neighbors' & Planted) >= 2 , +Planted  -- crops spread, one ring per statement
Plot , Moisture = avg/ neighbors'.Moisture                   -- per-plot mean over the neighbor fiber
Crop & Growth >= 100 , spawn Produce ; ~                     -- harvest the ripe
Farm & Acreage < avg/ Acreage @ Farm , Gold += 500           -- subsidy to small holdings
Gold , Gold = Gold * 1.05                                    -- 5% interest, world-wide
```

The two folds differ at the glyph. `@` after a fold is scoped-global, one scalar broadcast into the comparison. The tick is γ, a per-source column over each fiber. The moisture line drops no rows only because no grid fiber is empty. A pen with no animals keeps `Headcount` at `#/`'s identity 0. Performed, the crops-spread line advances one ring per statement. Installed with the arrow (`def spread = … => +Planted`, §11), it advances one ring per tick.

---

## Technical Explanation

A staging language for entity-component systems. FP / APL lineage, ASCII surface. An embedded query-and-command engine with console ergonomics, in the SQL, Datalog, and production-rules class. Beside a Lua-class host the split is coroutines versus triggers. Sequencing and UI stay imperative. Conditions over the world with bulk effects on a schedule, missions included, are ano's territory (§11).

A script states predicates over registered components. The host resolves which entities satisfy them. The predicate is the entity reference.

What the model buys is determinism and replay. A statement is a pure function of world state: gather against pre-state, emit an effect buffer, scatter at the barrier. The same world and the same statement log give the same run, so a session replays from its log and a rule set is testable against a snapshot.

Architecture. Scripts stage calls to host-registered functions, compile to bytecode, JIT the predicate-and-emit hot path, and return an effect buffer describing the work. The host interprets the buffer. The staging and registration mechanics are eBPF-shaped. The script invokes only registered functions, and component types and read/write footprints reside in the registry, declared once at registration and referenced by name in scripts. But the safety story is stronger than the one borrowed. eBPF needs a verifier to bound a general instruction set after the fact. Ano is total by construction: no loops, no recursion, registered functions, declared footprints. Termination is a corollary of the grammar, not a check bolted onto it.

The registry. The registry is ano's entire contact surface with the host: a script can name nothing the registry does not hold. Registration binds a name to the host five ways, all at compile time.

- A mutable column: an ordinary component column, read and write footprints declared, the Tier 1/2 territory every effect targets.
- A readonly column: a column whose write footprint is declared empty, so no effect buffer can name it as a target. This holds statically, at installation, not by runtime check. This is where the host exposes hot-path state (physics positions, render data) that scripts may predicate on but only C may move. Readonly does not exempt a column from the clock. Ano observes it at the tick's ingest snapshot, so host mutation lands between ticks as far as any script can tell, and the determinism claim survives.
- A callable function: host code invoked by name (`fib(index)`, `polar`, `phyllotaxis`), the Tier 3 dispatch path. The host runs it, the script keys off the returned column.
- An alias: a deictic resolver (`^cursor`, `^observer`, `^world`), re-resolved per evaluation, the こそあど engine (§2, the grammar appendix). The registry supplies the resolver, never a stored ID.
- An explicit binding: a bare proper noun bound as a constant (`Player`, `rally`, `Whiterun`). The sigil splits the last two kinds. `^name` moves with the context, a bare proper noun is fixed at registration. `Player.pos` mirror-reads through one, `spawn Convoy at rally` places by one, `Merchant @ Whiterun` scopes by one. What the constant denotes (an entity key, a pre-baked selection, an archetype) is open (Open Questions, Explicit bindings). The surface reads identically under all three, and the opacity is the point: the registry carries the denotation so the script never spells it.

The namespace is flat. Five kinds, one namespace, and no sigil marks provenance. The reserved words are a small closed set the lexer owns: the verbs, the hinge, the connectives. Every other name in a script is a registry row, and sentence position alone fixes its syntactic kind. A name applied to arguments is a callable, a name in a value position is a column, a bare name in a predicate is a mask, a proper noun in source position is a binding. Where a row came from is invisible to the grammar by design. `polar` is prelude, a row ano ships with. `phyllotaxis` arrives with the host. An author's `def` adds a third shipper. The script reads identically under all of them, the way C holds `sin` from libm and a user's function in one flat identifier space and leaves the coloring to the editor's symbol table. Provenance is metadata, and metadata is tooling's job (hover, color, the registry inspector), never a glyph. The surface spends its one sigil on deixis, a semantic axis, not a provenance one.

Two faces. The registry as drawn binds downward, to the C host: the Ground, raw nouns (memory, entities, state) pushed up into the syntax. It has a second face. A proving host can bind laws into the same namespace, and two such rows already exist unnamed: a reducer's registered identity (§12) and the merge certificates (§10, §11). Ground and sky entries differ in exactly one respect, the proof obligation. Ground entries are axioms: a resolver returns one entity, a footprint is honest. The trust floor sits at ingest the way every verified stack places one at its hardware. Sky entries are theorems the compiler is licensed to rewrite by. A statement is the inference step between them. The sky face is optional equipment. The compiler ships its own small law table (the fold identities, the merge families) and consults no prover to use it. A witness is owed only when a new law is registered, once, at registration, and no current example registers one. The doctrine, the Haskell embedding that prototypes it, and the machine-as-world hypothesis it opens are developed in `ano-sky.md`. The witness a sky entry must carry is open (Open Questions, The Sky Registry).

### Data model

Components are dense columns. An entity is a view, a key into those columns. The phrase "the entity's components" denotes the columns holding an entry under that key. Within a column the data is dense. Raggedness arises only at the cross-column cut, since different keys appear in different columns. This mirrors q, where a table is stored as a collection of column lists, operations on columns are vector operations, and atomic, aggregate, and uniform functions reduce to direct memory addressing.

### Two access patterns

Down a column is array calculus, a dense vector operation with no presence test. Across columns is a relational join, an intersection over which keys appear in which columns. The count of distinct components in a selector predicts the cost. Zero or one (or pure space) is a dense column op. Two or more is a cross-column join. The archetype index materializes "which keys appear in this exact column set," so the join evaluates presence once per archetype rather than once per key.

### Entity as view

Named views over the column store, differing only in binding time. An archetype is a view frozen at ship time. A `def` is a view frozen by the author. A bare predicate is a view computed at evaluation. Registration is early-bound selection.

### The relationship hop

A relationship component stores another entity's ID. `rel.Comp` is the indexed gather `Comp[rel]`, a foreign-key join, total under absent or dangling links by failing the predicate. This is the relational-model ECS that Flecs and the Bevy relations work have independently converged on.

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
| Space | PuzzleScript | Pattern-rewrite over a grid: the space tier as rules |
| Effects | ECS command buffers (Flecs, Bevy) | The deferred effect buffer, committed at a sync point |

### The maths

The selection sublanguage is relational algebra with grouping: selection (σ) by predicate, join (⋈) by relationship, projection (π) by component access, grouped aggregation (γ) by the fold over a relationship's fibers. σ, ⋈, π alone cannot express a grouped or correlated aggregate, and the farm needs three. The column sublanguage is the array calculus: reduce, scan, grade, outer product, replicate, reshape over dense vectors. A column store unifies them, since "set of rows" and "array of values" are the same bytes viewed along two axes. The comprehension correspondence (Trinder and Wadler 1989/1991; Buneman, Libkin, Suciu, Tannen, Wong 1994) identifies the double-generator comprehension with σ_p(A × B), the θ-join, so the comprehension and the join are one object. The combinator core is structural recursion over finite columns, so totality is a corollary of the grammar: no fixpoint, no unbounded iteration ever parses. The effect sublanguage has the same standing. A statement denotes a pure state transformer `World → World` (gather, effect buffer, scatter), so a program is a value in the state monad over the actual store, effects within one barrier composing in the commutative regime the merge laws declare (§10) and statements composing sequentially at the barrier: the two regimes an IO bind fuses into one, split back apart (`ano-sky.md`). The formal development lives in `proofs/foundations.md`.

---
# Tiers and Algebras

Three tiers, ordered by what a write into the data costs. The cost is fixed by the invariance an operation must keep (Klein's Erlangen program: classify the algebra by what it must commute with). Demand more of the index or the value, get less operational freedom. The tiers run from the permissive ground (space) through the aligned ledger (records) to the walled-off opaque. A datum carries no tier on its own. The tier is the (operation, view) pair. The formal development (the laws, the counterexamples, the machine-checked witnesses) lives in `proofs/foundations.md`. This section keeps the English and the worked examples.

---

# Tier 1 — Space

English. The index is a *place*, not a thing. A coordinate carries order and geometry but no durable identity of its own, because the ground is regenerable. Writing into position-space inscribes a figure on a conserved ground. So the full calculus applies: free rank change, scan, reduce, reshape, grade, annihilate cells. No closure demand, no contract, no types. It doesn't *keep* a promise. It has none. It just IS.

Math. The content of the tier is the key asymmetry. The index of space is **regenerable**: `I = ↕shape`, a definable key, recomputable from the shape alone at any time. The index of records is **nominal**: an allocated key, held only by the store, unrecoverable once dropped. That asymmetry is why the rank-changing index operations (`(¬m)/c`, reshape, a fold to lower rank) are free over space and forbidden as record write-backs. Over space no address is lost that `↕` cannot remint. Over records a dropped key is gone. Cells are still referred to durably (`Water = 100`, `+Cliff`, moisture diffusion all store state at cells across ticks), so nothing here says the figure is owed to no one. Only the ground under it is definable, and that alone buys the freedom. The tier demands coherence at the de/at boundary, not symmetry of the operator: the frame check `pos = φ(k) = o + S·k` with `@` fixing `(o, S)` (Part IV), and the counter identity `[world] = [world] + [world/cell]·[cell]`, the unit consistency the counter-typed numeral enforces. A reduction is a fold, a catamorphism `+/ : ℝⁿ → ℝ`, not a projection and not a deletion. The read consumes nothing and the field persists.

BQN:
```bqn
+´∾ h        # fold a 2D field to a scalar: rank 2 → 0, the field survives, reads are non-destructive
(¬m)/ c      # annihilate cells by mask: legal, ↕ remints the ground
⌽ g          # reverse/reshape: free, order is data not identity
```
Ano:
```haskell
8 8 & (x + y) % 2 == 0 , spawn Wheat   -- inscribe a figure (checkerboard) on the ground
+/ Elevation @ 64 64                    -- fold the field to a scalar; the field persists
+\ Cost @ 64 64                         -- scan: order is intrinsic to position
```

---

# Tier 2 — Records

English. The index is a *name* that must survive. The slot holds a record, and the record *is* the information. Reads are free (a read leaves into Tier 1's open world owing nothing). A write *back into* the slot is an amendment to a ledger: it must preserve the names or it's forgery, lost information. So write-backs are forced to be identity-aligned. Anything joined-against or referred-to durably must live here. One closure demand ⟹ one contract ⟹ one type.

Math. The law is permutation-equivariance under the **diagonal action** of `Sym(I)` on the whole per-entity record. A write-back `w : (I → V₁ × ⋯ × V_k) → (I → V_j)` may read several columns (`Nord & TwoHanded > 60 , Gold += 1000` reads two and writes a third, so a single-column signature would outlaw the flagship line) and must satisfy `w(ρ ∘ σ) = w(ρ) ∘ σ` for every `σ ∈ Sym(I)`. Relabel the entities and every column relabels together. The write must not notice. Maximal index symmetry ⟹ minimal operational freedom: the characterization is `f(c)_i = φ(c_i, ⟦c⟧)`, pointwise work plus multiset-level aggregates, nothing else (linear case `a·c + b·(Σc)·1`, Schur). Two corollaries fall out. *No canonical previous*: any order-dependent operator breaks the law, so a Fibonacci through gold held by Nords is impossible as a theorem, and an entity scan is legal only along a declared order (`along`, grade). The order is exactly the extra structure that dissolves the obstruction. *Ties*: stable rank breaks the law (`[5,5,3]` under a swap of the tied pair), so rank write-backs are value-only, dense or fractional. Reads are unconstrained. They exit the tier. Free order-work re-enters by conjugating with the data-derived grade `σ_c` (sort, act, unsort): `h(c) = f(c ∘ σ_c) ∘ σ_c⁻¹`, equivariant precisely because value-only ties give `σ_{c∘τ} = τ⁻¹ ∘ σ_c`. Conjugating by a *fixed* σ is not equivariant. `I` is invariant under value-writes. Creating and destroying names is a *structural* op, staged separately.

BQN:
```bqn
gold + 1000 × nord              # masked add: length & alignment MUST equal input (α→α)
+´ gold                         # read out to scalar: free, leaves the tier, owes nothing
(⍋⍋gold) ⊏ +` (⍋gold) ⊏ gold    # conjugation: grade out, free order-work in value order, index back aligned
```
Ano:
```haskell
Nord & TwoHanded > 60 , Gold += 1000   -- α→α, entity 47's gold stays entity 47's
+/ Gold @ Nord                          -- read: exits to a scalar, no return owed
Unit , Rank = rank(Gold)                -- conjugate by the grade: free order-work, scatter back aligned, value-only ties
```

---

# Tier 3 — Opaque

English. The *value* means something no array operator respects: a behaviour tree, a graph-with-traversal, a nav-mesh. All that remains is selection (the carrying entity is still an identity, still addressable) and dispatch (hand it to a registered host routine; ano guarantees the envelope, never inspects the value). The Erlang hand-off. Maximal partiality ⟹ maximal type ⟹ walled behind dispatch.

Math. The law is **naturality in `V`**: parametricity. Not "no map exists", since selection and dispatch are maps. No admitted map *inspects* the value. Every algebra map over an opaque column must be a family `f_V : (I→V) → (J→V)` natural in `V`, and by Yoneda every such map is a reindexing `f(c) = c ∘ u` for some `u : J → I`. So the legal maps are index manipulations: select, the subobject inclusion `ι : I_P ↪ I` (a mono into the index, never an endofunction on it), and dispatch `h : V ⇝ host`, admitted by the envelope, never by inspection. Tier is a property of the (operation, view) pair, not the data. The same bits viewed as `V₀ = 𝔹` (matrix) sit in Tier 1. Viewed as `V = Graph` (asserted traversal meaning) they sit in Tier 3. The type-pun is the functor `V₀ ⇄ V` that asserts or strips the meaning, identical to opening `√` into ℂ vs pinning it to ℝ: you open or close the codomain, sliding along the one axis.

BQN:
```bqn
+´ A          # SAME bits as a matrix → Tier 1, out-degrees, free
# as a graph → no expression exists; hand off
```
Ano:
```haskell
Node , OutDeg = +/ Adj@row          -- matrix view: Tier 1, full calculus
Hostile , shortestPath via Adj      -- graph view: Tier 3, host runs Dijkstra, writes a column back
^cursor , runBehaviorTree           -- opaque: select + dispatch, ano never looks inside
```

---

The one axis under all three. The ladder is monotone. Tier 1 demands **geometry of the index**: unit and frame coherence at the de/at boundary. Tier 2 demands **full symmetry of the index**: `Sym(I)`-equivariance under the diagonal action. Tier 3 demands **full abstraction of the value**: naturality in `V`. The monotonicity principle is the spine: for groups `H ⊆ G`, `Equiv_G ⊆ Equiv_H`. Demand more symmetry, admit fewer maps. Naturality in `V` extends the principle past groups, to invariance under *all* value substitutions, the limit of the demand, leaving the thinnest algebra. The data sits *nowhere* until an operation places it on the axis. Promotion and demotion are sliding along it, by opening the codomain or asserting a meaning.

Types live on the algebra, not the contents (direction confirmed, 2026-07-11). A column keeps one carrier — num, bool, sym, vec — and accumulates constraint evidence: `unique` (injective), `ordered` (grade and max admitted), `monoid(op, id)` (fold with identity), `keyed(col)`, quantized (ano-time's ingress boundary). The num carrier is IEEE 754 float64 with the non-finites refused at the world boundary — the value domain is the finite doubles, load ∘ save the identity on it, an overflowing tick a refused tick (the manual's edges chapter walks the cliffs). Tags gate which operators the compiler admits, and several coexist because they are stateless lawful predicates with set-intersection semantics, never inheritance. The lineage: qualified types (Wadler–Blott 1989, "How to make ad-hoc polymorphism less ad hoc" — one type, many instances: Num, Ord, Monoid), Rust traits, the Agda/Lean algebraic hierarchies (one carrier, many lawful structures, laws proved), TAPL's bounded quantification for the subtype flavor, with `unique` at the refinement-types end (Liquid Haskell: a predicate on values enforced at boundaries). §12's fold contract already IS this — `+/` demands a monoid, `max/` a semigroup whose empty scope fails the row — and §5's keyed hop is its relational face: injectivity is the license for inversion. The type IS the license for the operation.

The first value refinements shipped with Steel (2026-07-12). `bool` admits {0,1}, `nat` the naturals to 2^53 (the last contiguous integer of the double), `int` their signed twin, and a `range <col> <lo> <hi>` rider declares bounds over any numeric carrier. Enforcement splits across the three boundaries by what each owns: the loader seals data at rest — out-of-domain refuses, the `unique` precedent — the barrier retracts every committed write onto the carrier set (bool by positive-is-true, nat/int by floor then clamp, range by clamp; silent, the type's algebra, applied once per barrier however many writes merged), and the editor repairs interactively — kore clamps the entered value and says so. A mask op on a bool column elides the retraction: the refinement proves the write in-domain and the check disappears. Demos `13-typed-registries`; the negative battery pins the refusals.

The problem I was struggling with was how to have the scripting language remain consistent when faced with different kinds of information. Then this was revealed to me.

Tier 1 and 2 are native to the language, can happen entirely within ano's interpreter itself. Both encode columnar, homogenous, and contiguous information. The difference is in what the data actually represents, whether it's a space or records. Space is innately, ontologically, immutable[sic]. Records can take most of the same operations, operations cast over it irreversibly overwrite the information by those indices.

Tier 3 is data that is non-mutable by virtue of array operations over it beind *undefined*, so reads and effects upon Tier 3 data is delegated to the host process.

Game engine integration: The lang ingests the raw ECS data at the end of each tick, in accordance with the registry table for components and what algebra-tier they fall into.
For example, trying to draw a fib sequence through gold held by nords would be nonsensical. It would be undefined.
However, to draw a fib over space doesnt destroy space, you can't overwrite it. You create projections that are shapes you can use. Feed that into @Spawn or @Move. One line and you get a checkerboard, or a fractal, or anything you can model into a spatial projection.

Tier 3 can include non-columnar or functions where trying to treat it as a vector database would be incorrect. You can't select and operate upon an entity's MeshNavGraph directly, like you could for numerical data or space. So we allow the compile-time integration to register these types of objects and the functions that operate over them as static bindings, which call *outside* of ano but can't be operated upon directly in our native algebra. We allow for Tier 3 because one would still like to invoke things from the console, even if they aren't natively scriptable.

That's Ano.

## Appendix — Grammar

### Precedence

Fourteen levels, loosest to tightest. Everything else in the document is a consequence of this table.

- 1 (loosest) — `,` and `=>`: the hinge; everything left is selection, everything right is effect (`,` performs, `=>` installs). The comma inside a parenthesized presence tuple `(Nord, !TwoHanded)` is bracketed and does not compete.
- 2 — `;`: effect batching within the statement's one barrier.
- 3 — `|>`: pipeline stages (`order by`, `take`, `expand`).
- 4 — effect verbs and assignment: `= += -= *= /=` (`=` assigns only in effect position; the equals glyph, below), `+Comp -Comp ~ spawn`, the locatives `at` and `to`, replicate `*` in `spawn X * n`.
- 5 — `|`: mask or.
- 6 — `&`: mask and.
- 7 — `!`: mask not, prefix on one mask term.
- 8 — comparison: `== != < <= > >=`, and `=` in selection position (the equals glyph, below).
- 9 — fold and scan prefixes: `f/ f\` with f an operator or a registered reducer name, `fold(f)`, `scan(f) … along`, `grade`, `top k`.
- 10 — additive arithmetic: `+ -`.
- 11 — multiplicative arithmetic: `* / %`.
- 12 — `@` scope: locative on a mask (`Cheese @ cellar`) and fold scope (`Gold @ Nord`), one meaning: evaluate within this scope. The scope may take an anchor tail `mask @ frame(args) at origin`, the level-4 locative `at` reappearing to fill the frame's origin o (Part IV).
- 13 — hops: the dot `.` and the set-hop tick `'`, the gather, the tightest operator.
- 14 (tightest) — atoms: names, the `^alias` sigil (lexical, part of the identifier), colon symbols (`:Sym`), counter-typed numerals (`3mo`), parens and comprehension brackets.

Resolution, worked: `Cheese @ cellar & Aged > 3mo` parses as `(Cheese @ cellar) & (Aged > 3mo)`, since `@` (12) binds its scope before `&` (6), and `>` (8) binds before `&`. `Cow & Weight < avg/ Weight @ Cow` parses as `Cow & (Weight < (avg/ (Weight @ Cow)))`, since the fold prefix (9) outbinds the comparison (8).

### Fold and scan permutations

One table for the whole level-9 family, folds and scans together. Lineage: in k, `&` IS min and `|` IS max over numerics; ano's boolean reading is the k reading restricted to masks.

| f | `f/` fold | `f\` scan | empty-scope identity |
|---|---|---|---|
| `+` | sum | running sum | 0 |
| `*` | product | running product | 1 |
| `&` | ALL | still-all: a latch that trips off at the first false and stays off | 1 (vacuous truth) |
| `\|` | ANY | ever-any: a latch that trips on at the first true and stays on | 0 |
| `#` | count | running count | 0 |
| `max` | maximum | running peak (occlusion, high-water) | none → row drops |
| `min` | minimum | running floor | none → row drops |
| `avg` | fold-and-finish mean | running mean | none → row drops |
| `-` | rejected: not associative | — | — |
| `/` (divide) | rejected: not associative; `//` additionally unlexable (`/` is fold-marker and replicate) | — | — |

The identity column restates the §12/§13 law: a fold with a registered identity yields it on the empty scope, and a reducer without one fails the row. That is left-join-null extended to the empty fiber. The γ column-form (`f/ rel'.Comp`) inherits the same identities per fiber. A named reducer (`threat/`) enters the same scheme — registered identity or fail the empty scope — and its scan (`threat\`) is length-preserving, so the empty scope yields the empty column with no identity consulted.

The max ruling, recorded (2026-07-11): `max/` and `max\` stay named forms. `>/` was examined and rejected — `>` is a comparison returning bool, folding it is non-associative nonsense, and making `>/` mean max requires reinterpreting `>` as "the greater-of" in fold position, the same pun refused for `-/` (APL's alternating sum). k earns `|/` as max-fold because k's `|` IS max natively, not a fold-position reinterpretation. Under the named-reducer unification, max/min/avg are registered reducers like any other name, so no glyph is needed. Adopting k's `|` as native max — which would earn `|/` honestly, with boolean OR renamed to `||` — stays deliberately unruled.

Scan cells anoc does not yet emit: `min\`, the running mean, and the running count are ruled forms the compiler still refuses (`unknown scan op`); `+\ *\ &\ |\ max\` and named-reducer scans are live. The γ column-form takes operator folds today — a named reducer over fibers (`threat/ livestock'.Weight`) is refused with its own diagnostic, not yet in anoc.

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
8 8 & x == y , spawn Pillar                -- the rank-2 generator (↕ 8‿8), coordinates as columns (§20)
"…" to 8 8 , spawn (pieceOf char)          -- reshape pours the literal into a lattice source (§26)
Oil @ blast(5) at Firebolt.pos , +Fire     -- the anchored frame: the locative fixes (o, S) and `at` fills o with a mirror-read or a bound point (Part IV)
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

Dot, five roles, all gathers: (1) the functional relationship hop `rel.Comp`: one ID, one indexed read, left-join-null, chainable (`mentor.mentor.Dead`); (2) the mirror-read, the same hop rooted at a named singleton or alias (`Player.pos`, `^cursor.pos`): an arity-one gather usable inside effect expressions; (3) field projection into a registered compound component (`pos.x`): registry-resolved, no join; (4) the shift pseudo-relations on an ordered view or lattice (`prev`, `prev.prev`, `neighbor(clamp)`): functional single-step hops whose link is the view's order, boundary policy per registration, shifts never carries; (5) the gather inside a set hop (`neighbors'.Moisture`): the tick marks the fan-out, the dot still means gather. Dot never groups, never scopes, never folds.

`@`, two roles, one meaning: evaluate within this scope. (1) locative scope on a selection (`Merchant @ Whiterun`, `Cheese @ cellar`), which also fixes the coordinate frame `(o, S)`. The anchored form `mask @ frame(args) at origin` spells the frame explicitly, the locative `at` filling o with a mirror-read or a bound point. (2) fold and scan scope (`+/ Gold @ Nord`, `+/ Elevation @ 64 64`), always scoped-global, one scalar per fold. `@` never marks grouping. Grouping is the tick. Result arity is carried by fold-vs-tick, never by `@`.

The alias sigil `^` is not an operator: `^` glued to a name is one identifier (`^cursor`, `^observer`, `^world`), the こそあど deixis, re-resolved per evaluation. `^` appears nowhere else in the grammar, so `^name` never needs disambiguating.

### `!` and `^`

| sigil | is | precedence | binds to | resolved |
|---|---|---|---|---|
| `!` | mask NOT, a prefix operator | level 7 | one mask term | at evaluation, pointwise |
| `^` | not an operator: lexically part of the identifier; the alias/deixis sigil (`^cursor`, `^observer`, `^world`), こそあど | atom, level 14 | the name it is glued to | re-resolved per evaluation via the host |

They never compete: `!` negates a mask, `^` names a thing. `!^cursor` is "not the thing under the cursor" — the sigils compose, they do not overlap. One seam is deferred (2026-07-11): demo 14 pins both `!Whiterun` and `!^Whiterun` for one registered `bind … mask`, and whether `^` stays reserved to host-resolved deictics or reads as optional constant-reference sugar on any registered constant awaits its ruling. Both spellings stay legal meanwhile.

## Open Questions, Next Steps

```markdown
/\/\/\/\/\/\/\/\
```

- Explicit bindings, the denotation. The registry binds a bare proper noun as a constant, and the surface reads identically whether that constant is an entity key (`Player`), a pre-baked selection (`Whiterun`), or an archetype. The options are not exclusive. A binding could declare its denotation at registration and resolve by context: the mirror-read `Player.pos` demands a unique entity, the scope `@ Whiterun` demands a region, `at rally` demands a point. Whether one name may carry several faces or must declare exactly one is open. Context dispatch fits the language's inference stance. One face per name is easier to check and easier to read.

- Recurrences. `offset = prev.offset + prev.prev.offset` is not a recurrence under the one evaluation rule. The comma is gather-effect-scatter and every read observes pre-state, so `prev` is a parallel shift and the statement is one stencil step. No statement can carry a value along the line it is writing, and iterating it gives k stencil steps, never the order-carried sequence. A true recurrence is a scan whose step need not be associative, and §14's scan is a read-side column expression over a declared order. A scan that feeds its own column's scatter breaks the barrier by construction, so scans cannot live behind the barrier. Options, each with a cost. Keep recurrences host-side as registered functions (`fib(index)`, the current canonical form): the host runs the recursion and hands back a column, which a statement may key positions off in the same breath. Or admit a sequential `scan(f) along order` with non-associative f, legal only where its write footprint does not intersect its read footprint. Or admit a generator subclause: corecursion consumed under bounded demand, the lazy-list/Python-generator shape, total because the take is finite even when the definition is not, and read-side by construction so it never touches the barrier. A fixpoint/iterate form stays rejected, since totality comes from bounded demand, not a general fixpoint. One boundary note. Under a fixed-tickrate host the game loop is itself the scan, state[t+1] = F(state[t]), one barrier per tick. So recurrences across ticks are already expressible, and a stage counter advanced by standing rules is one running. Only the within-statement form is open. A neighboring form that is NOT a recurrence landed with the wand demos. The correlated order join ("next projectile at or after each modifier") is per-row over an ordered line, and it runs read-side twice: as data, an order-derived srel consumed by γ, and as value, a key column computed by scan-along standing in relation position with its inverse fibers feeding the same γ (demos 11-noita w3-b/w3-c). Neither carries a value along the line it writes, so the barrier is untouched. Unresolved. The Fibonacci examples are glossed as one stencil step, `fib(index)` canonical.

- Identity. The intensional/extensional boundary: how does a script say "the same bandit as last tick"? A predicate re-resolves per evaluation, so the surface is intensional at the statement level. Yet relationship components already store entity IDs, extensional handles, so the data level is extensional, and the surface cannot reach what the data already holds. The saved-mask continuation rule gives one statement of extension inside a script. Nothing spans ticks. The options pull against each other. A surface form that holds a resolved selection across ticks is a handle, which breaks the predicate-is-the-reference stance. A component that freezes the match (`+Marked`) keeps the stance but makes identity state the script must manage and retract. A third option rides the clock. A host with tick-stamped history (Anoptic plans a monotonic tick counter at a fixed rate) makes "the same bandit as last tick" an as-of join against the t−1 partition, q's `aj`, an extensional read recovered through the time axis, with no handle on the surface. Identity becomes an indexing question. Where the boundary between intensional statements and extensional data sits is the design. Ruled (2026-07-11) at the data level: IDs carry generations, kills bump them, the hop's staleness compare is O(1), slot reuse waits for tick seal (ano-ecs §2, §5, §12) — so the data's identity story is settled and the surface question is what remains. Unresolved.

- Rule retraction. A standing rule is named (`def spread = … => …`) so it can be withdrawn, but the retraction form is unspecified: an `undef spread`, a scope that expires (`=> … @ scene`), or a component guard the rule itself reads. A handle-shaped verb sits awkwardly beside predicate-is-the-reference. A guard makes rule state the script must manage and retract. Campaign logic makes this entry load-bearing: a mission stage is a standing rule that must withdraw itself on advance (§11). Unresolved.

- Staging and quotation. The Lisp row left the lineage table. Nothing in the current design is homoiconic: there is no quote form and no macro, and "registry" is Lua vocabulary, not Lisp. A staging story (scripts as data, rules that write rules) would have to earn the row back. The nihongo koto nominalizer, the clause-as-thing form, points at where it would enter. First step decided: `eval "…"` is ⍎ constrained to a literal one-statement quotation, spliced at parse time, reflection priced as program text, the footprint still visible to the §11 conflict check (demos 11-noita w6). A runtime string has an opaque footprint and waits on the interpreter. Templates with holes, the koto form, are where homoiconicity would be earned. Open past the literal splice.

- Outer product as a value. The double-generator comprehension covers the filtered-pairs case. Whether ano lets a script hold a materialized N×M matrix (`cross f A B`) as a first-class value is open.

- Space operations and their spelling. Working. Three primitives cover Part IV: filter (`/`, keep existing cells), generate (`↕`, a bare numeric shape, spelled `til`, mint a lattice where nothing exists), reshape (`⥊`, fold existing data into a shape, spelled `to`). The generate-vs-reshape boundary is decided by whether the operands already exist. The generate spelling is settled (`til`, q's word); the reshape spelling (`to` vs the `⥊` glyph) and whether `to` (a shape) and the locative `at` (a point) stay separate are unsettled.

- The Sky Registry, the witness obligation. A ground entry is an axiom and a sky entry is a theorem (Technical Explanation, Two faces), and the failure modes are not symmetric. A wrong axiom corrupts one gather. A wrong law miscompiles every statement it touches, silently. So what a sky entry must carry is the question. Three rungs, each priced. Trusted, like an eBPF helper: cheap, symmetric with the ground, silent when wrong. Property-tested, the law run against the BQN twin over generated worlds: catches most lies, proves nothing. Machine-checked, a lemma in the `proofs/foundations.md` regime stored in the registry and checked at registration: the only rung where "licensed to rewrite" means licensed. Whether the rung may vary per entry kind (a reducer identity is easy to check, a fusion law is not) is part of the question. `ano-sky.md` carries the development. Unresolved.

- The machine as a world. The hypothesis closing `ano-sky.md`: an x64 machine is columnar data plus a step function, a code column, a register column, memory, which is the standard formalization (Sail, the K framework, ACL2). An out-of-order core already runs the evaluation model: rename is the pre-state gather, the store buffer is the effect buffer, retirement is the barrier. Registering the machine itself as a world would make compilation an ano query, gather λ-terms and scatter instructions, the first Futamura projection with the evaluator written in ano. What is provable is dependence and layout, since barrier semantics makes intra-statement aliasing statically absent. What is not is latency, data-dependent and the Itanium lesson. Literal self-modifying code stays dead in the pipeline, so JIT synthesis into fresh columns is the usable form. First falsifiable step: the Haskell embedding (`ano-sky.md`, the prototype path). A hypothesis, not a roadmap. Unresolved.

- Bootstrap host. The first compiler may be written in APL, Haskell, or OCaml before the self-hosted toolchain. The bytecode VM and JIT target are fixed. The front-end implementation language is not. The Sky Registry reframes the choice without resolving it (`ano-sky.md`): the front-end is not the language ano is written in but the second host ano embeds in, which weights the pick toward the host whose type system can hold the registry schema. Ruled (2026-07-10): Rust. The language is drafted and its spec forged in Steel — the Rust reference implementation, standalone launcher included — and Cano, the embedded C implementation, is derived from verified Steel, never the inverse. The reframe's weighting held: Rust is the second host, and its type system holds the registry schema — `repr(C)` structs for the C ABI, affine ownership over columnar data. The bytecode VM and JIT target stay fixed; CBQN stays the differential oracle through the port.

- Reap ownership and granularity. Generational tombstoning is ruled (2026-07-11, ano-ecs §2/§5/§12): the gen bump plus presence clear is the mark, free-list reuse at tick seal is the deferred reap, and the mask-level meaning of `~` never changes — only storage reclamation is policy, exposed as the registry option `reap seal|host`. The remaining knob is the option's grain: world-wide as declared today, per archetype, or per column. Unresolved.



The name あの is the distal demonstrative ("that one over there").
