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

## Current implementation

This reference describes the current Steel/Kore implementation. The author's WIP [tour](../tour.md) and research notes are separate from executable contracts. The registry supplies names and callables; the examples below assume matching declarations.

Steel emits BQN and executes it through CBQN. General nominal spatial types, a native Ano JIT, seeded folds, fallible traversal, and mission manifests are not implemented. [Implementation limits](ISSUES.md) records the current gaps.

## Part I: Selection

### 1. Component masks

```haskell
Nord & TwoHanded > 60 , Gold += 1000
```

The left side selects rows; the right side applies an effect. A bare Boolean column supplies its mask. Presence and validity constrain value reads. Numeric comparisons produce masks. `&`, `|`, and `!` combine or negate masks.

`Merchant @ Whiterun` and `Merchant & Whiterun` have the same mask lowering. `@` has higher precedence and also has special fold, row, and spatial-scope forms; it is not universally interchangeable with `&`.

### 2. Targeting aliases

```haskell
^cursor , Health = 0
Player , Gold += 1000
```

`^name` first consults the dynamic alias overlay, then falls back to bare `name` only when that alias is absent. A present but stale alias refuses. Installing, rebinding, or deleting an alias does not change the bare declaration.

The host freezes the alias environment and resolver input for execution. Changes take effect at a later execution boundary, not halfway through one statement. The concrete resolver contracts are `deictic.cursor`, `deictic.observer`, `deictic.selected`, and `deictic.world`. They consume host input; Steel does not perform a game-engine raycast itself.

Spelling aliases (`as`, `ja`), stored registry `alias` masks, and the dynamic overlay are separate mechanisms. See [the registry](ano-registry.md).

### 3. Pattern-match selectors

```haskell
(Nord, TwoHanded > 60) , Gold += 1000
(Nord, TwoHanded _) , +Trained
(Nord, !TwoHanded) , +Untrained
```

These select a present constrained value, any present value, or an absent component. Absence is not a numeric or symbol value.

### 4. Value predicates

```haskell
Gold == 500 , Gold += 100
Gold > Health * 10 , +Greedy
```

`=` and `==` compare in a selection. Effect assignment uses `=`; comparisons inside an effect's expression use `==`. `:Name` is a symbol literal. A bare name resolves as a name, never as an implicit symbol.

### 5. Relationship joins

```haskell
Nord & mentor.TwoHanded > 80 , Gold += 1000
Frenzy.targets' , +Frenzied
Pen , Headcount = #/ (livestock' & Cattle)
```

`rel.Comp` follows a functional relationship and reads the target column. A bare relationship mask uses the same foundness test. `-1` is the silent no-link sentinel. An admissible missing target fails foundness; malformed endpoint carriers refuse.

Predicate conjunction does not short-circuit row by row. In `Enemy & Owner.Gold > 10`, the Owner crossing is evaluated on the predicate's source domain even where Enemy is false. A crossing used by the effect runs on the selected domain instead. Trace records preserve those separate uses; see [the trace protocol](../src/trace/trace.md).

A keyed relationship resolves against a declared unique column. An unkeyed relationship uses the legacy row-index key space. Unique means injective, not nonnegative; its declared carrier determines allowed values.

The tick `'` exposes set-valued or inverse fibers. In a source image, reached targets are deduplicated into a mask. Under a fold, each source's fiber is reduced separately. A bare fiber cannot be used as an ordinary flat value.

### 6. Named selections

```haskell
def master = Human & Nord & TwoHanded > 60
master , Gold += 1000
```

`def` can name a selection or a derived expression. It does not create a persistent captured selection handle.

## Part II: Effects

### 7. Value effects

`+=`, `-=`, `*=`, and `/=` update selected rows. Expressions read the statement's pre-state. The destination carrier and range govern publication; runtime promotion is not storage subtyping.

### 8. Assignment

```haskell
Bandit , Faction = :Hostile
```

Assignment writes through the selected row domain. Validity guards can remove rows from a write. Product-valued results and mismatched scan witnesses cannot be assigned merely because their buffers have the same length.

### 9. Structural effects

```haskell
Burdened , -Encumbered
Nord , +Blessed
Dead , ~
```

`+Component` adds presence, `-Component` removes it, and `~` despawns. Spawn fills from the prototype, then the column default, then the carrier zero. Unique columns mint fresh values.

These are the Rust registry and emitted BQN operations. They do not imply an implemented archetype/chunk or generational allocator.

### 10. Sequenced effects

```haskell
Nord , Gold += 1000 ; +Blessed
Nord & Dead , spawn Ghost
~
```

`;` batches effects in one barrier. Their reads use the same pre-state. Conflicting writes require a supported merge law; incompatible writes refuse rather than becoming last-writer-wins.

