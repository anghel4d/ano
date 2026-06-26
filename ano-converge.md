考案: Anghel (Anghel4d)

# ano-converge

Working notes. What falls out of the union of the ano 日本語 particle grammar (see ano_nihongo.md) and the array calculus, read through BQN. Section 1 is the synthesis. Section 2 runs every example in ano-language.md through the convergence so we can cherry-pick and cross-analyze. Nothing here is committed surface yet; the spatial generator's spelling (a numeric shape in the source slot, `↕` for computed shapes) and a few others are decisions flagged inline.

---

## 1. Findings

Holding both systems in view at once: nihongo is head-final, postfix, keyword-free, and assembles meaning by juxtaposition with a fixed particle order; the array calculus is head-final, tacit, keyword-free, and assembles meaning by juxtaposition with a fixed combinator order. They are the same machine. BQN is the bridge specifically because it's the APL that went fully tacit — modifiers, trains, bind, Under — so every Japanese particle has a BQN combinator waiting for it. APL would give you the column arithmetic; BQN gives you the grammar.

The load-bearing correspondences

The comma is Under (⌾). selection , effect means: run the effect in the context of the selection, then put the world back. That is exactly F⌾(mask⊸/) — gather the selected subset, transform it, scatter it home. Nord & TwoHanded>60 , Gold += 1000 is 1000+⌾(sel⊸/) Gold. In Japanese it's the same shape: the relative clause gathers, the predicate transforms, は marks the thing written back to. The comma is the modify-and-restore combinator, and BQN is the only one of the three surfaces where it already has a glyph.

The hop . is Select (⊏). の = ⊏. rel.Comp = Comp⊏˜rel, a foreign-key gather; chained の is chained ⊏. The spec's own APL (skill[mentor]) already says this; it just hasn't named it.

prev/neighbor are shift (» «). This is the §19 answer. A recurrence is not a fib source — it's a shift along the ordering axis under a scan. prev.Height + prev.prev.Height = »Height + »»Height. Fibonacci, diffusion, occlusion, slope are all shift-and-combine. The nihongo file already wrote it this way; BQN says why it's irreducible.

One generator; rank is the habitat. This kills the open question I asked you. ↕n is the entity index line; ↕w‿h is the cell odometer — the same Range primitive at rank 1 vs rank 2. grid 8 8 is ↕8‿8. range, line, fib_line are ↕ at rank 1 plus arithmetic. So "grid stays or collapses" was the wrong fork: it collapses into iota-with-a-shape, and Part V's "two habitats" are one ↕ at two ranks. The generator earns no English name — a numeric shape sitting in the source slot can only mean "iota this shape," so `16 16 & …` is already the generator and ↕ is its optional explicit glyph for when the shape is computed. The rank is the habitat marker; the shape is the whole spelling.

Selection is σ; generation is ↕; one does not reduce to the other

This is the result that makes a spatial keyword incoherent. Selection keeps the keys of an existing column where a predicate holds — Replicate-by-boolean, `/`, closed on the live key-set: its output is always a subset of its input, so it can never mint a key. Generation produces a fresh key-set from a shape with no antecedent — ↕shape, a constructor over no existing set. A filter needs a set to filter and the empty lattice has none, so ↕ ∉ closure(σ): generation is provably irreducible to selection. That forces the split. Acting on cells that already exist is σ over the pos column and carries no new power, so grid-as-selector dissolves into a pos-predicate; minting coordinates that hold nothing is the one job σ cannot do, so ↕-as-generator survives. Keep iota for that job; drop it everywhere a predicate suffices. (Worked end to end in APL and BQN in ano-tmpartifact §2.4.)

pos is an affine column; the frame is its (origin, spacing); the counter is its unit check

A cell key is integer; a world position is real. pos is the column between them, the affine map φ(k) = o + S·k — origin o in world units, spacing S in world-per-cell. With o = 12.0‿8.0 and spacing 1.5 the key (1,2) carries pos ⟨13.5, 11.0⟩: integer keys, real positions, space continuous in the values while the lattice stays discrete in the keys. The frame is exactly (o, S), and @scope picks it — default o is the cursor's raycast hit, @world sets o to the absolute origin, @player to the subject. Read the units: [world] = [world] + [world/cell]·[cell] holds only because S is present, so a bare cell-count dropped into world space with no S is the counter mismatch (三匹 counting a flat thing). The counter-typed numeral is dimensional analysis of φ, and the で/at boundary is where the counter is checked.

