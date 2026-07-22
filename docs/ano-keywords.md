# Ano keywords

The closed grammar of both surfaces, catalogued. Ano has two closed vocabularies. The language owns exactly eighteen reserved words — the lexer's closed set, everything else a name resolved against the world. The registry (`.reg` files) owns a separate set of line directives plus a handful of type and kind sub-words. The two meet where a registry schema becomes the nouns of an Ano sentence. This file is the atlas; `ano-language.md` is the spec it derives from, `ano-manual.md` the tutorial. Where this file and `ano-language.md` disagree, the spec wins.

What counts as a keyword. Per GRAMMAR.md: the closed keyword set the lexer owns is `def spawn at to via along order by take desc top grade fold scan scan2 cross expand til`; everything else, `index rank prev neighbor char x y row` included, is a name resolved by the registry or the prelude. So the eighteen below are the keywords; the structural glyphs (`, => ; |> & | ! @ . ' ~`) are operators, and the contextual specials (`index`, `rank`, `eval`) are names. Habitat and carrier capabilities do not create keyword classes.

## Atlas: language keywords

Each keyword carries its Japanese spelling. The `--! ja` skin converges on shared token kinds and the same parser. Conjugate programs must emit byte-identical BQN.

| Keyword | 日本語 | Token | Cluster | Role |
|---|---|---|---|---|
| `def` | 定義 | `T_DEF` | definition | name a predicate or derived column (inlined at use); with `=>`, install a standing rule |
| `spawn` | 生成 | `T_SPAWN` | generation | mint new entity rows; the only thing that invents keys |
| `at` | 於 | `T_ATKW` | generation | place spawned rows (`spawn X at pos`); fill an anchored frame's origin |
| `to` | 至 | `T_TO` | generation | reshape — pour an ordered selection into a shape, minting nothing |
| `via` | 経由 | `T_VIA` | relational | dispatch to a registered routine with declared habitat and footprint |
| `along` | 沿 | `T_ALONG` | array | name the order a scan accumulates along |
| `order` | 整列 | `T_ORDER` | order | pipeline sort stage (`\|> order by …`) |
| `by` | 別 | `T_BY` | order | the sort-key introducer inside `order by` |
| `take` | 取 | `T_TAKE` | order | pipeline truncation to k rows |
| `desc` | 降順 | `T_DESC` | order | descending modifier for `order` and `grade` |
| `top` | 上位 | `T_TOP` | order | top-k graded selection (`top 5 (grade …)`) |
| `grade` | 格付 | `T_GRADE` | order | the permutation that sorts (APL `⍋`); selection-only, never a write-back |
| `fold` | 縮約 | `T_FOLDKW` | array | long form of the reducer (`fold(f) col @ scope`); collapses a column to a scalar |
| `scan` | 走査 | `T_SCANKW` | array | long form of the scan (`scan(f) col along order`); length-preserving accumulation |
| `scan2` | 二重走査 | `T_SCAN2` | array | two-axis scan — the summed-area table over a lattice |
| `cross` | 交差 | `T_CROSS` | generation | outer product: apply a function to every pair of two selections |
| `expand` | 展開 | `T_EXPAND` | generation | pipeline replicate and flat-map (`\|> expand Count`) |
| `til` | 連番 | `T_IOTA` | generation | the generator: `til n` is `0 1 … n-1` |

Adjacent vocabulary, closed grammar but not lexer keywords, listed so the atlas is honest. These resolve as names or contextual heads, and a registry entry can shadow most of them.

| Word | What it is |
|---|---|
| `index` | ordinal within the selection, or per-copy under replicate; never shadowable |
| `char` | per-cell glyph column of a board literal; never shadowable |
| `x` `y` `row` `prev` `neighbor` | probe-guarded specials — coordinate columns, row self, stencil shifts; take a registry shadow under the fold |
| `rank` `abs` `sin` `polar` `dist` `fib` … | builtin or registered callables; `rank(…)` is the value-only write-back twin of `grade` |
| `eval` | contextual statement head — the compile-time quotation splice (`eval "…"`), not a lexer keyword |

Structural operators, glyphs and not keywords, for completeness. Each carries its Japanese particle where the skin defines one.

| Glyph | 日本語 | Meaning |
|---|---|---|
| `,` | 、 が は | the hinge — separates selection from effects |
| `=>` | なる | anonymous standing-rule hinge |
| `;` | て | effect separator (all against one pre-state, one barrier) |
| `\|>` | `\|>` | pipeline stage separator |
| `&` `\|` | と か | Lesser and Greater over masks or numbers |
| `!` | ない | mask NOT |
| `@` | で | scope — fold scope, locative binding, frame fix (distinct from the `at` keyword) |
| `.` | の | functional hop (`rel.Comp`) |
| `'` | tick | fiber or image (`targets'`, `sel.rel'`) |
| `~` | 消 | despawn the saved mask |
| `+Comp` `-Comp` | 付 除 | presence writes (add or remove a component) |
| `<-` | `<-` | comprehension binder |
| `= += -= *= /=` | にする たす ひく かける わる | assignment family |
| `== != < <= > >=` | 同 不同 未満 以下 超・より 以上 | comparisons |
| `/` `\` | — | fold and scan markers — fuse to a name or operator; see the fold and scan table |

Fold and scan operators, the closed reducer family. Each is a fold under `/` and a scan under `\`, carrying its own word on the `--! ja` fold and scan skins. Full semantics, empty-scope law, and compiler liveness live in the Operators sections below.

| fold | scan | 日本語 (fold / scan) | reading (fold / scan) | identity |
|---|---|---|---|---|
| `+/` | `+\` | 総和 / 累和 | sum / running sum | 0 |
| `*/` | `*\` | 総積 / 累積 | product / running product | 1 |
| `&/` | `&\` | 皆 / 累皆 | all or minimum / still-all or running minimum | mask 1, number none |
| `\|/` | `\|\` | 或 / 累或 | any or maximum / ever-any or running maximum | mask 0, number none |
| `#/` | `#\` | 総数 / — | count / running count | 0 |
| `max/` | `max\` | 最大 / 累大 | numeric bridge for `\|/` / `\|\` | none — row drops |
| `min/` | `min\` | 最小 / — | numeric bridge for `&/` / `&\` | none — row drops |
| `avg/` | `avg\` | 平均 / — | mean / running mean | none — row drops |
| `f/` | `f\` | `脅威/` / `脅威\` | named reducer / its scan | registered |

The incomplete bridge scans are `#\`, `min\`, and `avg\`. Numeric `&\` is the running minimum, but Steel does not yet implement the carrier overload. `scan(min) X along order` already works. The running mean and running count exist in no spelling. Full status lives under the fold and scan markers below. `-/` and `//` remain rejected as non-associative.

