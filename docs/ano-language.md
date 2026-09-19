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

## Language contract and implementation

This reference specifies Ano's language semantics and identifies gaps in the current Steel/Kore implementation. Steel/Kore implement sequential effect composition with `|>` as well as simultaneous batches with `;`. The author's WIP [tour](../tour.md) introduces the language; research notes remain separate from executable contracts. The registry supplies names and callables; the examples below assume matching declarations.

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
Frenzy.targets , +Frenzied
Pen , Headcount = #/ (livestock & Cattle)
```

`rel.Comp` follows a functional relationship and reads the target column. A bare relationship mask uses the same foundness test. `-1` is the silent no-link sentinel. An admissible missing target fails foundness; malformed endpoint carriers refuse.

Predicate conjunction does not short-circuit row by row. In `Enemy & Owner.Gold > 10`, the Owner crossing is evaluated on the predicate's source domain even where Enemy is false. A crossing used by the effect runs on the selected domain instead. Trace records preserve those separate uses; see [the trace protocol](../src/trace/trace.md).

A keyed relationship resolves against a declared unique column. An unkeyed relationship uses the legacy row-index key space. Unique means injective, not nonnegative; its declared carrier determines allowed values.

Declared set-valued relationships need no suffix. `Frenzy.targets` selects the union of the selected fighters' targets; a target reached more than once still receives the effect once. `livestock.Health` produces one group of animal health values per owner, and `+/ livestock.Health` reduces each group separately. A bare set-valued relationship can be queried as groups of entity identities; it cannot be scattered into a scalar column.

#### Prime: relational converse

Read the apostrophe `'` as **prime**, as in the notation `f'`. Its current meaning is relational converse: reverse every live edge. If `R` relates source `a` to target `b`, `R'` relates `b` to `a`. It is an expression-local relation, recomputed from the incoming state of its stage; it neither writes the world nor introduces a hidden stored column. This use of prime belongs to Ano's special relational algebra. It does not mean calculus differentiation.

For a functional relationship `parent` with targets `-1 0 0 1 1`, the edges are `1→0, 2→0, 3→1, 4→1`. Its converse groups are `(1,2), (3,4), (), (), ()`: the children of each row. If the registry declares `inv children parent`, `children` and `parent'` denote the same relation and both observe later changes to parent.

```haskell
parent.Gold
parent'
Selected.parent
Selected.parent'
#/ parent'
+/ parent'.Gold
def childrenOf = parent'
Selected , Out = #/ childrenOf
```

The first expression reads each row's parent's Gold. The next queries the reverse groups. The two images select the selected rows' parents and children respectively. The folds count children and sum their Gold per parent. Every noun in these examples must have a registry declaration or an explicit definition.

Prime composes: `R'' = R`, `R''' = R'`, and `R'''' = R` as relations. These are semantic equalities, not additional assignment or relationship-comparison syntax. Ordinary functional values retain their original scalar-key representation after a pair of primes; set-valued relationships retain their grouped representation. A malformed operand is still refused under two primes.

A dot composes declared relations in traversal order. Prime binds to the immediately preceding leg: `A.B'` means follow A, then converse B. Parentheses select the whole composition: `(A.B)'` means converse the two-leg relation, equal as an edge set to `B'.A'`. Names, spelling aliases, dynamic aliases that resolve to relationships, and definitions of relational expressions admit prime. Both readers accept repeated primes; Japanese also accepts the space-separated form `辺 ' '`.

Declared relations use their own endpoint key spaces. A numeric stored key column also admits prime: `Bind'` groups sources by matching their present Bind value to the world's canonical identity column (role id, then role keys, otherwise positional identities). This retains the existing computed-key inverse use. A numeric expression such as `(Gold + 1)'`, a Boolean mask, an ordinary callable result, and a projected value group are not declared relations and refuse prime. Numeric-column double prime restores that column's original value representation.

Relational groups are sets of live edges: duplicate endpoints contribute once, dead targets contribute no edge, and projected absent values are omitted before reduction. Members are visited in current world-row order, giving ordered folds a deterministic traversal. Functional `-1` remains silent absence; malformed endpoints still refuse. Converse scans the original source domain for incoming edges, so its dead-link trace records retain SOURCE domain even when the resulting owners are selected by an effect.

Unkeyed positional edges retain their original row identity within an execution after despawn. Saving rebases live positional endpoints into the saved world's row order, drops dead set members, and stores a dead functional endpoint as `-1`; reopening cannot reinterpret a removed row's offset as another survivor. Use a declared unique key when endpoint numbers must remain stable across saves.

Grouped arithmetic, direct group assignment to a scalar column, grouped scans, and named reducers over groups are refused rather than implicitly flattened. Project an ordinary column and use a supported grouped fold to obtain a scalar value per owner.

`/// !TODO: Rework and review the special relational algebra as a whole: relation carriers and domains, composition, grouping, ordering, callable boundaries, and its connection to expression-local tuples/generators/guards/comprehensions. The current executable contract is prime = converse, including R'' = R; this TODO does not suspend that contract.`