こそあど deixis is the alias paradigm and supplies the frame origin

Aliases proliferate freely as long as they carry no semantics of their own. The test: a word is a legitimate alias iff it desugars to the same calculus expression varying only its referent; a word that desugars to a different expression is an operation wearing a noun. @cursor, @player, @world, @observer all desugar to one deictic-resolve(referent) — pronouns, a closed class, keep them all. grid, range, fib, board each desugar to a different expression — operations, opened into the calculus. Japanese is the paradigm: こそあど is one demonstrative engine indexed by one deictic axis (こ proximal / そ medial / あ distal / ど interrogative), これ/それ/あれ/どれ one mechanism resolved by referent. The targeting aliases are that engine. The deictic axis is the frame origin from φ: @cursor is こ proximal, origin at the raycast hit; @player is そ medial, origin at the subject; @world is あ distal, the absolute origin; dore is ど, the query form. The origin o is which demonstrative the selection points with, so the deixis carries the frame at no new semantic cost. The split sets the defaults: selection is address-by-description, distal, あの — the language's namesake — so it defaults to @world; generation is hands-on, proximal, この, so it defaults to @cursor.

The dichotomy ano already encodes

する vs なる is the array calculus's deepest split, named in advance:

- する = value effect = shape-preserving → Under with arithmetic (+⌾sel). Rank in, same rank out.
- なる = structural effect = shape-changing → group / replicate / reshape / append / delete (⊔ / ⥊ ∾ ¬/). The archetype changes because the shape changes.

spawn = replicate/append (∾), despawn ~ = filter-out (¬/), archetype migration = regroup. Japanese handed you the rank-preserving/rank-changing boundary as two ordinary verbs.

The primitive that makes ECS fall out: Group (⊔)

The cross-column join, the archetype index, "which keys appear in which exact column-set" — that is ⊔, Group, a single BQN primitive that buckets a vector by integer key. The whole relational-ECS data model the spec describes is ⊔ over component-signatures. This is the missing piece that turns "array calculus" into "entity-component array calculus" for free.

は = bind, が = the deleted argument

は (ambient topic, carried across clauses) is ⊸ — bind the subject so a run of effects is point-free in it: subject⊸(effect ; effect ; effect). That's your with-block / @cursor open item, resolved. And が — the real subject, constantly invisible as ゼロが — is the tacit argument. A BQN train is a Japanese sentence with every が elided. Point-free programming and zero-pronoun grammar are the same discipline. The spec's "tacit subject" question is just: default が to the ambient は.

Further correspondences, same logic

Each (¨) is pervasion is the listing particle. りんご と みかん と バナナ — と distributes one relation across a heterogeneous list, indifferent to each member's type. That is ¨, element-wise application over a vector. The masked column op is the same move: one operator pervades a selection uniformly.

The て-form is composition (∘ / ○). 食べて寝る, eat-then-sleep sharing a subject, is Sleep∘Eat on one 𝕩 — and at effect scale it's the ;-batch over a shared selection. The native "and then."

こと/もの nominalizers reify a function as a value. こと turns an event into a noun; that is first-class functions/blocks — an effect captured as data, passed, deferred. Pairs with the homoiconic core and the と-quotative (mention vs use) as the quotation boundary a macro reads.

Rotary conjugation is a vowel-indexed gather. 書か/書き/書く/書け/書こ is forms ⊏˜ mood — a base-5 paradigm table indexed by vowel, generated not stored. Grammatical function is an index; the stem is the data; selecting a form is ⊏. Pervades every godan verb uniformly: one operator, rank-polymorphic.

Transitivity pairs are the する/なる split at the root. 開く/開ける, 上がる/上げる — one event, a self-move (なる-side, from ある) and an other-move (する-side). Each verb root yields a value-effect form and a structural-effect form by a regular transform. ano derives the two effect kinds of an operation from one root the same way.

Three predicate terminals are the three predicate kinds. う-verb / だ-noun / い-adjective = action / identity / property. action = a する-effect (verb), identity = the `=` assignment / archetype membership (だ), property = a boolean mask (い-adj). One が-engine, three terminals, no others — and ano's selection engine resolves to the same three.