## Atlas: registry keywords

`.reg` files are line-based; the first word of each line is a directive. Directive and kind words are exact-byte lowercase ASCII — they never fold and have no Japanese spelling, though the entry names they introduce may be kanji. Source: the current Rust reference, `steel/src/registry.rs`.

| Directive | Shape | Role |
|---|---|---|
| `n` | `n <count>` | world row count — the one header; precedes anything counted against it |
| `lattice` | `lattice <w> <h>` | declare a spatial grid; precedes any `field` |
| `col` | `col <name> <type> <values…>` | a per-entity component column |
| `field` | `field <name> <type> <values…>` | a per-cell column over the `w·h` lattice |
| `unique` | `unique <name> [num\|nat\|int] <values…>` | an injective key column — mints `1+max` on spawn, write-refused |
| `pres` | `pres <col> <mask>` | presence mask — makes a component partial |
| `default` | `default <name> <v>` | the spawn-fill value for a column |
| `range` | `range <col> <lo> <hi>` | declared value bounds — seals rest data, clamps writes |
| `rel` | `rel [key] <name> <values…>` | functional relationship (one target per source); `-1` means no link |
| `srel` | `srel [key] <name> <fibers…>` | set-valued relationship (`\|`-separated fibers) |
| `inv` | `inv <name> <rel>` | the inverse read of a functional `rel`; fibers computed at load |
| `alias` | `alias <name> <mask>` | a stored boolean mask — a value with a name |
| `bind` | `bind <name> <kind> <values…>` | a named constant (see bind kinds) |
| `fn` | `fn <name> [verbatim BQN]` | a registered callable body |
| `as` | `as <word> <name>` or `as <word> <col> <value>` | pure name alias (3-word) or derived tag, an equality mask (4-word) |
| `ja` | `ja <word> <name>` | name alias, flagged as documenting the Japanese surface |
| `role` | `role <keys\|id\|parent\|proto\|pos> <col>` | point a system role at a native column |
| `def` | `def <name> [col=v …]` | a proto or archetype — named fields for spawn fill |
| `reap` | `reap <seal\|host>` | the `~` reclamation policy, world-level |

Sub-word vocabularies, filling the second slot of a directive and equally closed.

| Slot | Words | Where |
|---|---|---|
| column types | `num` `bool` `nat` `int` `sym` `char` `vec` | `col` and `field` type slot; `nat` `int` `num` also refine `unique` |
| bind kinds | `entity` `num` `point` `mask` `vec` | `bind` kind slot |
| roles | `keys` `id` `parent` `proto` `pos` | `role` role slot |
| reap policies | `seal` `host` | `reap` policy slot |

The refined numeric carriers: `nat` is ℕ ∩ [0, 2⁵³], `int` is ℤ ∩ [−2⁵³, 2⁵³], `bool` is {0,1}, `num` is any finite double. Out-of-carrier rest data refuses at load; it does not repair.

## How the two vocabularies intersect

The registry and the language are the schema and the query over one column store. Five seams join them.

The registry vocabulary is the language's noun vocabulary. Every `col`, `rel`, `srel`, `bind`, `fn`, `as`, and `def` name declared in a `.reg` becomes a resolvable word in the program's selection predicates: `col gold num …` makes `Gold` a mask and column you write `Gold += 100` against. Resolution folds case for registry names (so `Gold` reaches column `gold`), then walks the alias table one hop — but program-level `def` heads, comprehension binders, and values (`:Sym`, sym and char data) stay exact-byte. Complexity lives registry-side; the language stays flat.

`def` lives on both sides and they never meet. Registry `def` is a proto, a row-oriented archetype of `col=value` fields consumed by `spawn`. Program `def` is a predicate or rule, inlined at use or installed on `=>`. Registry `def` resolves at load, program `def` at parse, so the reuse is deliberate, not a collision. `spawn Marine` reaches the registry proto; `def master = Human & Nord …` names a program predicate.

The key machinery couples `unique`, `role`, `default`, and `range` to `spawn` and to writes. `unique` (or `role keys`/`role id`) declares which column mints on `spawn` and refuses effect writes; `default` and `range` fix what `spawn` fills and what an effect write clamps to. The keyword `spawn` has no minting policy of its own — it reads it entirely from the registry.

The relation directives are the hops the language traverses. `rel`, `srel`, and `inv` register spans that `.` (functional hop), `'` (fiber and image), the grouped fold, and `via` consume. `via Adj` dispatches over a registered relation; `neighbors(wrap)'.Moisture` folds over an explicit neighborhood and boundary; `inv` exposes the reverse fibers.

