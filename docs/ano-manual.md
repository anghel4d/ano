# LEARN YOU AN ANO TO BECOME GREAT AND POWERFUL

A beginner's manual for ano (あの), in the proud tradition of learning yourself a Haskell for great good and some Erlang for the same. Like those books, this one assumes you are clever but new here, teaches by running code, and refuses to apologize for its jokes. Unlike those books, every example is pinned by a test you can run in the next sixty seconds. If a code block cites a file, that file is in this repository, it executes, and its post-state is asserted. The manual cannot drift from the language without a suite going red.

The chapters climb the same ladder as the demo suite, `demos/1-selection` through `demos/11-noita`, each new floor built on the last. Nothing is taught twice: when a later chapter needs an earlier idea it names it and moves on. What IS repeated, deliberately and often, is the problem. You will see the same task solved several ways side by side, same input data, same post-state, because a language's semantic integrity shows exactly there: two spellings of one meaning forced to agree in public.

Contents: Introduction · Starting out · The registry · Selection · Effects and the barrier · Folds and scans · Order · Generation · Space · Habitats and tiers · The grouped fold · The Japanese surface · Standing rules and the clock · The wand · Where the edges are.

## Introduction, or: that one over there

In Skyrim, you give gold to one entity by alias or by handle: `player.additem f 1000`. To reach a merchant you click it or look up its FormID. To reach every Nord with a high Two-Handed skill across the whole map, you write a script with a loop. In ano:

```haskell
Nord & TwoHanded > 60 , Gold += 1000
```

No alias, no handle, no FormID, no loop. The predicate is the reference. あの is the Japanese distal demonstrative, "that one over there", and pointing at things by describing them is the entire language. One shape governs everything you will ever write:

```haskell
source & predicate , effect
```

Selection on the left, effect on the right, a comma between. The source defaults to the live world. Everything else in this book, every fold, join, lattice, and standing rule, is a way of filling those three slots.

Two facts to hold onto before we start. First, Ano is embedded: it is the resident query-and-command language of a game engine's entity-component store, the way q is resident in kdb+. The host keeps its coroutines, cinematics, and UI. Ano gets conditions over the world plus bulk effects. Second, the native calculus has no loops, recursion, or fixpoints. One native statement terminates over finite inputs. Registered functions cross that boundary. Replay requires the same registry behavior, runtime, inputs, world, seed, and log.

## Starting out

You need Nix with flakes enabled, and nothing else. The dev shell carries the Rust toolchain and CBQN, the array language ano compiles to. From the repository root:

```text
nix develop            # BQN, cargo, and friends
cargo build --release  # builds ./target/release/steel, the ano-to-BQN transpiler
./target/release/steel --run demos/1-selection/001-canonical-masked-update.ano
```

If the last line printed nothing and exited 0, congratulations: you have run ano and it agreed with its own spec. `steel` compiled the file to a BQN program, ran it under CBQN, and asserted the post-state. A nonzero exit is a divergence. To skip the shell, prefix commands with `nix develop -c`. With no Nix at all, a Rust toolchain plus a `bqn` binary on PATH will do.

The demo tree comes in twins. Every `.bqn` file is a witness: the semantics worked out by hand in BQN, assertions included. Beside it sits an `.ano` twin, the same statements as an actual ano program, compiled and checked against the same post-state. Differential testing, both directions, on every example in this book. The three entry points:

```text
./demos/check.sh       # every .bqn witness (93 files)
./src/check-ano.sh     # every .ano twin through steel --run (257 files)
nix flake check        # both suites, hermetically
```

An `.ano` file is statements plus harness directives. A directive line starts `--!` (a plain comment starts `--`) and there are five: `--! registry ../registries/<name>.reg` loads the world fixture (a bare `<name>` instead loads `<name>.reg` from beside the `.ano`); `--! expect <col> = v v v ...` asserts a column's post-state in world order; `--! expect-n <k>` asserts the row count after spawns and despawns; `--! out <v ...>` asserts what a query statement prints; `--! ja` says the statement lines are in the Japanese surface (a later chapter). Here is the whole first demo, `demos/1-selection/001-canonical-masked-update.ano`:

```haskell
--! registry 001-canonical-masked-update
--! expect gold = 1100 200 300 1400 500 600

Nord & TwoHanded > 60 , Gold += 1000
```

Your first exercise is to break it. Change the 1400 to 1401, run it, and watch the assertion fail. Change it back and study why rows 0 and 3 moved and the others did not: row 2 has the skill but is not a Nord, row 4 likewise, and row 5 is a Nord sitting at exactly 60, which is not greater than 60. The harness is not a formality. It is how this manual keeps its promises.

## The registry, or: worlds on six rows