One logical particle per noun is one structural role per operand. が, を, に, へ, で each mark one role, and a noun carries exactly one per clause; は/も layer over them orthogonally. That is ⊸-bind (scope) layered over a role marker — exactly the SQL discipline where a column takes one of select/where/update per statement, with scope an orthogonal axis.

The hop on the left of = is a lens, and Under is the set. pos.x = grade(Milk) * spacing writes through a .-hop, which is value ⌾ (x‿⊸⊏…) — modify-through-a-Select. So the optic/point-free register the spec lists and the dotted hop are the same object: の-hop = lens, , / = = Under = lens-set.

The two places the union exceeds BQN

This is the part that's genuinely new — where Japanese forces something raw BQN doesn't have:

1. Counter-typed numerals. 三本/三匹/三ヶ月 — a scalar tagged with the frame of what it counts, checked against the noun. BQN numbers are untyped atoms. This is your coordinate-frame open question: a world-space at rejecting a cell-space coordinate is counter disagreement (三匹 can't count a flat thing). The array calculus gives the operations; Japanese supplies a dependent-unit type on the literals that APL/BQN throw away. That's ano's original contribution, and the で/at boundary is where the counter is checked.

2. ⊔-keyed columns as first-class. BQN arrays are anonymous and positional; ano's columns are entity-keyed and registry-named. The registry plus archetype layer is the structure Japanese's noun-with-particle insists on — every value has a role-marker — and that BQN leaves implicit.

So: ano = BQN + counter-typed literals + a ⊔-keyed named-column registry. Everything else — selection, the comma, hops, effects, space, recurrence — is already BQN, and the particles are BQN's tacit combinators spelled as postfix role-markers. The nihongo surface reads as near-perfect because it and BQN are the same head-final tacit machine; ano is their intersection.

Free payoffs (open questions this closes)

- Effect commutativity (;-batch): effects commute iff their Under-write-sets are disjoint. Footprints already record which columns each ⌾ writes — disjoint writes commute, checkably. Not "trust the author."
- Scan ordering: BQN scan runs along the leading axis. Over space the lattice axis is the order; over entities there's no order, so scan requires a grade (⍋) to manufacture one. "implicit along vs error" resolves to: no leading order ⇒ you must supply the grade.
- grid/range: gone — no spatial keyword. A numeric shape in the source slot is the generator (↕-with-shape); the glyph ↕ is the optional explicit form for computed shapes.

Glyph key

↕ Range / odometer (with a shape, the lattice) · ⊏ Select (the hop) · ⊑ Pick · » « Shift before/after (prev / neighbor) · ⌾ Under (modify-and-restore; the comma) · ⊔ Group (bucket by key; the archetype index) · ⊸ ⟜ Bind before/after (は; partial application) · / Replicate / Filter (mask→subset; per-source count) · ∾ Join / append (spawn) · ⥊ Reshape (arrange, board) · ⍋ ⍒ Grade up/down · `` ` `` Scan (\) · ´ Fold (/) · ¨ Each (pervasion; と) · ⌜ Table (outer product; ∘.) · ∘ ○ Compose (て) · ¬ Not (!) · ∧ ∨ And/Or (& |).

---

## 2. Every example, converged

Convention for this section: the converged surface has no spatial keyword. A numeric shape in the source slot is the generator — `16 16 & …` is a 16×16 lattice, `12 , …` a line of twelve — and `↕` is the optional explicit glyph when the shape is computed rather than literal (`↕steps`). `grid` and `range` both retire into this positional rule. `prev`/`neighbor` stay as ASCII hop names in the surface and read as shifts in the machine. Each entry: current ano → converged ano → BQN reading → note. Where a value effect is shown as masked multiply-add (`+↩ v×m`), that is the same write as the Under-scatter `v+⌾(m⊸/)`; the dense form is just more legible.

### Part I — Selection

#### §1 Component masks
```haskell
-- current
Nord , Gold += 100
Bandit & !Dead & Faction == Bandit , Faction = Hostile
Merchant @ cell(Whiterun) , Gold += 5000
```
```haskell
-- converged
Nord , Gold += 100
Bandit & !Dead & Faction == Bandit , Faction = Hostile
Merchant @ Whiterun , Gold += 5000
```
```bqn
Gold +↩ 100×Nord
Faction Hostile¨⌾(sel⊸/) Faction        # sel ← Bandit∧(¬Dead)∧Faction=Bandit
Gold +↩ 5000×(Merchant∧Whiterun)
```
Note: `&`=`∧`, `!`=`¬`, `,`=Under. `@ cell(Whiterun)` → `@ Whiterun` (region is a registered entity, no constructor wrapper).

#### §2 Targeting aliases
```haskell
@cursor , Health = 0
@observer , Faction = Friendly
@selected , Damage *= 2
```
```bqn
Health 0¨⌾(cursor⊸/) Health
Faction Friendly¨⌾(observer⊸/) Faction
Damage ×↩ 2×selected
```
Note: the `@`-aliases are tacit per-eval selections — ゼロが. Under は-binding a run of effects share the alias without renaming (see §10).

#### §3 Pattern-match selectors
```haskell
(Nord, TwoHanded > 60) , Gold += 1000   -- present, value constrained
(Nord, TwoHanded _) , +Trained          -- present, any value
(Nord, !TwoHanded) , +Untrained         -- absent
```
```bqn
# presence is ⊔-bucket membership, resolved once per archetype, before any value read
hasTH ← TwoHanded e∊ archetypeColumns        # ⊔ over component-signature
```
Note: the three presence states are the three relations to the archetype `⊔`-bucket — value-in-bucket, in-bucket-any-value, not-in-bucket. Group precedes gather.

#### §4 Value predicates
```haskell
Gold == 500 , Gold += 100
Gold > Health * 10 , +Greedy
Stamina < Weight , Speed *= 0.5
```
```bqn
Gold +↩ 100×Gold=500
+Greedy : regroup where Gold>Health×10
Speed ×↩ 0.5×Stamina<Weight
```
Note: cross-column comparison is pervasive dyadic over the leading axis — already pure calculus. `+Greedy` is なる (regroup); `*= 0.5` is する (Under).

#### §5 Relationship joins (the dotted hop)
```haskell
Nord & mentor.TwoHanded > 80 , Gold += 1000
Student & mentor.Dead , -Mentored
```
```bqn
Gold +↩ 1000×Nord∧(80<mentor⊏TwoHanded)
-Mentored : regroup where Student∧(mentor⊏Dead)
```
Note: `.` = `⊏`. `mentor.TwoHanded` = `mentor⊏TwoHanded`, foreign-key gather (の). Absent/dangling link → out-of-bounds fill → predicate false. `-Mentored` is なる.

#### §6 Named selections
```haskell
def master = Human & Nord & TwoHanded > 60
master , Gold += 1000
rich , Faction = Hostile ; +Marked
master & rich , +Legendary
```
```bqn
Master ← Human∧Nord∧TwoHanded>60          # a named tacit predicate (a train)
Gold +↩ 1000×Master
```
Note: a `def` mask is a bound train, inlined into the plan at no runtime cost. `master & rich` = `∧` of two masks. `;` = one-barrier run (§10).

### Part II — Effects

#### §7 Value effects (する)
```haskell
Nord , Gold += 1000
Bandit , Health -= 25
Merchant , Gold *= 2
```
```bqn
Gold +↩ 1000×Nord
Health -↩ 25×Bandit
Gold ×↩ 1+Merchant            # ×2 where Merchant
```
Note: する = Under-arithmetic, shape-preserving ⇒ no archetype change. The spec's `gold +← 1000 × nord` is this exact mask form.

#### §8 Assignment (する)
```haskell
Bandit , Faction = Hostile
Dead , Loot = Empty
```
```bqn
Faction Hostile¨⌾(Bandit⊸/) Faction
Loot Empty¨⌾(Dead⊸/) Loot
```
Note: set = Under-replace where the mask holds.

#### §9 Structural effects (なる)
```haskell
Dead , ~                     -- despawn
Frenzy.targets , +Frenzied   -- add component
Burdened , -Encumbered       -- remove component
```
```bqn
world ↩ (¬Dead)/ world                    # despawn = filter-out
world ↩ world ⊔↩ Frenzied @ (Frenzy⊏targets)   # add column → regroup archetype
world ↩ world drop Encumbered → regroup
```
Note: なる = shape-changing. `~` = `¬/`, `+Comp`/`-Comp` = `⊔` regroup, `Frenzy.targets` = `⊏` then add.

#### §10 Sequenced effects (て / one barrier)
```haskell
@cursor , Knockback 5 ; Flash Red ; -Shielded
Nord & TwoHanded > 60 , Gold += 1000 ; +Blessed
```
```haskell
-- the Cortex lineage, restored: terse @-verbs on an ambient selection
@cursor , spawn Marine ; size 10 ; damage 500
```
```bqn
# all three Under the same pre-state; commute iff write-sets disjoint
Knockback:pos  Flash:color  -Shielded:archetype     # disjoint ⇒ commute
```
Note: `;` = `∘`-compose of effects against the pre-state (て-form). は binds `@cursor` as the ambient subject; each verb is ゼロが in it. Commutativity = footprint disjointness. `@spawn marine ; @size 10 ; @damage 500` is this shape verbatim — the real front-door register, not the LINQ pipeline.

### Part III — Column expressions

#### §11 Reduction (`/` = `´`)
```haskell
+/ Gold @ Nord              -- total Nord gold
&/ Alive @ Party            -- whole party alive
#/ (Nord & TwoHanded > 60)  -- count of masters
```
```bqn
+´ Nord/Gold
∧´ Party/Alive
+´ Nord∧TwoHanded>60         # count = sum of the mask
```
Note: `@` = scope = the filter feeding the fold (総和). Unchanged — already the calculus.

#### §12 Scan (`\` = `` ` ``)
```haskell
-- current
+\ Weight @ path(A, B)
max\ Height @ ray(Eye, fwd)
```
```haskell
-- converged: path/ray are ordered iota lines
+\ Weight @ (↕steps |> route A B)
max\ Height @ (Eye + ↕n * fwd)
```
```bqn
+` route/Weight
⌈` (Eye+(↕n)×fwd) ⊏ Height
```
Note: scan runs along the leading axis, so the source must be ordered. `path`/`ray` dissolve into an ordered index line (`↕` + arithmetic) that supplies that order. `max\` is the occlusion sweep.

#### §13 Grade and rank (`⍋ ⍒`)
```haskell
Unit , Slot = grade(Initiative)
top 5 (grade desc Threat) , +Targeted
Enemy |> order by Threat desc |> take 5 , +Targeted   -- alias register
```
```bqn
Slot ↩ ⍋Initiative
5↑⍒Threat
```
Note: `grade`=`⍋⍒`, `top k (grade desc)`=`k↑⍒`. The `|>`/`order by`/`take` pipeline is a transparent alias for `k↑⍒` — legible register, kept as opt-in, not canonical.

#### §14 Outer product (`∘.` = `⌜`)
```haskell
[ t & c , +InRange | t <- Tower, c <- Creep, dist(t, c) < 50 ]
cross dist Tower Creep
```
```bqn
(Tower dist⌜ Creep) < 50      # the pair-table, then a mask
```
Note: comprehension and table are one object (Wadler). `cross` = `⌜` (alias). The filter on the pair is a mask over the table.

#### §15 Replicate (`/` dyadic)
```haskell
Spawner , spawn Minion * Count
Spawner |> expand Count , spawn Minion at pos   -- alias register
```
```bqn
world ∾↩ Minion ⥊˜ Count       # per-source multiplicity, then append
```
Note: `* Count` = `/` replicate (per-source count). `expand` is its alias. `spawn` = `∾`, irreducible. `at pos` redundant when the spawn inherits the source cell.

#### §16 Reshape (`⍴` = `⥊`) — #2: arrange → to
```haskell
-- current
arrange Soldier into grid 8 8
arrange Soldier into ranks 4
arrange Archer into line 20
```
```haskell
-- converged
Soldier , pos = to 8 8
Soldier , pos = to 4 _           -- 4 rows, width inferred
Archer  , pos = to 20            -- one rank
```
```bqn
pos ↩ 8‿8 ⥊ Soldier
pos ↩ 4‿∘ ⥊ Soldier              # ∘ = inferred axis
pos ↩ 20 ⥊ Archer
```
Note: `arrange … into …` is `⥊` with English on top — a pure alias, cut. The op is spelled `to` (the allative へ); `Soldier , pos = to 8 8` was already the converged form.

#### §17 Named column transforms (`def` = a train)
```haskell
def threat = Damage * Speed / Range
Enemy , Priority = threat
Enemy & threat > 100 , +Dangerous
+/ threat @ (order by dps desc |> take 10)
```
```bqn
Threat ← Damage×Speed÷Range       # a fork; cf. Mean ← +´÷≠
Priority ↩ Threat
+´ Threat ⊏˜ 10↑⍒DPS
```
Note: a `def` whose body is number-over-entities is a derived column, first-class wherever a raw column is. This is exactly the derive-don't-name discipline Part IV needs.

### Part IV — Space (one `↕`, rank 2)

Two operations live here and they do not reduce to each other. Selection filters existing keys (`/`, closed on the live key-set); generation mints fresh keys (`↕shape`, a constructor with no antecedent). A filter needs a set to filter and the empty lattice has none, so generation is irreducible — which is why a bare numeric shape survives in the source slot as the generator while every grid-as-selector collapses to a pos-predicate. Positions are the affine column `pos = φ(k) = o + S·k`, with the frame `(o, S)` set by `@scope` (default: the cursor's raycast). Full derivation in ano-tmpartifact §2.4–2.5.

```bqn
# selection: filter existing keys       generation: mint keys that don't exist
   sel ← mask / keys                     cells ← ↕16‿16
   # ↕ ∉ closure(/) : a filter can never invent a key