Both surfaces are bilingual, but only one carries the mirror. Every language keyword has a kanji spelling (`定義` is `def`, `生成` is `spawn`), and `ja` registry lines alias entry names across surfaces — a natively Japanese world (`col 金 num …`) needs no alias at all. Registry directives, by contrast, are ASCII-only. `reap` sets the policy for the language's `~`; the derived tag from `as` becomes a read-only selection noun. The shared concept-words `num` `vec` `mask` `point` `entity` name registry carriers that are also the language's value kinds: a `col pos vec` is what the program reads as `.pos`, a `bind … mask` is the object the mask algebra produces.

## Language keywords

### `def` — 定義

Two jobs, chosen by whether a `=>` hinge follows. As a name, `def <name> = <selection-or-column-expr>` names a predicate or derived column, textually inlined at every use, never a stored value. A `def` head is a program variable, exact-byte and outside the registry's case fold, so `def ridge` beside a column `Ridge` are two distinct names; a lexer-reserved word cannot head one (`def til` rejects, `def Til` is legal). As a rule, `def <name> = <selection> => <effects>` installs a standing rule the clock fires each tick.

```haskell
def threat = Damage * Speed / Range          -- a derived column, inlined at use
def master = Human & Nord & TwoHanded > 60   -- a named predicate
def kin = #/ (moore' & Planted)              -- a grouped-fold column (Conway neighbor count)
def payload = SparkBolt & Trigger & Hit => spawn Firebolt at pos ; ~   -- a rule
```

### `spawn` — 生成

Mints new rows; the sole key-inventing operation, since filters cannot invent keys. Forms: `spawn Proto`, `spawn Proto * countExpr` (replicate — each selected source emits `count` copies, `index` bound `0..` per source), `spawn Proto at posExpr` (placement), and `spawn (pieceOf char)` from a board literal. Several spawns may batch in one barrier; keys mint once across the batch. Fill layers: proto field, else the column's `default`, else the type zero.

```haskell
Spawner , spawn Minion * Count                        -- one copy per Count, per source
Nest , spawn Egg * Fertility at pos + polar(index, index * 137.5)
Nord & Dead & Soul , Soul = 0 ; spawn Ghost           -- consume then mint, same barrier
```

### `at` — 於

Placement and origin. In `spawn X at posExpr` it positions the minted rows; in an anchored frame `mask @ frame(args) at originExpr` it fills the frame's origin, which must be one point (a pair mirror-read or a `point` binding). Do not confuse it with `@`, the scope glyph (日本語 `で`): `at` is a keyword, `@` an operator.

```haskell
Wand , spawn SparkBolt at pos + (12, 0) ; Mana -= 5
Wand , spawn SparkBolt * 3 at pos + polar(12, (index - 1) * 90)   -- a fan of 3
```

### `to` — 至

Reshape: pours an ordered selection into a numeric shape, producing exactly count-of-selection positions and minting nothing. A `_` axis is inferred. Also drives the board literal (`"glyphs" to 8 8`). Rule of thumb: generate when the rows don't exist, reshape with `to` when they do.

```haskell
Soldier , pos = to 8 8       -- reshape the soldiers into an 8×8 block
Soldier , pos = to 4 _       -- 4 ranks, width inferred
"RNBQ..." to 8 8 , spawn (pieceOf char)
```

### `via` — 経由

Host dispatch: `fn via Col` hands a selection and registered relation to a callable with declared habitats and footprints. Ano guarantees that envelope and does not inspect the routine's internals. The same stored relation may also expose a boolean field on `Node × Node`, where `Node , OutDeg = +/ Adj@row` is a native fold. These are two registered interfaces.

```haskell
Hostile , shortestPath via Adj      -- the host runs Dijkstra over the Adj relation
```

### `along` — 沿

Names the order a `scan` accumulates along when the selection has no intrinsic order. Steel sorts by the order key, scans, and scatters back through the inverse grade and query lineage. Tied order keys retain stable-index behavior, so effects depending on ties must declare that policy.

```haskell
scan(+) Weight along pathCells       -- order named explicitly
+\ Weight @ (til steps |> route A B) -- or: the source view already carries an order
Spell , Bal = 1 + (scan(+) (DrawAdd - 1) along rot)   -- balance along a wand rotation
```

### `order` — 整列

The pipeline sort stage, `src |> order by <col> [desc]`, producing an ordered view that composes with `take`, folds, and further stages.

```haskell
Enemy |> order by Threat desc |> take 5 , +Targeted
Spell |> order by Slot desc |> take 1 , spawn Copy
```

### `by` — 別

The sort-key introducer, only ever inside `order by <col>`, naming the column the sort ranks on. Grouping-by, q's `by`, is not this word in Ano — grouping is the grouped fold, a tick-marked hop under a reducer.

```haskell
Enemy |> order by Threat desc |> take 5
```

### `take` — 取

Pipeline truncation: `|> take k` keeps the first k rows of an ordered view, the value-level twin of top-k.

```haskell
Enemy |> order by Threat desc |> take 5 , +Targeted
+/ threat @ (Enemy |> order by dps desc |> take 10)   -- fold over the truncated view
```

### `desc` — 降順

Descending modifier for `grade` and `order by`. Ascending is the default.

```haskell
order by threat desc
top 5 (grade desc Threat) , +Targeted
```

### `top` — 上位

The graded top-k selection, `top k (grade [desc] <col> [@ scope])` — the fold-prefix reading of the same k rows the `order … take` pipeline selects. The APL reading and the sentence reading, one plan.