A script can name nothing the host did not register. The registry is ano's entire contact surface with its engine, and in this repository the `.reg` file format stands in for that engine. The world your first demo ran against, `demos/registries/001-canonical-masked-update.reg`:

```text
n 6
col nord bool 1 1 0 1 0 1
col twoHanded num 80 55 70 61 90 60
col gold num 100 200 300 400 500 600
```

Six entities, three components, and that is a world. Components are dense columns and an entity is a key into them. The column store is the data model, which is why selection can be relational algebra on one side and array calculus on the other with nothing converted between them. The line kinds you will meet across the demos:

```text
n 6                                # entity row count
col gold num 100 200 ...           # numeric column; bool columns are masks; sym columns hold :Enum values
col pos vec 5 5 | 40 5 ...         # pair column (positions); fixed arity — ragged data lives behind an srel
pres twoHanded 1 1 0 1 1 1         # presence: absent rows carry junk, the (Nord, TwoHanded _) machinery
unique id 0 1 2 3 4 5              # a column under an algebraic constraint: pairwise-distinct, checked at load
rel mentor -1 0 3 -1 2 2           # functional relationship, one target id per row, -1 dangling, keyed to the row index
rel id mentor -1 0 3 -1 2 2        # the keyed form: key column first, then the name — mentor is a rel over id
srel targets 3 4 | 4 5 | | ...     # set-valued relationship, one fiber per row; keys the same way
inv livestock pen                  # set-valued as the inverse read of a functional rel (keyed rels invert keyed)
alias cursor 0 0 1 0 0 0           # deictic resolver, re-resolved per evaluation
bind Player entity 2               # proper-noun constants: entity, mask, point, num, vec
default gold 50                    # spawn fill for one column; the full lookup is proto → default → type zero
def Marine soldier=1 hp=100        # a proto: the registered archetype, spawn Marine fills through it
fn fib {𝕩...}                      # a callable, carrying its own BQN the way a host carries C
lattice 12 12                      # a rank-2 world (space chapter)
field oil bool 0 0 1 ...           # per-cell lattice column, row-major
reap seal                          # despawn reclamation policy: seal (default) or host-owned
ja 北 nord                         # Japanese alias for the ja skin
```

That list is also a census of the ways a name reaches the host: a mutable column, a readonly column (write footprint declared empty: physics positions you may predicate on but never move), a callable, an alias, a binding, and a proto — the archetype noun `spawn` fills through. The bare namespace is flat, and sentence position alone decides what a name is doing. Applied to arguments it is a callable, in a value position a column, bare in a predicate a mask, a proper noun in source position a binding. No sigil marks provenance: `polar` from the prelude and `phyllotaxis` from the host read identically, the way C holds `sin` from libm and your own function in one identifier space. The one sigil, `^`, selects the dynamic alias overlay. `^cursor` moves with the context. `^Whiterun` reads a live alias named `Whiterun` when one exists and otherwise falls through to bare `Whiterun`; rebinding or deleting the alias never changes the registered name.

## Selection

Everything left of the comma. A bare component name is the set of entities carrying it, `&` `|` `!` build masks out of masks, and comparisons build masks out of columns. These lines are `demos/1-selection` and the world's most honest query language:

```haskell
Nord , Gold += 100
Bandit & !Dead & Faction == :Bandit , Faction = :Hostile
Gold > Health * 10 , +Greedy
Merchant @ Whiterun , Gold += 5000
^cursor , Health = 0
```

Four small rules carry all of that. The colon is the atom literal of Erlang and Ruby: `:Bandit` is the enum value, `Bandit` the component mask, decided lexically, never by lookup. The equals glyph is positional, the SQL rule: left of the hinge `=` compares (identical to `==`, which stays legal everywhere), right of it assigns. So `NPC & Tunic = :Red , Gold = 0` reads one comparison, one assignment. `@` and `^` get their own paragraphs in a moment. Precedence runs loosest-to-tightest through hinge, `;`, `|>`, effect verbs, `|`, `&`, `!`, comparisons, fold prefixes, `+ -`, `* / %`, `@`, dot. So `Cheese @ cellar & Aged > 3mo` is `(Cheese @ cellar) & (Aged > 3mo)` and you can stop counting parentheses. The full fourteen-level table is the spec appendix.