```
```apl
⍝ selection                              ⍝ generation  (⎕IO←0)
      sel ← mask / keys                  cells ← ⍳16 16
```

#### §18 Patterns from the coordinate lattice — #1: grid → a shape in the source slot
```haskell
-- current
grid 8 8 & (x + y) % 2 == 0 , spawn Wheat  at (x, y)   -- checkerboard
grid 8 8 & x == y , spawn Pillar at (x, y)              -- diagonal
grid 8 8 & x + y < 8 , +Buildable                       -- triangle
```
```haskell
-- converged: shape in the source slot is the generator; cell carries x,y so `at (x,y)` is dropped
8 8 & (x + y) % 2 == 0 , spawn Wheat   -- checkerboard
8 8 & x == y , spawn Pillar             -- diagonal
8 8 & x + y < 8 , +Buildable            -- triangle
```
```bqn
2|+⌜˜↕8        # checkerboard: parity of coordinate sums
=⌜˜↕8         # diagonal
≤⌜˜↕8         # lower-triangular
```
Note: `grid w h` = `↕w‿h`, written as the bare shape `w h` in the source slot. The pattern is a dyadic `⌜`-table over the index vector — the APL above said this already. `spawn` keeps; `at (x,y)` redundant (cell position is implicit, like なる).

#### §19 Generation along a computed lattice — #1 headline: fib dissolves
```haskell
-- current
fib 12 , spawn Cheese at Player.pos + (n, 0)                 -- fib gaps
fib 12 , spawn Cheese at Player.pos + polar(n, n * 137.5)    -- spiral
range 8 , spawn Pillar at Player.pos + (n * 2, 0)
arrange Coin onto spiral(137.5)
```
```haskell
-- converged: fib is a recurrence — a shift along the line; spiral is iota→polar
12 , offset = prev.offset + prev.prev.offset
   , spawn Cheese at Player.pos + (offset, 0)