```haskell
top 5 (grade desc Threat) , +Targeted
top 8 (grade desc Safety @ 64 64) , spawn Sentry   -- the same operator at rank 2 (space)
```

### `grade` — 格付

Returns the permutation that sorts (APL `⍋`), an ordering witness rather than a value column aligned to the original query domain. It cannot be written directly into a component. Its value-only twin is `rank(...)`, where ties share a rank; write-back still follows query lineage.

```haskell
top 5 (grade desc Threat) , +Targeted     -- grade drives selection
Unit , Slot = rank(Initiative)            -- rank(), not grade, on the effect side
```

### `fold` — 縮約

The long form of the reducer, `fold(f) col @ scope`, collapses a column to one scalar under `@`. Its head is any admitted operator or registered reducer, not a special case for `+`. It covers `+/ */ #/ &/ |/ max/ min/ avg/` and named reducers like `threat/`. An empty fold uses the operand carrier's identity. `+/` and `#/` give 0. Mask `|/` gives false and mask `&/` gives true. Numeric `|/`, numeric `&/`, their `max/` and `min/` bridges, and `avg/` fail the row. Failure is no result row, so a bare query prints nothing. The grouped fold is the same word over a tick-marked hop. It yields one value per selected source. `@` never groups; the tick does. Steel still accepts names only inside `fold(f)`, with operator parity pending in `todo/12-unbuilt-scans.md`.

```haskell
+/ Gold @ Nord                                -- total Nord gold, one scalar
#/ (Nord & TwoHanded > 60)                    -- count; takes a parenthesized mask
fold(threat) Damage @ Enemies                 -- long form of threat/ Damage @ Enemies
Pen , Headcount = #/ (livestock' & Cattle)    -- grouped: count per pen, over the inverse fiber
Plot , Moisture = avg/ neighbors'.Moisture    -- grouped: per-plot mean
```

### `scan` — 走査

The long form of the scan, `scan(f) col along order`. Unlike a fold, a scan is length-preserving — one running value per selected cell — so it needs an order, from the view or from `along`. Live steps split by spelling: the glyph `f\` under an `@` scope emits `+ * max & |` and named reducers (the boolean latches `&\` still-all and `|\` ever-any among them); this long `scan(f) … along` form emits `+ * max min`, so the running minimum runs only through it, never as a `min\` glyph. Steps like `-` and `/` are rejected as non-associative, and the running mean and running count are unbuilt in either spelling (the fold and scan table maps it). The barrier trap the repository pins four ways: `offset = prev.offset + prev.prev.offset` is not a recurrence — the comma is a barrier and every read observes pre-state, so `prev` is a parallel shift, not a carry. Recurrences belong in a callable or across ticks.

```haskell
+\ Weight @ Route            -- running pack weight along a route
*\ Multiplier @ comboChain   -- running combo product
max\ Height @ Ray            -- running high-water along a sightline
scan(+) Weight along pathCells
```

### `scan2` — 二重走査

The two-axis scan, taken along both lattice axes to build a summed-area table, the integral image.

```haskell
+\ Cost @ 8 8         -- Version A: single leading-axis scan
scan2(+) Cost @ 8 8   -- Version B: the summed-area table (both axes)
```

### `cross` — 交差

The outer product: `cross f A B` applies `f` to every pair (a ∈ A, b ∈ B), yielding the matrix of results as a value. Its filtered, selection-shaped sibling is the comprehension `[ t & c , +InRange | t <- Tower, c <- Creep, dist(t, c) < 50 ]`, the theta-join σ_p(A × B).

```haskell
cross dist Tower Creep     -- the full tower×creep distance matrix
```

### `expand` — 展開

The pipeline spelling of replicate, a flat-map stage that emits each row `count` times. `Spawner |> expand Count , spawn Minion` denotes exactly the same flat-map as `Spawner , spawn Minion * Count`, minting identical keys row for row.

```haskell
Spawner |> expand Count , spawn Minion
```

### `til` — 連番

The generator, q's iota: `til n` is `0 1 … n-1`, an ordered index view.

```haskell
+\ Weight @ (til steps |> route A B)      -- ordered scope from a generated view
max\ Height @ (Eye + til n * north)       -- a sightline: eye + 0..n-1 steps north
```

## Operators

The glyphs the lexer owns that are not keywords. They carry the 14-level precedence table (GRAMMAR.md), loosest to tightest: the hinges, then `;`, then `|>`, then the effect and assignment family, then `|`, `&`, prefix `!`, the selection-position comparisons, the fold and scan prefixes with `grade` and `top`, then `+ -`, then `* / %`, then `@`, then `.` and `'`, then atoms. Sections below follow that order, families grouped.

### `,` — the hinge (、 が は)

The first top-level comma, not inside parens or brackets, is the hinge: it splits a statement into selection on the left and effects on the right. A leading comma, `, <effects>`, is the continuation form — a new statement and a new barrier over the antecedent's saved mask. Inside `(…)` and `[…]` the comma is not the hinge but a separator, delimiting a presence tuple or a comprehension's clauses.

```haskell
Predicate , Source *= 2                       -- selection , effect
Nord & TwoHanded > 60 , Gold += 1000 ; +Blessed
   , +Marked                                  -- continuation: new barrier, saved mask
```

### `=>` — the rule hinge (なる)

Installs an anonymous standing rule, `<selection> => <effects>`, which the clock fires each tick. The named form is `def <name> = <selection> => <effects>`. Every installed rule fires every tick in one shared barrier.

```haskell
def bloom = Plot & !Planted & kin == 3 => +Planted
SparkBolt & Trigger & Hit => spawn Firebolt at pos ; ~
```