The sigil first. `^cursor` is a dynamic alias. `bind Player entity 2` made a proper noun: the referent was chosen once, at registration, and never moves. `^cursor` is the word "you". Who it refers to is decided at the moment of speaking, by the engine, not the script. Concretely, the host registers a resolver (a tiny function like "raycast from the mouse") and every gather re-runs it. So `^cursor , Health = 0` kills whatever is under the mouse at that gather, and two statements mentioning `^cursor` may hit two different entities. That is why it carries a glyph in a language with no other sigils: you must know this name can move between statements. In C# it is an expression-bodied property, never a field. `Entity Cursor => Physics.Raycast(mouse)` re-raycasts on every read, the way `DateTime.Now` differs from a stored timestamp. In Haskell it is `asks cursor` in a Reader: the script is a function of an environment the engine rebuilds each tick. The host may install, rebind, or delete the alias between statement steps. In filesystem terms, `Player` is `/home/pyrus` and `^cursor` is `./`, the same spelling landing somewhere different depending on where you are standing. Against `Nord` the difference is arity. A component name is a mask over many rows. `^cursor` resolves to a referent, usable anywhere a selection or a mirror-read root goes (`^cursor.pos`). A sigiled name first reads the live alias overlay and falls through to its bare namesake when no alias exists. `^` glued to a name is one identifier and appears nowhere else, so nothing has to guess.

`@` is the scope operator, and it has exactly one meaning, always: evaluate the left thing within this scope. The Japanese surface keeps them as separate words: the operator is the particle で ("at"), the sigil a demonstrative ("this one here"). What varies across the operator's uses is what you asked it to evaluate, never what `@` does:

```haskell
Merchant @ Whiterun            -- a mask, evaluated in a scope → a mask
+/ Gold @ Nord                 -- a fold, evaluated over a scope → one scalar, always
+/ Elevation @ 64 64           -- same, the scope is a lattice
+/ Adj@row                     -- same, the scope is the row: a per-row view for the fold
Oil @ blast(5) at impact       -- a mask under a frame-fixing scope (the anchored form)
```

The last three live in later chapters (space, tiers, the wand) but the sentence never changes, and result arity is never carried by `@`. One consequence worth internalizing early: misplace a parenthesis around a scope and you get a different well-typed meaning, not garbage. `+/ Gold @ (Merchant & !Whiterun)` folds over the compound mask. `+/ Gold @ Merchant & !Whiterun` binds the scope to the fold first and broadcasts the scalar total onto the leftover masks. The failure mode isn't nonsense. It's meaningful things you didn't mean. `demos/3-fold-scan/028-reductions.ano` pins both spellings side by side.

Presence is three-valued, and the tuple form tells the cases apart: present-and-constrained, present-any, absent (`demos/1-selection/006-presence-patterns.ano`):

```haskell
(Nord, TwoHanded > 60) , Gold += 1000
(Nord, TwoHanded _)    , +Trained
(Nord, !TwoHanded)     , +Untrained
```

A relationship is a component whose value is another entity's id, and the dot is the hop: `rel.Comp` reads the target id and gathers `Comp` there, one indexed read, chainable (`mentor.mentor.Dead`). A bare relationship is the same found-guard as a mask: true only when the target resolves now, not merely when an id was once assigned. The left-join-null law rides along: an absent or dangling link fails the predicate and the row drops out, no null ever surfaces. What "id" means is the rel's key column: declare `unique id …` and write the key first (`rel id mentor …`) and the hop resolves stored ids against it — a target that despawned simply fails the found-guard and drops, staleness included in the same law. `DEAD` is the one diagnostic word for any nonnegative target that fails that guard. Never-existed, despawned, and mistyped do not become separate states, while the `-1` None sentinel stays silent. An undeclared rel is keyed to the row index, which is exactly what its values say. The same dot rooted at a binding is the mirror-read (`Player.pos`), and into a compound component it is field projection (`pos.x`). Dot always gathers. It never groups, scopes, or folds.

```haskell
Nord & mentor.TwoHanded > 80 , Gold += 1000
Student & mentor.Dead , -Mentored
```

The set hop is the postfix tick, for relationships that fan out. `rel'` is the fiber, the target set per source, and it may stand in exactly three places. In source position `sel.rel'` selects the image, the union of the fibers, and because a selection is a mask the effect lands once per target no matter how many sources reach it: set semantics, no silent multiplicity. Under a fold it becomes the quantifiers and the grouped fold (two chapters ahead). Bare anywhere else, it is a compile error, on purpose.

```haskell
Frenzy.targets' , +Frenzied              -- the image: each reached target, once
Plot & |/ neighbors'.Planted , +Watered  -- any planted neighbor
Pen & &/ livestock'.Healthy , +Certified -- all animals healthy
```

Finally, `def` names a selection, and the name folds into any selection slot at plan time, free (`009-named-selections.ano`):

```haskell
def master = Human & Nord & TwoHanded > 60
def rich   = Gold > 10000
master & rich , +Legendary
```

## Effects and the barrier

Everything right of the comma, and the one law that makes ano deterministic. A value effect is masked arithmetic down a dense column. Assignment overwrites under the mask. Structural effects change which rows exist or what they carry (`demos/2-effects`):