Migration from the earlier implementation: remove primes that merely exposed an already declared set-valued relation, such as `targets'`, `neighbors'`, or `livestock'`. Those forms now reverse their relations. Keep `Bind'` when the intent is inverse lookup from a numeric key column. The active ASCII and Japanese examples use the new contract.

### 6. Named selections

```haskell
def master = Human & Nord & TwoHanded > 60
master , Gold += 1000
```

`def` can name a selection or a derived expression. It does not create a persistent captured selection handle.

## Part II: Effects

### 7. Value effects

`+=`, `-=`, `*=`, and `/=` update selected rows. Expressions read the incoming state of their effect stage. Effects composed with `;` share that state; a stage after `|>` reads the preceding stage's result (section 10). The destination carrier and range govern publication; runtime promotion is not storage subtyping.

### 8. Assignment

```haskell
Bandit , Faction = :Hostile
```

Assignment writes through the selected row domain. Validity guards can remove rows from a write. Product-valued results and mismatched scan witnesses cannot be assigned merely because their buffers have the same length.

Several columns can be assigned together with an n-tuple. Arity is not restricted to two:

```haskell
Nord , (Gold, Silver, Copper) = (4, 51, 13)
Nord , (Gold, Silver, Copper) = (Silver, Copper, Gold)
```

The first statement gives each selected row Gold 4, Silver 51, and Copper 13. The second rotates those values: Gold becomes 51, Silver 13, and Copper 4. Every RHS member reads the same incoming state; the tuple publishes its component writes together. There is no right-to-left assignment chain.

Targets and values must be explicit tuples of matching arity and nesting. Each target leaf names a distinct registered column; aliases for the same column count as the same destination. Each value leaf is an ordinary expression checked against its own destination carrier. Mixed carriers and nested tuples are allowed:

```haskell
Nord , (Gold, (Marked, Race), Copper) = (Silver + 1, (Gold > 20, :Rich), 13)
Nord , (Gold, Silver, Copper) += (1, 2, 3)
```

The update operators `+=`, `-=`, `*=`, and `/=` apply componentwise, retaining each column's existing publication rules, including carrier/range projection. A missing RHS member removes that row from the whole tuple write; other selected rows can still receive the complete tuple. A type, shape, lineage, or runtime validity failure refuses the operation. An independently composed sibling effect retains its own validity mask.

```haskell
Nord , (Gold, Silver, Copper) = (Silver, Copper, Gold) ; Marked = Gold > 20
Nord , (Gold, Silver, Copper) = (Silver, Copper, Gold) |> Marked = Gold > 20
```

With `;`, Marked reads the incoming Gold. With `|>`, it reads the rotated Gold. Tuple assignments are single effects in these compositions, and also work in standing rules, existing comprehensions, and continuations. An omitted-subject tuple update uses the same antecedent/`^cursor` rules as a scalar update; use an explicit subject or leading comma for tuple `=` assignments.

A trailing comma is allowed, including the singleton tuple `(Gold,) = (4,)` in effect position; `(Gold)` is grouping. Empty tuples are not assignment targets. This form assigns columns from explicit matching tuple expressions; it does not introduce tuple-valued registry storage or tuple-returning function signatures. Erlang-style tuple comprehensions and guards are a separate [design exploration](../todo/07-tuple-comprehensions-and-guards.md). [Demo 145](../demos/2-effects/145-tuple-assignment.ano) is an executable example.

### 9. Structural effects

```haskell
Burdened , -Encumbered
Nord , +Blessed
Dead , ~
```

`+Component` adds presence, `-Component` removes it, and `~` despawns. Spawn fills from the prototype, then the column default, then the carrier zero. Unique columns mint fresh values.

These are the Rust registry and emitted BQN operations. They do not imply an implemented archetype/chunk or generational allocator.

### 10. Simultaneous and sequential effects

The comma is the hinge: selection on the left, effects on the right. Both sides admit sequential composition with `|>`: each stage receives the preceding stage's result. On the selection side, stages transform the selected view; on the effect side, later stages observe the state produced by earlier effects.

`;` batches effects at one barrier. Every effect in the batch reads the same incoming state, and their writes take effect together. Textual order does not make one effect's writes visible to another. Conflicting writes require a supported merge law; incompatible writes refuse rather than becoming last-writer-wins.

```haskell
Nord , Silver = Gold ; Gold = Silver
```