### `;` — the effect separator (て)

Separates effects within one statement. All effects run against the same pre-state and commit at one barrier; same-column writes compose through the merge laws (§10) or are rejected as written-twice.

```haskell
^cursor , Knockback 5 ; Flash :Red ; -Shielded
Wand , spawn SparkBolt at pos + (12, 0) ; Mana -= 5
```

### `|>` — the pipeline (|>)

Chains selection stages left to right, each feeding the next: `order by`, `take`, `expand` in any sensible order.

```haskell
Enemy |> order by Threat desc |> take 5 , +Targeted
Spawner |> expand Count , spawn Minion
```

### `&` — and (と)

Conjunction over masks; precedence level 6, tighter than `|`. Ano's boolean reading is k's `&` (min over numerics) restricted to masks.

```haskell
Nord & Dead & Soul , Soul = 0 ; spawn Ghost
8 8 & x == y , spawn Pillar
```

### `|` — or (か)

Disjunction over masks; level 5, looser than `&`. k's `|` (max over numerics) restricted to masks. (Inside a `.reg` line `|` separates vec and srel fibers instead — a registry glyph, not this operator.)

```haskell
def life = Plot => Planted = kin == 3 | Planted & kin == 2
```

### `!` — not (ない)

Prefix negation of a mask; level 7. In a presence tuple `!Comp` reads as absent.

```haskell
SparkBolt & Hit & !Trigger , ~
(Nord, !TwoHanded) , +Untrained
```

### `== != < <= > >=` — comparisons (同 不同 未満 以下 超・より 以上)

Comparisons over column expressions, returning bool masks; selection-position, level 8. On the Japanese surface each is postfix on its comparand, re-rooted before it in normalization.

```haskell
Aged > 3mo
Faction == :Hostile
Spell & Proj & Member & Slot == min/ Slot @ (Spell & Proj & Member) , Damage += 10
```

### `=` — set and equality (にする)

Context-decides. Left of the hinge, in selection position, `=` is an equality comparison. Right of the hinge, in effect position, it is the SET assignment. Two SETs on one column in a barrier are rejected unless they are pair-field writes on differing fields.

```haskell
Bandit , Faction = :Hostile        -- effect: SET
Unit , Slot = rank(Initiative)     -- effect: SET from a value
```

### `+= -= *= /=` — compound assignment (たす ひく かける わる)

Accumulating effect-side writes, level 4; each RHS still observes pre-state. The additive family `+= -=` and the multiplicative family `*= /=` each compose within a barrier by rebasing the later accumulate on the earlier commit; mixing families on one column is rejected.

```haskell
Enemy & Hit , Health -= 3
Target , Hits += #/ attackers'
```

### `+Comp  -Comp` — presence writes (付 除)

A `+` or `-` prefixed to a component name in effect position adds or removes that component (a presence bit), distinct from arithmetic `+ -`. On the Japanese surface the marks are postfix on the component (`付`, `除`).

```haskell
(Nord, TwoHanded _) , +Trained
^cursor , Knockback 5 ; Flash :Red ; -Shielded
```

### `~` — despawn (消)

Reclaims the selected rows. Alone on a line it is the continuation form, despawning the antecedent's saved mask; as an effect it follows `;`. The storage policy is the world's `reap`, but the mask-level meaning never changes.

```haskell
Cursed , ~
SparkBolt & Hit & !Trigger , ~
```

### `+ - * / %` — arithmetic (+ - * / %)

Value-expression arithmetic; `+ -` at level 10, `* / %` at level 11 (`%` is modulo). Two glyphs double-book. `*` is the replicate marker inside `spawn X * n` (level 4), and `/` is division only when spaced on its left (`Dmg*Spd / Rng`); glued after a name it is the fold marker instead, as `\` is the scan marker.

```haskell
def threat = Damage * Speed / Range
8 8 & (x + y) % 2 == 0 , spawn Wheat
```

### `@` — scope (で)

Fixes a scope; level 12, and it never groups. Three readings: fold scope, where a fold under `@` is always exactly one scalar (`+/ Gold @ Nord`); the locative binding of a mask to a proper-noun place (`Cheese @ cellar`); and the frame fix of an anchored blast, its origin filled by the `at` keyword. At rank 2 the scope is a lattice or line (`+/ Elevation @ 64 64`). Distinct from the `at` keyword: after a fold, `@` yields a scalar where the tick yields a per-source column, lexically.

```haskell
+/ Gold @ Nord                          -- fold scope: one scalar
Cheese @ cellar & Aged > 3mo            -- locative: cheese in the cellar
Oil @ blast(5) at Firebolt.pos , +Fire  -- frame fixed by @, origin by at
```

### `.` — the functional hop (の)

The single-valued relationship hop, `rel.Comp`, chainable. A bare relationship is its found mask: true exactly when the stored target resolves now. A dangling `¯1` or missing target fails the row like a left-join null. It also mirror-reads a vec column's pair (`Player.pos`) and spells the stencil shift over pre-state (`prev.offset`, `neighbor(clamp).Height`), never a carry. Level 13.

```haskell
12 , spawn Cheese at Player.pos + polar(index, index * 137.5)
def slope = abs(Height - neighbor(clamp).Height)
```

### `'` — the tick: fiber and image