```haskell
Bandit , Health -= 25          -- value
Wounded , Speed /= 2           -- value
Dead , Loot = :Empty           -- assignment
Burdened , -Encumbered         -- remove component
Dead , ~                       -- despawn
```

Now the law. Every statement is exactly one gather-effect-scatter **barrier**: the selection gathers against the pre-state, every effect expression reads only that pre-state, and the writes commit together at the statement's end, where the next statement's gather first observes them. `;` batches several effects into one statement's barrier. All of them still read pre-state, so ordering among them is invisible:

```haskell
Nord & TwoHanded > 60 , Gold += 1000 ; +Blessed
^cursor , Knockback 5 ; Flash :Red ; -Shielded
```

What if two batched effects write the same column? They must commute under a registered merge law. The additive family (`+=` `-=`) merges, the multiplicative family (`*=` `/=`) merges, presence writes of one kind merge idempotently, and anything else (`+=` beside `*=`, a double `=`) is rejected at compile time with "no merge law: written twice in one barrier". Not reordered, not last-wins, rejected. The demos `024-pre-state.ano` and `022-barrier-granularity-a/b.ano` pin the law from both sides.

Statements chain through the saved mask. `~` alone, a leading comma, or a bare effect line continues the antecedent: a new statement, a new barrier, over the mask saved at the previous gather. The mask, not the pre-state. The classic (`demos/9-nihongo/094-block-anaphora.ano` pins it):

```haskell
Nord & Dead , spawn Ghost
~
```

The ghosts survive. The despawn applies the saved mask, the dead Nords, never a re-gather that would sweep up the freshly minted ghosts. If the block opens cold with a bare effect (`spawn Wheat`), the subject falls to the host default `^cursor`. Japanese grammar calls the machinery ゼロが, the zero subject, and ano simply implements it.

## Folds and scans

APL's `/` and `\`, aimed at selections (`demos/3-fold-scan`). A fold collapses a column to a scalar. `@` scopes it, and a fold under `@` is always exactly one scalar:

```haskell
+/ Gold @ Nord               -- total Nord gold
&/ Alive @ Party             -- all alive?
|/ Threat @ Frontier         -- maximum threat
&/ Distance @ Route          -- minimum distance
#/ (Nord & TwoHanded > 60)   -- count; takes a parenthesized mask
threat/ Damage @ Enemies     -- named registered reducer: the slash attaches to the name
fold(threat) Damage @ Enemies    -- the long form, same fold
```

Two honesty rules. A fold on a declared order is exact left accumulation and accepts any compatible registry step. Unordered regrouping requires associativity; parallel/unordered execution that may discard traversal order requires associativity and commutativity. An identity is needed only to produce a value on empty input. `+/` and `#/` give 0 on empty input. Mask `|/` gives false and mask `&/` gives true. Numeric `|/` and `&/` have no identity in Ano's finite float64 carrier, so they fail an empty scope. Their `max/` and `min/` bridge spellings obey the same law. `avg/` also fails because the pairwise mean is not its reduction. Failure means no result row, the same nothing as a predicate with no matches. An assignment writes nothing and a bare query prints nothing. Steel's current bare-query `0` is an implementation bug tracked in `todo/18-empty-result-output.md`.

A scan accumulates and returns a column of equal length, which means it needs an order, and an abstract selection has none. Either the source view carries one, or you name one. The same scan, two spellings, same result (`030-scans.ano`, `031-scan-along.ano`):

```haskell
+\ Weight @ (↕steps |> route A B)    -- the view is ordered, scan along it
scan(+) Weight along pathCells       -- the order named explicitly
```

The whole family fits one table. Ano adopts q's Greater and Lesser operations directly. `|` is OR on masks and pointwise maximum on numbers. `&` is AND on masks and pointwise minimum on numbers. A mask and number never coerce. `|/` and `&/` are q's folds, and their scans follow from the same dyads. The mask scans are the ever-any and still-all latches. A named reducer's scan comes free (`threat\`, `040-reducer-spellings.ano`).

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