12 , spawn Cheese at Player.pos + polar(index, index * 137.5)
8  , spawn Pillar at Player.pos + (index * 2, 0)
Coin , pos = Player.pos + polar(index, index * 137.5)        -- arrange-onto = assign-from-iota
```
```bqn
o ↩ »o + »»o                       # fib recurrence = shift+add along the line (seed 0‿1)
pos ↩ Player.pos + Polar (↕12) ×⟜137.5⊸… index
```
Note: this is the cut #1 was about. `fib n` becomes a two-back shift (`»`/`»»`) under the line's order; the bare numeric shape is `↕n`; the spiral is `↕`→`polar`; `arrange onto` is reshape/assign from the iota. No named generator survives.

#### §20 Reduction and scan over space
```haskell
-- current
+/ Elevation @ grid 64 64
max/ Threat @ region(Frontier)
max\ Height @ ray(Eye, north)
```
```haskell
-- converged
+/ Elevation @ 64 64
max/ Threat @ Frontier
max\ Height @ (Eye + ↕n * north)
```
```bqn
+´ ⥊ Elevation                     # ravel then fold
⌈´ Frontier/Threat
⌈` (Eye+(↕n)×north) ⊏ Height       # occlusion sweep along the ray
```
Note: `grid` → the bare shape `64 64`; `region(Frontier)` → `@ Frontier` (named scope); `ray` → an ordered iota line `↕n`.