A subsequent statement is another barrier. A leading comma, lone `~`, or omitted-subject effect can reuse the saved antecedent mask. It reuses the selection, not the old world. A cold omitted-subject effect uses `^cursor`; a cold explicit continuation refuses.

### 11. Standing rules

```haskell
def mark = Nord & !Blessed => +Blessed
undef mark
```

`=>` installs a standing rule. Named retraction is a top-level control operation, not an effect. Duplicate live names and unknown retractions refuse.

Rules scheduled together read one pre-state. Writes must have disjoint footprints, a supported merge, or guards proving row disjointness. There is no fixpoint evaluation.

The standalone emitter schedules rule steps around fresh installation runs. Kore advances its staged program with `n`. Neither implements the proposed mission clock or history services. See [time](ano-time.md).

## Part III: Column expressions

### 12. Reduction (`/`)

```haskell
+/ Gold @ Nord
max/ Threat @ Frontier
fold(-) Weight @ Route
```

A scoped-global fold produces one scalar when it has a result. An identityless empty fold produces no result row: no printed placeholder and no assignment.

For input 10, 3, 2, a subtraction fold computes (10 − 3) − 2 = 5. The corresponding scan returns 10, 7, 5. A right fold would give a different result, so traversal order is part of this operation's meaning.

Current reductions use exact left accumulation. A homogeneous operation starts with the first element and applies its step to the accumulator and each remaining element. Descriptor laws may admit other strategies, but Steel does not currently select regrouping or reordering.

| Head | Meaning | Empty fold |
|---|---|---|
| `+` | Sum | 0 |
| `*` | Product | 1 |
| `#` | Count admitted elements | 0 |
| `&` | All on masks; minimum on numbers/chars | True on masks; otherwise no row |
| `\|` | Any on masks; maximum on numbers/chars | False on masks; otherwise no row |
| `min`, `max` | Numeric/char extrema bridges | No row |
| `avg` | Sum/count, then divide | No row |
| `-`, `fold(/)` | Left subtraction/division | No row |

`fold(f)` and `f/` resolve through the same operation table. The lexer does not admit `//` for division reduction. Registered homogeneous reducers must satisfy the current carrier and callable contracts.

There is no explicit seed argument. That remains an open decision in [task 06](../todo/06-seeded-fold-and-traverse.md).

### 13. Grouped fold

```haskell
Pen , Headcount = #/ (livestock' & Cattle)
Plot , Moisture = avg/ neighbors'.Moisture
```

A fold over `rel'` reduces each fiber to a per-source result. Empty fibers use the operation's identity or lose their result row. Count counts admitted elements, not their numeric payload.

`f/ col @ mask` is scoped-global. `f/ rel'.col` is per-source. The legacy `rel @ row` form has its own row-fiber lowering. Named reducers over fibers and per-fiber scans are currently refused.

### 14. Scan (`\`)

```haskell
+\ Weight @ Route
scan(-) Weight along pathCells
avg\ Weight @ Route
```

A homogeneous scan emits the first element and every later left accumulator, preserving input length. Empty input gives an empty column, with no extra seed row.

For input 2, 4, 9, count emits 1, 2, 3, while average emits 2, 3, 5. Each average prefix is computed from the accumulated sum and count; taking the average of the previous average and the next value would produce a different operation.

Average carries sum and count and emits their quotient. Count carries a prefix count. These are stateful prefix machines, not homogeneous reducers over the output carrier.

`along` supplies the traversal view. Scan assignment retains and validates its world-row witness. Duplicate, omitted, or foreign rows refuse; scan reads and effect writes must also have disjoint footprints.

### 15. Grade and rank

```haskell
Unit , Slot = rank(Initiative)
Enemy |> order by Threat desc |> take 5 , +Targeted
top 5 (grade desc Threat) , +Targeted
```

Grade orders a view; rank produces values. The current dense-rank form gives tied values the same rank. Ordered views retain row identity for later effects. `rank` is a resolved built-in form, not one of the lexer keywords.

### 16. Outer product

```haskell
cross dist Tower Creep
[ t & c , +InRange | t <- Tower, c <- Creep, dist(t, c) < 50 ]
```

`cross` materializes a rank-2 product value. Its product lineage does not become entity lineage through a pure wrapper, so world-column assignment refuses. The double-generator comprehension selects pair rows and applies effects to the admitted image.

The callable and every name must be declared. This syntax does not provide a generic distance function.

### 17. Replicate

```haskell
Spawner , spawn Minion * Count
Spawner |> expand Count , spawn Minion
```

Each selected source contributes its copy count. A copy retains its source and a zero-based copy `index`. The two forms use that same source/copy relationship.

### 18. Reshape