A scan needs no identity. Empty input yields an empty column. The 2026-07-21 ruling keeps boolean OR as `|` and adds numeric maximum to the same dyad. `&` mirrors it as AND and numeric minimum. There is no `||`. `max/`, `max\`, `min/`, and `min\` remain numeric bridges. Steel does not yet implement numeric `|` or numeric `&` in direct, fold, or scan form. That work is pending in `todo/17-greater-lesser.md`. The `min\` bridge, running mean, and running count remain pending in `todo/12-unbuilt-scans.md`.

The parenthesized head of `fold(f)`, `scan(f)`, and `scan2(f)` is any admitted operator or registered reducer, not a one-off allowance for `+`. Like Haskell folds and scans or LINQ Aggregate, the head denotes the accumulator operation. Ano then applies its own laws: an unordered fold still requires associativity, and parallel reassociation still requires commutativity. Steel's long-form parser parity remains pending in `todo/12-unbuilt-scans.md`.

And now the trap this repository has pinned four different ways (`034-fib-stencil-a` through `-d`): the tempting recurrence. You cannot write Fibonacci like this —

```haskell
12 , offset = prev.offset + prev.prev.offset     -- NOT a recurrence: one stencil step
```

— because the comma is a barrier and every read observes pre-state, so `prev` is a parallel shift, not a carry. On a fresh line of zeros that statement computes one shift-and-add over zeros, and iterating it gives you k stencil steps, never the sequence. The honest spelling runs the recurrence inside a registered callable and keys positions off the result:

```haskell
12 , offset = fib(index)
   , spawn Cheese at Player.pos + (offset, 0)    -- continuation: next barrier, sees the committed offset
```

Same shape, opposite verdicts, and the manual's first rule of thumb: if a value must travel along the line you are writing, it belongs in a callable or across ticks, never inside one barrier.

## Order

Grade returns the permutation that sorts. Rank returns positions as values (`demos/4-order`). The distinction is load-bearing: a grade written back into a component would leak which tied row came first, index information a record write must not carry (the tiers chapter says why). So write-backs are `rank(...)`, value-only, ties sharing a rank (`045-rank-ties.ano` pins the tie case):

```haskell
Unit , Slot = rank(Initiative)
```

Top-k selection, one meaning, two spellings, same five entities marked (`041-grade-and-rank.ano`, `042-pipeline-topk.ano`):

```haskell
top 5 (grade desc Threat) , +Targeted
Enemy |> order by Threat desc |> take 5 , +Targeted
```

The fold-prefix form reads like APL, the pipeline reads like a sentence. Pick per audience, the plan is the same. Ordered views compose with everything from the last chapter: `+/ threat @ (Enemy |> order by dps desc |> take 10)` folds over the truncated ordered view (`043-fold-ordered-topk.ano`).

## Generation

Filters cannot invent keys. Only generation mints rows (`demos/5-generate`). `spawn` appends, `at` places, and the replicate `* count` makes each selected source emit that many copies, binding `index` from 0 up to the count per source, every copy reading its own source's columns. That one binding replaces the position-math loop from the introduction's Lua exhibit:

```haskell
Nest , spawn Egg * Fertility at pos + polar(index, index * 137.5)
```

Replicate has a pipeline spelling too, the flat-map exposed (`048-replicate-spawn.ano`, `049-expand-alias.ano`):

```haskell
Spawner , spawn Minion * Count
Spawner |> expand Count , spawn Minion
```

A statement may batch several spawns in one barrier, and keys mint once across the batch: minting two rows in one barrier equals minting one then one, a commutation the wand chapter's `110-cast.ano` pins as arithmetic. Pairwise work is the comprehension, a statement form whose two generators and filters are the source, the theta-join σ_p(A × B) with binders naming each side (`046-outer-product-comprehension.ano`):

```haskell
[ t & c , +InRange | t <- Tower, c <- Creep, dist(t, c) < 50 ]
```

And reshape repositions what already exists. `to` pours an ordered selection into a shape, minting nothing (`050-reshape-positions-*.ano`):

```haskell
Soldier , pos = to 8 8       -- an 8×8 block
Soldier , pos = to 4 _       -- 4 ranks, width inferred
```

Generate when the rows don't exist, reshape when they do. That boundary, not a keyword, is the whole spatial story, coming right up.

## Space

Space is the same calculus with the raggedness removed (`demos/6-space`). There is no spatial keyword. A numeric shape in the source slot is the generator, carrying `x` and `y` per cell, and every regular pattern is just a predicate on those columns (`055-lattice-patterns.ano`):

```haskell
8 8 & (x + y) % 2 == 0 , spawn Wheat     -- checkerboard
8 8 & x == y , spawn Pillar              -- diagonal
8 8 & x + y < 8 , +Buildable             -- triangle
```

A bare `n` is a line, its `index` the same binding replicate taught you, so computed arrangements are index math through callables (`056-computed-line.ano`, `057-spiral-assign.ano`). Folds, scans, grades, and replicates all run at rank 2 unchanged: `+/ Elevation @ 64 64`, `top 8 (grade desc Safety @ 64 64) , spawn Sentry`, `64 64 , spawn Tree * Density`. The one new wrinkle is the two-axis `scan2(+)` for summed-area tables (`059-space-reductions-b.ano`). A `def` over the coordinate columns is a field, and `neighbor(clamp).Height` is the shift pseudo-relation with its boundary policy visible. A shift over pre-state, never a carry. You already know why (`062-derived-fields.ano`):

```haskell
def ridge = sin(x / 8) + sin(y / 8)
def slope = abs(Height - neighbor(clamp).Height)
64 64 & ridge > 0.5 , spawn Peak
```

Two flourishes. A board literal may reshape a glyph string into an anonymous lattice carrying `char` per cell, provided its output-to-input map is explicit. A registered lattice reaches world-space only through a declared affine placement `χ(k) = o + β(k)`; `@` selects a scope and cannot invent that placement. The wand chapter's blast callable must therefore declare its ambient space and linear part before `at Firebolt.pos` can supply an origin.

## Habitats and capabilities

You have now seen the same column operators over several finite row domains. That is the point, but they are not one index read at two ranks. The world contains many habitats: live entities, component-presence sets, fields, relation edges, event batches, and derived query views.

A column expression is `Col X V` over the current query habitat `X`. Layout metadata turns `X` into a dense buffer; lineage maps remember where its rows came from. Equal length is never alignment. A stored field is total on one named habitat. An ECS component is partial on the entity habitat.

Operations ask for structure rather than a tier. A scan needs order. A stencil needs a lattice chart and boundary rule. World placement needs an ambient affine space, origin, and basis map. A fold needs a carrier algebra. Host dispatch needs a registered signature and footprint.

```haskell
+/ Elevation @ Ground
Nord & TwoHanded > 60 , Gold += 1000
Node , OutDeg = +/ Adj@row
Hostile , shortestPath via Navmesh
```

These are all columnar lowerings, but their habitats and admitted operations differ. Reads may create derived domains. Writes return through explicit lineage, so a registered field keeps its habitat, shape, and rank across every tick. Opaque host values remain ordinary columns whose native operation set happens to be empty.

## The grouped fold

One glyph separates two arities, and this chapter is that glyph (`demos/8-gamma`). You know `fold/ col @ scope`: one scalar, always. Put the fold over a tick-marked hop instead and it collapses each fiber to one value per selected source: a column, aligned, written back under the ordinary rule. This is γ, q's `by` and Datalog's grouped aggregation, spelled as fold-under-each with zero new syntax:

```haskell
Pen , Headcount = #/ (livestock' & Cattle)    -- count per pen, over the inverse fiber
Plot , Moisture = avg/ neighbors'.Moisture    -- per-plot mean
Target , Hits += #/ attackers'                -- in-degree, folded at the target
```

Same problem, both arities, side by side. The pair `079-global-vs-gamma-a/b.ano` exists precisely to stop you from confusing them:

```haskell
Cow & Weight < avg/ Weight @ Cow , +Marked    -- @: one scalar, the herd mean, broadcast
Plot , Moisture = avg/ neighbors'.Moisture    -- ': a column, one mean per plot
```

After a fold, `@` yields a scalar and the tick yields a per-source column, lexically, never by registry lookup. `@` never groups. Empty fibers follow the fold-identity law you already know: a pen with no animals keeps Headcount 0 under `#/`, and an `avg/` over an empty fiber drops the row. The farm interlude in the spec (and `076-farm-gamma-*.ano`) runs a nine-line farm on exactly these pieces. Read it now and notice you can parse every line.