Postfix on a name, level 13. `rel'` is the fiber, legal under folds and quantifiers (`#/ r'`); `sel.rel'` in source position is the image, bits OR'd into a mask so multiplicity collapses to one; `rel'.Comp` is the gathered fiber column the grouped fold reads (`neighbors'.Moisture`, `r'.V`). The mask algebra never sees multiplicity — say so with a fold when it matters.

```haskell
Frenzy.targets' , +Frenzied ; Health -= 10   -- the image: a mask, reached-twice counts once
Target , Hits += #/ attackers'               -- the fiber under a fold: the in-degree
Plot , Moisture = avg/ neighbors'.Moisture   -- the gathered fiber column
```

### `/  \` — the fold and scan markers

Glued to a name or operator with no interior whitespace, `/` lexes one fold token and `\` lexes one scan token. The family includes `+/ */ &/ |/ #/`, named reducers such as `threat/`, and the scans `+\ *\ &\ |\ max\ threat\`. Ano's `|/` and `&/` are q's folds. The operand carrier selects Greater or Lesser. The lexer stays registry-blind, so an unregistered reducer fails at emit.

```haskell
#/ (Nord & TwoHanded > 60)   -- count over a parenthesized mask
threat/ Damage @ Enemies     -- the slash attaches to the reducer name
+\ Weight @ Route            -- the scan marker, one running value per cell
```

A fold on a declared order is exact left accumulation and accepts any compatible registry step. Unordered regrouping requires associativity; parallel/unordered execution that may discard traversal order requires associativity and commutativity. The derived forms keep their spellings while the registry records the step, identity, finish, and available laws. `avg/` folds sum and count, then divides. `#/` is `+/` over ones. A fold without an identity fails the empty row. This is an empty result, so a bare query prints nothing rather than a placeholder scalar. A scan needs no identity, so empty input yields an empty column.

| operator | `f/` fold | `f\` scan | empty-scope identity | Steel emits the scan? |
|---|---|---|---|---|
| `+/` `+\` | sum | running sum | 0 | yes |
| `*/` `*\` | product | running product | 1 | yes |
| `&/` `&\` | all on masks, minimum on numbers | still-all on masks, running minimum on numbers | mask true, number none | mask yes, number no |
| `\|/` `\|\` | any on masks, maximum on numbers | ever-any on masks, running maximum on numbers | mask false, number none | mask yes, number no |
| `#/` `#\` | count | running count | 0 | no |
| `max/` `max\` | numeric bridge for `\|/` | numeric bridge for `\|\` | none — row drops | yes |
| `min/` `min\` | numeric bridge for `&/` | numeric bridge for `&\` | none — row drops | only via `scan(min) … along` |
| `avg/` `avg\` | mean | running mean | none — row drops | no |
| `f/` `f\` | named reducer | registered scan | registered | yes |
| `-/` | rejected — not associative | — | — | — |
| `//` | rejected — not associative, and unlexable (`/` is fold-marker and replicate) | — | — | — |

The scan family has two spellings. Numeric `|\` is running maximum and numeric `&\` is running minimum. Steel still emits both glyphs only for masks. `max\` works as the numeric maximum bridge. `scan(min) X along order` works, but the `min\` bridge is still unbuilt. Running mean and running count remain absent. `avg\` is an emit error, and `#\` is a lex error. Those bridge gaps stay in `todo/12-unbuilt-scans.md`. The carrier overload is pending in `todo/17-greater-lesser.md`. `>` remains a comparison, so `>/` stays rejected.

### `+/`  `+\` — sum, running sum

Addition. `+/` totals the scope; `+\` returns the running sum, one cell per selected position. Identity 0, so an empty scope folds to 0.

```haskell
+/ Gold @ Nord               -- total Nord gold
+\ Weight @ Route            -- pack weight accumulating along the route
```

### `*/`  `*\` — product, running product

Multiplication. `*/` multiplies the scope; `*\` returns the running product. Identity 1.

```haskell
*/ Multiplier @ comboChain   -- the whole combo product
*\ Multiplier @ comboChain   -- the running combo up each step
```

### `&/`  `&\` — all and minimum, still-all and running minimum

`&` is q's Lesser. On masks, `&/` is ALL and `&\` is the still-all latch. On numbers, `&/` is minimum and `&\` is running minimum. Empty mask input yields true. Empty numeric input has no identity and fails the row.

```haskell
&/ Alive @ Party             -- is the whole party alive?
&\ Intact @ Hull             -- still whole up to each section
```

### `|/`  `|\` — any and maximum, ever-any and running maximum

`|` is q's Greater. On masks, `|/` is ANY and `|\` is the ever-any latch. On numbers, `|/` is maximum and `|\` is running maximum. Empty mask input yields false. Empty numeric input has no identity and fails the row.

```haskell
|/ Burning @ Forest          -- is anything burning?
|\ Reached @ Fuse            -- has the flame passed each cell yet
|/ Threat @ Frontier         -- maximum threat
|\ Height @ Ray              -- running maximum
```

### `#/`  `#\` — count, running count

Cardinality. `#/` counts, taking a parenthesized mask; it is `+/` over ones. Identity 0, and the fold is live. The running count `#\` has no spelling at all: `#` forms only the fold `#/`, so `#\` is a lex error — `line N: '#' begins only the fold '#/'`.

```haskell
#/ (Nord & TwoHanded > 60)             -- how many trained Nords
Target , Hits += #/ attackers'         -- in-degree: count per target
```

### `max/`  `max\` — maximum, running peak