#### §21 Grade over space (best cells)
```haskell
-- current
top 8 (grade desc Safety @ grid 64 64) , spawn Sentry at pos
grid 64 64 |> order by Resource desc |> take 5 , +MiningNode
```
```haskell
-- converged
top 8 (grade desc Safety @ 64 64) , spawn Sentry
64 64 & top 5 (grade desc Resource) , +MiningNode
```
```bqn
8↑⍒Safety
5↑⍒Resource
```
Note: `grid` → the bare shape `64 64`; `top k (grade desc)`=`k↑⍒`; pipeline = alias; `at pos` dropped (selected cells already carry position).

#### §22 Replicate over space (density fields)
```haskell
64 64 , spawn Tree * Density
64 64 & Fertility > 0 , spawn Crop * Fertility
```
```bqn
world ∾↩ Tree ⥊˜ Density           # per-cell count from a field, then append
```
Note: `* Density` = `/` replicate; `spawn` = `∾`; `grid` → the bare shape `64 64`.

#### §23 Named fields (`def` over coordinates) — the #1 template done right
```haskell
def ridge = sin(x / 8) + sin(y / 8)
def basin = ridge < 0
def slope = abs(Height - neighbor.Height)   -- neighbor = shift
64 64 & ridge > 0.5 , spawn Peak
64 64 & basin , Water = 100
64 64 & slope > 30 , +Cliff
```
```bqn
Ridge ← (1⊸○÷⟜8)⊸+○… (sin of x/8 plus sin of y/8)
Slope ← |Height - «Height                   # neighbor = shift
```
Note: already the derive-don't-name model. A field is a train over `x y`; `neighbor.Height` is `«Height`/`»Height`. Only the spatial source (now the bare shape `64 64`) and the dropped `at (x,y)` change.