While the glyph is fresh, the representation underneath it. A one-to-many relationship is another relation, not a ragged cell: `srel targets 2 4 | | | 4 5 | | | |` (`018-structural-effects.reg`) is one fiber per row — a list of lists in the BQN prototype, CSR in Steel, one offsets column plus one flat edge array (ano-ecs §5). Two flat arrays, array-like all the way down, the same shape as q's nested columns and Arrow's list columns. And the mask algebra never sees multiplicity: `Frenzy.targets'` in source position is the image, bits OR'd into a mask, so an entity reached twice is one bit. The idempotent-scatter law is free because the representation cannot express multiplicity (ano-ecs §4). When multiplicity matters you say so with a fold, and `010-frenzy-image.ano` pins both readings over one fixture:

```haskell
Frenzy.targets' , +Frenzied ; Health -= 10   -- the image: a mask — entity 4, reachable twice, takes the batch once
Target , Hits += #/ attackers'               -- the in-degree: a fold — the same entity 4 counts 2
```

The image is the set; the fold is the count (spec §5, §13). Vectors as cell values exist in the same disciplined form: `col pos vec` holds fixed-arity pairs (`050-reshape-positions-a.reg`), `bind pathCells vec` holds an index sequence (`031-scan-along.reg`). The discipline, stated once: fixed-shape tuples live in vec columns, while ragged, variable-arity data lives behind a relation — named, with fibers, an inverse, and the γ machinery. Both are array-native. Neither leaks raggedness into the mask algebra.