This swaps Silver and Gold for each selected row with both values present. With Silver 30 and Gold 10 before the batch, the result is Silver 10 and Gold 30. Reversing the two effects gives the same result.

`|>` sequences effects from left to right. The preceding stage completes before the next stage reads its values; each stage's incoming state includes the preceding stage's writes.

```haskell
Nord , Silver = Gold |> Gold = Silver
```

Starting again from Silver 30 and Gold 10, Silver first becomes 10, then Gold reads the updated Silver and becomes 10. Both end with the original Gold. Reversing these stages instead leaves both with the original Silver. The pipe composes the two assignments on the right of the comma.

```haskell
Nord , Gold += 100 |> Silver = Gold
```

Here Silver receives the increased Gold. With `;` in place of `|>`, Silver would receive the original Gold instead. A statement containing an effect pipeline therefore cannot be described as every effect reading one statement-wide pre-state.

Parentheses group simultaneous effects into a pipeline stage:

```haskell
Nord , (Silver = Gold ; Gold = Silver) |> Gold += Silver
```

The first stage swaps the values; the second adds the new Silver to the new Gold. Without parentheses, `|>` binds more tightly than `;`: in `Nord , Silver = Gold ; Gold += Silver |> Marked = Gold > 20`, Silver reads the incoming Gold, while Marked reads the increased Gold inside its own branch. Reversing those simultaneous branches preserves the result. A pipeline's intermediate writes stay private to that branch; its final writes participate in the enclosing batch's merge checks. Sequences made entirely of additive updates (`+=`, `-=`) or multiplicative updates (`*=`, `/=`) on unrefined numeric columns retain that merge family, with their operands evaluated in stage order. Boolean add/remove sequences likewise retain their compatible mask merge. Assignment, mixed families, refinement boundaries, and structural changes require the enclosing branch's final writes to pass the ordinary compatibility checks.

The selection supplies the subject for the whole effect expression. Changing a selected row's values does not re-run the predicate. Spawned rows are visible to subsequent reads, but do not automatically join that subject; despawn removes selected rows from later stages and continuations. Each stage applies the carrier/range checks before the next stage reads its result.