#### §24 Board literal — #1: board → to
```haskell
-- current
board "RNBQKBNR/PPPPPPPP/......../......../......../......../pppppppp/rnbqkbnr"
  , spawn (pieceOf char) at (x, y)
```
```haskell
-- converged
"RNBQKBNR/PPPPPPPP/......../......../......../......../pppppppp/rnbqkbnr"
  to 8 8 , spawn (pieceOf char)
```
```bqn
8‿8 ⥊ rows                          # ⍉ 8 8 ⍴ glyphs, as the APL above said
PieceOf¨ board
```
Note: `board` = `⥊` of a glyph string — a pure alias. The cell carries `char`; `pieceOf` maps it to a spawn type; `at (x,y)` implicit.

### Part V — the two habitats become one `↕` at two ranks

```haskell
-- current framing: two habitats
              over entities                 over space
reduce   +/ Gold @ Nord              +/ Elevation @ grid 64 64
...
```
```haskell
-- converged framing: one generator, rank = habitat
              rank 1 (entities)            rank 2 (space)
generator  n   (↕n)                     w h   (↕ w‿h)
reduce     +/ Gold @ Nord             +/ Elevation @ 64 64
scan       +\ Damage @ graded         +\ Cost @ 64 64
grade      top 5 (grade Threat)       top 8 (grade Safety @ 64 64)
table      a dist⌜ b                  pattern ⌜ over ↕
replicate  spawn Minion * Count       spawn Tree * Density
reshape    to Unit                    to Soldier 4 _
recurrence prev.X   (shift »)         neighbor.X  (shift «/»)
def        def threat = Dmg*Spd/Rng   def ridge = sin(x/8)+sin(y/8)
```
Note: over entities the leading axis is the entity key (ragged, `⊔`-bucketed); over space it's the cell index (dense). Same `↕`, rank 1 vs rank 2. The grammar `source & predicate , effect` is `mask ⌾ source` at both ranks. The table isn't two columns — it's one column read at two ranks.