The current `pos = to shape` operation pours coordinates into a legacy position column. It is not a general first-class value reshape or proof of a nominal spatial frame. Shape and spatial compatibility remain limited by the current emitter.

### 19. Named column transforms

```haskell
def threat = Damage * Speed
Enemy , Priority = threat
+/ threat @ Enemy
```

A named expression can be reused in selections, queries, and effects where its carrier and row domain are valid. It cannot hide an invalid domain or footprint.

Arithmetic converts masks and chars to numbers. Greater/Lesser joins mask, number, and char upward. Comparisons use numeric values or code points and return masks; symbols have same-carrier equality/inequality. Typed signatures and storage destinations remain exact.

## Part IV: Current spatial boundary

The code has a legacy singleton `lattice w h`, not a general registry of nominal habitats or frames. The following sections identify existing syntax without claiming task 99 is implemented. Quarantined spatial demos do not establish support.

### 20. Patterns from a coordinate lattice

One- and two-dimensional shapes and conventional coordinate names are recognized by the current parser/emitter. They do not establish persistent named habitat identity.

### 21. Derived lines and computed placement

Numeric sources and per-copy indices support finite generation. Position expressions depend on registered values/callables and legacy positional conventions. There is no certified player-plane or support-projector API.

### 22. Reduction and scan

The operations use the same reducer table described in sections 12 and 14. A legacy field domain does not grant arbitrary-rank or cross-habitat alignment.

### 23. Grade

Ordered spatial views still depend on the legacy spatial representation. They do not supply the nominal destination certificates proposed for task 99.

### 24. Replicate and spawn

General replication is implemented. The exact-51 supported spatial spawn described in the Lean model is not constructed by Steel. A parsed placement expression does not prove valid support geometry.

### 25. Fields, neighborhoods, and boundaries

Fields share the registry's one lattice width and height. The built-in neighbor path uses a clamp helper; alternate boundary spellings are not distinct implemented policies. Explicit stored relationship fibers carry their declared edges.

### 26. Board literals and value reshape

Literal/shape generation uses legacy parser and emitter paths. It does not implement a new nominal habitat, arbitrary-rank reshape, or a certified placement. General value-reshape spelling remains a design question.

## Part V: Domains and storage

The emitter distinguishes scalars, world/selected values, copies, and product or ordered row witnesses. These checks reject specific invalid combinations. They are not the proposed universal `Col<X,V>` IR.

`Registry` stores ordered entry vectors and one legacy lattice. Set-valued relationships are nested vectors. Current storage is documented in [ano-ecs.md](ano-ecs.md); declaration syntax and schema replacement in [ano-registry.md](ano-registry.md).

## Numeric values

`num` admits finite float64 values and signed infinities. NaN is refused at load and publication. Both machine zero signs canonicalize to Ano zero. Refined Boolean, natural, and integer carriers are finite, with their declared range checks.

An admitted infinity is not an empty-extrema identity. A failed publication does not turn an invalid numeric result into a value.

## Quotation

`eval "…"` is recognized at statement position and splices one literal statement at parse time. Runtime strings, interpolation, and a first-class code carrier are not supported. `eval` is not a general-purpose interpreter or one of the lexer keywords.

## Appendix: Grammar

The authoritative tables and parser are [lex.rs](../steel/src/lex.rs), [parse.rs](../steel/src/parse.rs), and [lib.rs](../steel/src/lib.rs). [ano-keywords.md](ano-keywords.md) lists tokens and context-sensitive forms.

The main precedence order is mask OR, mask AND, negation, comparison, fold/scan prefixes, addition/subtraction, multiplication/division/modulo, scope, then hops/atoms. Parenthesize compound fold scopes. Assignment and control forms are parsed by their statement context rather than being ordinary value operators.

Both readers produce the same token kinds and parser input. Japanese noun spellings survive until resolution; paired active demos check equivalent emission. See [ano_nihongo.md](ano_nihongo.md).

## Open Questions, Next Steps

[Task 06](../todo/06-seeded-fold-and-traverse.md) preserves the seeded-fold and fallible-traversal decisions. [Task 99](../todo/99-spatial-lattice.md) owns the spatial contract and its implementation-path choice. Neither is silently settled by this reference.

When sources disagree, preserve the author's rulings and identify whether the conflict concerns a design requirement or implementation status. Current code and tests establish what runs; an unimplemented ruling remains an obligation, not a feature to advertise or a decision to erase. Old progress annotations in the read-only ledger are historical; the live task index records what has since landed.

The [historical rulings](../todo/00-historical-rulings.md) retain the author's decisions. Lean theorems establish their stated abstract contracts, not that Steel constructs the required evidence. [proofs/lean.md](../proofs/lean.md) states that boundary.