These forms run in Steel and Kore, including standing rules, comprehensions, and continuations. Kore saves the resulting world after successful execution; a refused later stage does not save an intermediate state. The [worked examples](ano-examples.md#simultaneous-and-sequential-effects) include a complete registry.

```haskell
Nord , Gold += 1000 ; +Blessed
Nord & Dead , spawn Ghost
~
```

A subsequent statement is another barrier. A leading comma, lone `~`, or omitted-subject effect can reuse the saved antecedent mask. It reuses the selection, not the old world. A cold omitted-subject effect uses `^cursor`; a cold explicit continuation refuses.

Augmented assignments such as `Gold += 100` admit the same omitted subject. Bare `Gold = 100` remains a comparison query; use `, Gold = 100` to assign through a saved antecedent. Registered verbs accept ordinary argument lists, including `verb()` and `verb(a, b)`; the registry still supplies their signatures and effects. They also admit omitted subjects: `verb() |> Gold += 1` uses the saved subject in the current program, or `^cursor` when cold. Registry declarations distinguish these effect calls from value-returning query calls. Each Kore submission is a new program, so an omitted subject in a later submission starts cold.

#### Displaying selected rows

Register `fn show` in the world registry to use the tutorial's display effect:

```haskell
IsHostile , show()
Race = :Nord , show(Race, TwoHanded, Gold)
```

`show()` displays every entity column in declaration order; named arguments select and reorder columns. Rows appear in table order with their current zero-based row indices. Scalar bindings, selection aliases, callable declarations, and lattice fields are not entity columns. Display changes no world values. An absent component displays `_`, and an empty selection still displays its headers.

Display observes the same stage rules as other effects:

```haskell
Race = :Nord , Gold += 100 ; show(Gold)
Race = :Nord , Gold += 100 |> show(Gold)
```

Run independently from the same initial world, the first displays the original Gold and the second displays the increased Gold. Kore places each table in its Outputs pane, labeled by the source statement. [Demo 144](../demos/1-selection/144-show.ano) runs the tutorial's examples with their registry.

### 11. Standing rules

```haskell
def mark = Nord & !Blessed => +Blessed
undef mark
```

`=>` installs a standing rule. Named retraction is a top-level control operation, not an effect. Duplicate live names and unknown retractions refuse.

Rules scheduled together begin from one pre-state. Sequential stages within a rule see their own preceding stages, while other rules retain that shared incoming state. Final writes must have disjoint footprints, a supported merge, or guards proving row disjointness. There is no fixpoint evaluation.

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

`fold(f)` and `f/` resolve through the same operation table. This includes `fold(#)` for count and `scan(#)` for prefix count. The lexer does not admit `//` for division reduction. Registered homogeneous reducers must satisfy the current carrier and callable contracts.

There is no explicit seed argument. That remains an open decision in [task 06](../todo/06-seeded-fold-and-traverse.md).

### 13. Grouped fold

```haskell
Pen , Headcount = #/ (livestock & Cattle)
Plot , Moisture = avg/ neighbors.Moisture
```

A fold over a declared set-valued relation reduces each group to a per-source result; prime changes the edge direction rather than enabling grouping. Empty fibers use the operation's identity or lose their result row. Count counts admitted elements, not their numeric payload.

`f/ col @ mask` is scoped-global. `f/ rel.col` is per-source. The legacy `rel @ row` form has its own row-fiber lowering. Named reducers over fibers and per-fiber scans are currently refused.

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

On the left of the comma, `Enemy |> order by Threat desc |> take 5` selects enemies, orders that view by descending Threat, then takes up to five rows from the ordered result. Each stage consumes the preceding stage's result. The same sequential-composition operator sequences effects on the right of the comma (section 10); its meaning is not limited to selection stages.

A scalar ordering key ties every admitted row and preserves the incoming order. Rows with no valid ordering key are omitted. Grade orders a view; rank produces values. The current dense-rank form gives tied values the same rank. Ordered views retain row identity for later effects. `rank` is a resolved built-in form, not one of the lexer keywords.

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

Each selected source contributes its copy count. A copy retains its source and a zero-based copy `index`. The two forms use that same source/copy relationship. Further selection filters retain only their sources' copy counts, and simultaneously scheduled rules each retain their own expansion.

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

The implemented token tables, parser, and AST are [lex.rs](../steel/src/lex.rs), [parse.rs](../steel/src/parse.rs), and [lib.rs](../steel/src/lib.rs). [ano-keywords.md](ano-keywords.md) lists tokens and context-sensitive forms. Missing parser support does not remove a form from the language contract.

The composition precedence, from loosest to tightest, is the `,` / `=>` hinge, `;`, `|>`, then individual effects and assignments. `|>` composes stages left to right and applies on both sides of the hinge. Thus `Nord , Silver = Gold |> Gold = Silver` has one selection and two sequential effects: the pipe composes the assignments, rather than becoming part of the first assignment's value expression.

Postfix prime repeats on a relationship operand. In a dotted chain it applies to the preceding leg; `(A.B)'` explicitly converses the whole composition.

Within expressions, the main precedence order is mask OR, mask AND, negation, comparison, fold/scan prefixes, addition/subtraction, multiplication/division/modulo, scope, then hops/atoms. Parenthesize compound fold scopes. Assignment and control forms have their own grammatical context rather than being ordinary value operators.

In effect position, parenthesized targets followed by an assignment operator form a tuple assignment; other parenthesized effects group a batch. Commas within those target/value parentheses separate tuple members and do not introduce another selection/effect hinge.

The effect grammar parses semicolon-separated branches, each containing a left-to-right `|>` sequence of effects or parenthesized effect groups. The emitter evaluates each sequence from the enclosing batch's incoming state, commits each stage for the following stage's reads, then merges the branch's final writes at the enclosing barrier. Parallel branches still require compatible writes.

Unary minus negates a numeric value; repeated mask negation composes normally. Newlines inside parentheses or comprehension brackets, and after a hinge, effect separator, or unfinished operator, continue the same statement. A newline after a complete statement remains a barrier. Within a comprehension effect, the unparenthesized `|` begins the generator list; put value-level `|` expressions in parentheses or call arguments.

The parser rejects excessive expression nesting with a diagnostic before recursive descent or downstream expression traversal can exhaust the stack. Parenthesis, parser-recursion, operator-chain, and expression-tree depth each have a conservative 64-level ceiling; a program may still contain arbitrarily many separate statements.

Both readers produce the same token kinds and parser input. Japanese noun spellings survive until resolution; paired active demos check equivalent emission. See [ano_nihongo.md](ano_nihongo.md).

## Open Questions, Next Steps

[Task 06](../todo/06-seeded-fold-and-traverse.md) preserves the seeded-fold and fallible-traversal decisions. [Task 99](../todo/99-spatial-lattice.md) owns the spatial contract and its implementation-path choice. Neither is silently settled by this reference.

When sources disagree, preserve the author's rulings and identify whether the conflict concerns a design requirement or implementation status. Current code and tests establish what runs; an unimplemented ruling remains an obligation, not a feature to advertise or a decision to erase. Old progress annotations in the read-only ledger are historical; the live task index records what has since landed.

The [historical rulings](../todo/00-historical-rulings.md) retain the author's decisions. Lean theorems establish their stated abstract contracts, not that Steel constructs the required evidence. [proofs/lean.md](../proofs/lean.md) states that boundary.