`max/` and `max\` remain numeric bridges for `|/` and `|\`. An empty scope drops the row because finite float64 has no maximum identity.

```haskell
max/ Height @ Ray            -- the tallest along the ray
max\ Height @ Ray            -- the running skyline up the sightline
```

### `min/`  `min\` — minimum, running floor

`min/` and `min\` remain numeric bridges for `&/` and `&\`. `min/` works. The `min\` bridge is still unbuilt, while `scan(min) X along order` works. An empty scope drops the row because finite float64 has no minimum identity.

```haskell
Spell & Proj & Member & Slot == min/ Slot @ (Spell & Proj & Member) , Damage += 10
```

### `avg/`  `avg\` — mean, running mean

Arithmetic mean folds sum and count, then divides. `avg/` is live. `avg\` is the ruled running mean but has no emit rule. An empty fold fails the row.

```haskell
Cow & Weight < avg/ Weight @ Cow , +Marked   -- @: one scalar, the herd mean
Plot , Moisture = avg/ neighbors'.Moisture   -- ': a column, one mean per plot
```

### `f/`  `f\` — the named reducer

Any registered reducer name fuses with the marker: `threat/` folds, `threat\` scans, and `fold(threat)` is the long form — one reducer, three spellings. Its identity is whatever the registry records. The lexer is registry-blind, so an unregistered name in fold position fails at emit.

```haskell
threat/ Damage @ Enemies     -- fold with the registered reducer
threat\ Damage @ graded      -- its scan, free
```

### `-/`  `//` — the rejected reducers

`-/` is rejected because subtraction is not associative, and `//` is rejected on the same ground and is additionally unlexable, since `/` is already the fold marker and the replicate glyph. Neither is guessed at; both are errors.

### `( )` — grouping, call, tuple

Parentheses group for precedence, apply a callable (`blast(5)`, `polar(…)`, `dist(t, c)`), and write vec and point literals (`(12, 0)`). In predicate position a comma-tuple is the presence tuple: `(Nord, TwoHanded > 60)` present and constrained, `(Nord, TwoHanded _)` present any, `(Nord, !TwoHanded)` absent.

```haskell
(Nord, TwoHanded > 60) , Gold += 1000
SparkBolt , pos = pos + (24, 0)
```

### `[ ]` — the comprehension

Brackets delimit a comprehension, a statement form: `[ <sel> , <effect> | a <- A, b <- B, <filter>… ]`, the theta-join σ over the cross of its generators, filters permitted to call registered functions.

```haskell
[ b & e , +Hit | b <- SparkBolt, e <- Enemy, dist(b, e) < 8 ]
[ t & c , +InRange | t <- Tower, c <- Creep, dist(t, c) < 50 ]
```

### `<-` — the comprehension binder

Binds a per-side row variable inside a comprehension, naming each generator's element for the filters and body.

```haskell
[ a & b , Collide | a <- Body, b <- Body, a < b, overlap(a, b) ]
```

### `_` — the wildcard

A lone underscore, T_WILD: an inferred axis in a reshape (`to 4 _`, width computed) or the present-any slot in a presence tuple (`TwoHanded _`).

```haskell
Soldier , pos = to 4 _         -- 4 ranks, width inferred
(Nord, TwoHanded _) , +Trained
```

### `^` — the alias sigil

A caret immediately followed by a name, no whitespace, lexes as one alias token; a bare `^` is a lex error. It selects the dynamic alias overlay. Lookup reads the live alias first and falls through to the bare namesake when none exists, so `^Whiterun` may shadow bare `Whiterun` without replacing it. It follows the identifier policy, so `^世界` is legal.

```haskell
^cursor , runBehaviorTree
^cursor , Knockback 5 ; Flash :Red ; -Shielded
```

### `:` — the symbol sigil

A colon immediately followed by a name, T_SYM: a symbol value. Symbols are values and case-sensitive, never folded, and UTF-8 is legal (`:山賊`).

```haskell
Bandit , Faction = :Hostile
Dead , Loot = :Empty
```

## Registry keywords

### `n` — world row count

`n <count>`. The single header; must precede any line counted against it. Redeclaration is refused — a world validated against two counts has no one-header spelling to dump back. `n 0` is legal, an empty world.

```
n 5
col source num 10 20 30 40 50
```

### `lattice` — spatial grid

`lattice <w> <h>`. Declares a `w × h` cell grid; must precede any `field`. Redeclaration refused.

```
lattice 5 5
field seed bool 0 0 1 0 0  0 1 1 1 0  ...
```

### `col` — per-entity component

`col <name> <type> <values…>`, `n` values. Types: `num` `bool` `nat` `int` take `n` numbers, carrier-checked; `sym` takes `n` exact-byte words; `char` takes the raw glyph run, length checked in bytes; `vec` takes `|`-separated pairs, fixed-arity tuples.

```
col gold nat 30 0 0 0 55
col predicate bool 1 0 1 0 1
col race sym Nord Imperial Nord Breton Nord
col pos vec 0 0 | 3 4 | 1 1 | ...
```

### `field` — per-cell column

`field <name> <type> <values…>`, exactly `col` but over the lattice's `w·h` cells, row-major. Requires a prior `lattice`.

```
lattice 3 3
field moisture num 1 2 3 4 5 6 7 8 9
```

### `unique` — injective key column

`unique <name> [num|nat|int] <values…>`, a numeric column under a declared injectivity constraint: every element pairwise-distinct, checked at load, a repeat named with both rows. An optional kind word refines the carrier by set intersection — `unique id nat` is injectivity ∩ ℕ, `unique slot int` lets negative keys live. `spawn` mints `1+max`; effect writes to it are refused; `pres`, `default`, and proto fields all reject on it, since a key column is total and minted, never defaulted.

```
unique id nat 3 5        -- distinct naturals; spawn mints 6, then 7
unique slot int -5 -3    -- negative keys allowed under the int carrier
```

### `pres` — presence mask