## The Japanese surface

Ano's grammar keeps turning out to be Japanese grammar with the serial numbers left on. `ano_nihongo.md` carries the research and `demos/9-nihongo` the executable examples. The Japanese surface normalizes to shared token kinds and the same parser. Surface noun spellings remain distinct until resolution. Conjugate programs must emit byte-identical BQN.

```text
北 と 両手 六十 より 、 金 に 千 たす
Nord & TwoHanded > 60 , Gold += 1000
```

The particle map is small and shockingly clean: と is `&`, の is the dot hop (`083-dotted-hop.ano`), で is the `@` scope (`084-scoped-selection.ano`), より the comparison, 、 the hinge, たす the `+=`. Kanji numerals lex (`九千九百九十九` is 9999), and counters are typed numerals: `三ヶ月` is `3mo`, three months, and the unit rides the value into the comparison (`092-counters-becoming.ano`):

```haskell
Cheese @ cellar & Aged > 3mo , Price *= 2
```

The deeper claims live in `085-relative-clause`, `093-zero-subject`, `094-block-anaphora`: the relative clause as selection (a full clause before a noun restricts it, no relativizer, which is precisely "the predicate is the entity reference"), the zero subject the effects chapter already used, and the する/なる voice split the next chapter is about to cash in. To run one: the file says `--! ja`, and `steel --tokens` shows you the normalized stream.

## Standing rules and the clock

Everything so far was the する register, the performed voice: do it now. Change one glyph and you get なる, the becoming voice. Same left side, same right side, and the statement installs instead of performing (`demos/10-conways`). Side by side, the exact pair from the spec:

```haskell
Plot & !Planted & #/ (neighbors' & Planted) >= 2 , +Planted                  -- performed: one ring, now
def spread = Plot & !Planted & #/ (neighbors' & Planted) >= 2 => +Planted    -- installed: one ring per tick
```

A standing rule re-gathers against each tick's pre-state and scatters once. No cascading, no fixpoint, one step per tick, which is also the game-legible behavior: crops advance one ring per tick because that is what the rule means. The lineage is Datalog's `head :- body` and the production rules of OPS5 and CLIPS, and the paradigm shipped whole campaigns in StarCraft 2's trigger editor. Ano is that layer with a relational predicate language.

All rules active in a tick share ONE barrier, the `;` law lifted to the rule set, and overlapping writes are admitted three ways, statically: disjoint registered footprints, a merge law (you know these), or complementary guard literals proving the masks row-disjoint. The third is the elegant one. Conway's Life as two rules (`109-life-naru-b.ano`):

```haskell
def kin    = #/ (moore' & Planted)
def bloom  = Plot & !Planted & kin == 3 => +Planted
def wither = Plot & Planted & kin != 2 & kin != 3 => -Planted
```

Both write `Planted` with no common merge family, and a column-level check would reject them. But bloom selects under `!Planted` and wither under `Planted`, so no row of one pre-state can satisfy both, and the emitter reads that complement as the disjointness certificate. The blinker blinks synchronously. A per-rule barrier would break it in either order, and `105-life-step-a/b.ano` and `107-life-glider.ano` pin the one-barrier synchrony from the performed side first.

One honest note about running rules here, since Steel reads a file top to bottom and the real engine has a clock: the harness pretends the clock beats once at the end of each unbroken run of `def` lines, and every rule installed so far fires at each beat. Installs persist, so a rule installed early fires again at a later beat, exactly as the engine would have it. What a file cannot do is let ticks pass without installing something new. `ISSUES.md` states the residue plainly, and the next chapter has a demo that makes the clock beat twice on purpose.

## The wand

The final suite (`demos/11-noita`) takes Noita, a build-a-projectile roguelite whose wands are little programs, and translates it card by card, because a language earns its keep on somebody else's problem. The mapping: a projectile spell is a spawn, a stat modifier is a masked value write, multicast is barrier batching, a formation is replicate's per-copy index, a behavior modifier is a marker component plus a relationship plus a standing rule, the trigger card is a rule turning a hit into a cast, and the material world is the conways lattice under a fuel guard. Wand slot order is program order: the barrier makes each card read the previous card's commit. Walk `110-cast` through `119-wand` and you will recognize every piece from earlier chapters. What follows are the set-pieces where one meaning gets several spellings.