### A farm interlude, converged
```haskell
16 16 & (x + y) % 2 == 0 , spawn Wheat                      -- checkerboard field
Pen , Headcount = #/ (livestock & Cattle)                   -- count cattle per pen
Cow & Weight < avg/ Weight @ Cow , +Marked                  -- below-average weight
Cow , pos.x = grade(Milk) * spacing                         -- line the herd by yield
Plot & !Planted & #/(neighbors & Planted) >= 2 , +Planted   -- crops spread (stencil)
Plot , Moisture = avg/ Moisture @ neighbors                 -- irrigation diffusion
Crop & Growth >= 100 , spawn Produce ; ~                    -- harvest the ripe
Farm & Acreage < avg/ Acreage @ Farm , Gold += 500          -- subsidy to small holdings
Gold , Gold = Gold * 1.05                                   -- 5% interest, world-wide
```
```bqn
Headcount ↩ +´ Cattle/livestock
+Marked : Weight < (+´÷≠) Cow/Weight          # avg/ = the +´÷≠ fork
pos.x ↩ (⍋Milk) × spacing
+Planted : (¬Planted) ∧ 2≤ +´ neighbors∧Planted   # neighbors = shift-stencil
Moisture ↩ (+´÷≠) neighbor⊏Moisture
world ∾↩ Produce ⋄ world ↩ (¬ripe)/world          # spawn then despawn
Gold ×↩ 1.05
```
Note: the showcase — `#/`=`+´`, `avg/`=`+´÷≠` (a fork), `grade`=`⍋`, `neighbors`=shift-stencil, `spawn … ; ~`=append-then-filter. `grid` → the bare shape `16 16`; `at (x,y)` dropped. Everything here is the calculus already; convergence only drops the spatial keyword for a bare shape and drops the redundant position.

---

## 3. Space: three operations (working)

Still being worked out. Part IV reduces to three array primitives, told apart by one question: do the operands already exist?

| intent | operation | glyph (APL · BQN) | surface |
|---|---|---|---|
| keep some of what exists | filter / select | `/` · `/` | a predicate (`Fertility > 0`) |
| mint a lattice where nothing exists yet | generate | `⍳` · `↕` | a bare numeric shape in the source slot (`8 8 & …`) |
| fold data you already hold into a shape | reshape | `⍴` · `⥊` | `to` (`pos = to 8 8`, `"…" to 8 8`) |

Existence decides generate vs reshape. A checkerboard on empty ground mints cells; arranging units you already have, or a board string of glyphs you already have, folds existing data. Both take a shape, so the surface reads alike; §2.4 separates them — generation has no antecedent set, reshape operates on one.

The seam I tripped on: "arrange Soldier into 8 8" is reshape (`8‿8 ⥊ army`), the units already exist. The checkerboard is generation, the cells do not. Same `8 8`, opposite operation, decided by existence.

Open here: the reshape spelling (`to`, the allative へ, versus the `⥊` glyph as an explicit-register twin that telegraphs the cyclic-fill behaviour), and whether `to` (allative, a shape) and `at` (locative, a point) stay two markers or collapse to one goal particle. Minimal way to see all three in a BQN REPL: `↕8` generate, `3‿3 ⥊ "abcdefghi"` reshape, `(0=2|↕8)/↕8` filter.

---

## Cherry-pick checklist

Decisions baked into Section 2, each reversible:
- `grid`/`range` retired — a numeric shape in the source slot is the generator (rank = arity), `↕` the glyph for computed shapes. The biggest surface change, and the algebra forces it: selection is σ (closed on live keys), generation is ↕ (outside σ), so the lattice keyword has no coherent single meaning. Alternative: keep a spelled name for the rank-2 lattice as opt-in sugar.
- Drop `at (x,y)` whenever the spawn target is the current/selected cell. Keep it only when spawning at a computed position different from the cell.
- `fib`/`board`/`path`/`ray`/`region(...)` dissolved into `↕`+arithmetic, shifts, reshape, and named scopes. Confirm none carried meaning beyond the calculus.
- `arrange` → `to` (the reshape op `⥊`, spelled as the allative へ); `spawn`/`~` kept. Confirm `spawn` is the only irreducible structural verb you want spelled.
- Pipeline / `order by` / `into` / `expand` left as opt-in alias register, symbolic form canonical. Decide whether they stay in the spec at all.
- Restore the real Cortex front-door register at §10 (`@verb arg ; …`), fix the Lineage table row, credit BQN for the tacit grammar.