`pres <col> <mask>`, a length-`n` bitmask of which rows carry the component. Rejected on a `unique` column, a key column being total.

```
col mana num 100 0 50 0 0
pres mana 1 0 1 0 0        -- only rows 0 and 2 have mana
```

### `default` — spawn fill

`default <name> <v>`, the value `spawn` fills when no proto field covers the column. Must sit inside the column's carrier and any `range`. Rejected on a `unique` column, minted not defaulted.

```
col hp nat 100 100 100
default hp 50
```

### `range` — declared bounds

`range <col> <lo> <hi>`, a value refinement beside the kind word: rest data seals here, out-of-range refusing at load; effect writes clamp to the interval at the barrier; the TUI clamps and warns. Rejected on `bool`, `sym`, `char`, `vec`, and `unique` columns, and if redeclared; the column's default must lie inside.

```
col health nat 8 8 8 8
range health 0 8         -- writes clamp into 0..8
```

### `rel` — functional relationship

`rel <name> <n values>`, one target per source, `-1` the None sentinel, keyed to the row index. The keyed form `rel <keycol> <name> <n values>` names a `unique` key column first (negatives refused). Bare `rel` is the found mask, so it is true only when the stored target resolves in the current world. The hop uses the same index-of and found-guard. Any nonnegative target that fails it is DEAD, with no finer missing-state taxonomy. The `-1` sentinel stays silent.

```
rel target 2 0 -1 4 1              -- entity 0 → 2, entity 2 → nobody
rel id owner 5 3 -1 5 3            -- keyed: owner values live in id-space
```

### `srel` — set-valued relationship

`srel <name> <fibers>`, one fiber per row, `|`-separated, empty fibers legal; keys the same way as `rel`. This is the ragged one-to-many representation the grouped fold and the `'` fiber operate over, stored as CSR: offsets plus a flat edge array.

```
srel targets 2 4 | | | 4 5 |      -- row 0 → {2,4}, rows 1-2 empty, row 3 → {4,5}
```

### `inv` — inverse read

`inv <name> <rel>`, the reverse of a functional `rel` materialized as an `srel` whose fibers compute at load (ascending sources per target). The inverse of a keyed rel is keyed automatically. Never dumped; recomputed on reload.

```
rel owner 2 0 2 4 1
inv owned owner          -- owned' gives each entity its set of owned rows
```

### `alias` — stored mask

`alias <name> <n-mask>`, a boolean mask given a name — a value, distinct from the name alias `as`. Read in selection position like any mask.

```
alias frontline 1 1 0 0 1
```

### `bind` — named constant

`bind <name> <kind> <values…>`, a constant of a declared kind: `entity` (one id), `num` (one number), `point` (two, an x,y pair usable as a frame origin), `mask` (`n` bits), `vec` (any-arity sequence).

```
bind cursor entity 3
bind impact point 12 4
bind pathCells vec 0 1 2 5 8 7
```

### `fn` — registered callable

`fn <name> [verbatim BQN body]`, a registered callable and the home of any true recurrence. The current registry stores a BQN body because CBQN is Steel's backend; the semantic signature must also declare argument/result habitats and footprints. A bodyless `fn` names a host-provided routine.

```
fn fib {𝕩≤1 ? 𝕩 ; (𝕊 𝕩-1) + 𝕊 𝕩-2}
fn shortestPath                    -- host-provided, dispatched via `via`
```

### `as` — name alias or derived tag

Two forms by arity. `as <word> <name>` is a pure name alias: one hop, no transitivity, outranked by real entries. `as <word> <col> <value>` is a derived tag, the word naming the live equality mask (present ∧ col = value), recomputed at each use, hops included. Num, bool, and sym carriers only; char and vec are rejected at load. A tag is read-only as an effect target — assignment, presence writes, and protos all reject, naming the carrier — and cannot be pinned by `--! expect`, which pins storage.

```
as HP health              -- name alias: HP resolves to column health
as Hostile faction 2      -- derived tag: the mask (faction == 2)
```

### `ja` — Japanese-surface alias

`ja <word> <name>`, filling the same one-hop alias table as 3-word `as` but flagging the row as documenting the Japanese surface at the declaration site. An alias works from either surface; a natively Japanese registry needs none.

```
ja 金 gold                -- 金 resolves to column gold
ja 源 source
```

### `role` — system role binding

`role <keys|id|parent|proto|pos> <col>`, pointing a built-in behaviour at a native, often kanji-named column so the emitter routes spawn, key, parent, or position machinery through it. Resolution ladder: the declared role line wins, else the literal role name resolves as an ordinary noun. The roles are `keys` and `id` (the minted stable-id column), `parent` (spawn lineage), `proto` (archetype pointer), and `pos` (position column for placement and frames).

```
role pos 位置             -- give the 位置 column the pos behaviour
role keys 番号            -- 番号 is the minted key column
```

### `def` — proto or archetype

`def <name> [<col>=<v> …]`, a registered archetype of named field values, consumed by `spawn <name>`. Values must sit in each column's carrier and range. Unique and key columns refuse fields, minted not defaulted; `vec` and `char` columns are unsupported as proto fields. This `def` is wholly separate from the program's `def`.

```
def Marine soldier=1 hp=100 gold=0
def Egg fragile=1
```

```haskell
Spawner , spawn Marine * Count      -- fills through the proto, then default, then zero
```

### `reap` — reclamation policy

`reap <seal|host>`, the world-level policy for `~` despawn reclamation, declared once. `seal`, the default, reaps at tick seal; `host` hands reclamation to the host. Schema-only in anoc — the mask-level meaning of `~` never changes, whichever is chosen.

```
reap seal
reap host
```