The detonation, spelled twice with one meaning (`116-alchemy.ano`: both lines live in the file, hit the same cells, and since `+Fire` is idempotent the second barrier is the first's echo, the post-state pinning the equivalence):

```haskell
Oil & (x - 6) * (x - 6) + (y - 6) * (y - 6) <= 5 , +Fire     -- raw coordinate arithmetic, nothing else
Oil @ blast(5) at Firebolt.pos , +Fire                       -- the anchored frame
```

The first is column math you could have written after the space chapter. The second is the frame machinery cashing in: `@` fixes the frame, the locative `at` fills its origin, here a mirror-read of the payload's position and in `119-wand.ano` a bound point (`at impact`), and the registered frame fn runs per cell against that origin. Reference by description on the left, and now on the anchor too.

The alchemy pair runs spread and consume in the SAME tick (`117-alchemy-b.ano`): spread rings the oil that consume is removing, both reading one pre-state. That is the shared rule barrier doing visible work, since either sequential order gives a different, wrong world. And `118-alchemy-c.ano` lets the clock beat twice: spread installed alone rings once, a query counts the burning cells, then consume's install brings the second beat, where spread, still installed, rings again while consume burns. Installs persist. The late joiner costs the first ring nothing.

The w-series strips the clock and stress-tests the evaluator: Noita's cast-block algorithm itself becomes column calculus, with no new grammar. The draw is a scan (`w1`: the block's running balance is `1 + +\ (DrawAdd - 1)` along slot order, and `w1-b` pins the recovery trap where `bal >= 0` is the wrong evaluator). The deck wrap is modular index arithmetic (`w2`). Reflection is free because the program is data (`w4`: the Greek-letter cards are one-line selections over the wand's own table). And then the crown jewel: modifier binding, "each modifier binds the next projectile at or after it", the same interleaved wand `[DP, Triple, SB, DP, SB, SB]`, the same damage column out, three spellings:

```haskell
-- w3: the collapsed shortcut — legal only when all modifiers precede all bolts
Spell & Proj & Member & Slot == min/ Slot @ (Spell & Proj & Member) , Damage += 10 * (#/ (Spell & Mod & Member))

-- w3-b: the relation as DATA — an order-derived srel in the registry, its fibers fed to γ
Spell & Proj & Member , Damage += 10 * (#/ (binds' & (Mod & Member)))

-- w3-c: the relation as VALUE — computed in-program, then the column IS the relation
Spell , Bind = scan(min) (Keys + 99 * (1 - Proj)) along revDeck
Spell & Proj & Member , Damage += 10 * (#/ (Bind' & (Mod & Member)))
```

The `.bqn` witness pins the interleaved wand where the shortcut provably dies. `-b` and `-c` both run the general join on it and agree. In `-c`, a key-valued column has exactly a functional relationship's shape, so `Bind'` resolves to its inverse fibers, the value-level rel, and the damage line is character-for-character the grouped fold from `-b` with the data source swapped for a computed one. That is what semantic integrity buys: the relation can arrive as geometry, as registry data, or as a scanned column, and γ does not care.

One last pair, quotation (`128-eval.ano`), the same statement direct and spliced:

```haskell
SparkBolt , Damage += 10
eval "SparkBolt , Damage += 10"
```

`eval` is APL's ⍎ kept on a leash: a literal string, one statement, re-lexed and parsed in place at compile time, so the spliced footprint stays visible to every static check, the rule-conflict analysis included. Dynamic strings are interpreter territory and deliberately not this.

## Where the edges are

You are now great and powerful, so you get the honest map. `ISSUES.md` is short and current: identity across the barrier (a trigger payload's parent can name a row the same barrier despawned; where the intensional/extensional boundary sits is open), recurrences within a statement (you met the trap; the carry stays host-side for now), quotation past the literal (templates with holes, rules writing rules: unbuilt), deriving order-srels mechanically (w3-b's fibers are hand-listed today), world-to-chunk frame conversion (the host does it), frame-homogeneous rule ticks (no mixed entity-and-lattice tick yet), and the harness clock's residue you already know. The spec's "Open Questions, Next Steps" holds the deeper unsettled design: rule retraction, explicit bindings' denotation, the recurrence options, the reap option's granularity, each with its tradeoff stated, never resolved silently.

Ano numbers are IEEE 754 float64. Integers are contiguous through 2^53. Above that, small additions may be absorbed by the local spacing. Overflow and non-finite values are refused at load or save, and a failed save leaves the world file untouched. The registry's value domain is exactly the finite doubles, and load ∘ save is the identity on it, negative zero included. `src/refusals/` pins the refusal boundary. Host float replay needs recorded input bits or an explicit quantization rule; the host binding is unbuilt.

Where to go next: `ano-language.md` is the spec this manual has been quoting. `spatialmaths.md` derives habitats, fields, lineage, lattices, placement, boundaries, and the retired-tier counterexamples. `proofs/foundations.md` states the active obligations, `ano_nihongo.md` carries the Japanese surface, and `demos/` holds the executable witnesses. Now go address something by description.
