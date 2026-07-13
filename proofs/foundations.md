# Foundations

**Status: TENTATIVE. 2026-07-02.** These are claims and hand-arguments, not proofs or verified results — nothing here is a theorem and nothing here is load-bearing. Executable counterexamples live in `demos/7-tiers/`. Lean4 work has not started.

## 1. Setting

The store is columnar. Fix a finite index set `I`, the keys of the live world. A component is a column `c : I → V`. An entity is a key; "the entity's components" are the entries under that key across columns. Different columns are defined on different subsets of keys; presence itself is data.

A selection is not an endofunction on `I`. Selection by predicate `P` is the **subobject inclusion** `ι : I_P ↪ I`, the mono picking out the satisfying keys. Writing it `σ_P : I → I` is a type error — there is no sensible value for `σ_P` off the mask, and composing two of them presumes a retraction that does not exist. A selection is a mask; masks have no multiplicity, which is why the image of a set-valued hop applies an effect once per reached target regardless of in-degree.

An effect is a write against pre-state. One statement is one gather-effect-scatter barrier: every read on the right of the comma observes the store as it stood at the statement's gather; the scatter commits at statement end. `;`-batched effects share the barrier and must commute under the registered merge laws. Nothing in this document evaluates mid-statement.

A datum carries no tier on its own. The tier is the **(operation, view)** pair: the same bits read as `V₀ = 𝔹` (an adjacency matrix) sit in Tier 1 under `+/ Adj@row`, and read as `V = Graph` (asserted traversal meaning) sit in Tier 3 under `shortestPath via Adj`. Promotion and demotion are re-viewing, not conversion.

## 2. Tier 1 — space

### 2.1 The real distinction: generable versus nominal keys

The index of space is **regenerable**: `I = ↕shape`, a definable key, recomputable from the shape alone at any time. The index of records is **nominal**: an allocated key, held only by the store, unrecoverable once dropped. This asymmetry is the whole content of "space is indestructible," and it is exactly why rank-changing index operations — `(¬m)/c`, reshape, fold to lower rank — are free over space and forbidden as record write-backs: over space the output index `J` is as definable as the input index was, so no address is lost that `↕` cannot remint; over records a dropped key is gone.

### 2.2 What the old proof got wrong

The spec's two-move proof of indestructibility does not hold and is recorded here so the failure is not re-invented.

Move 1 — "a write cannot consume its own domain" — is true but discriminates nothing: it holds for any keyed store, records included. A write `w : I → V` never acts on `I` whether `I` is a lattice or a set of entity IDs. A premise that both tiers satisfy cannot separate them.

Move 2 — "nothing refers to a space cell durably" — is false in the language's own examples. `Water = 100` at a cell, `+Cliff`, `+MiningNode`, and moisture diffusion all store state at cells and read it back across ticks. Cells are referred to durably; what is special is not that the figure is owed to no one, but that the ground under it is definable (2.1).

### 2.3 The refuted equivariance criterion

The spec asserted: a Tier-1 operation is legal exactly when it is `G₀`-equivariant, `f(g·c) = g·f(c)` for the geometry group `G₀` (translations, uniform scaling, the lattice rotations and reflections). Refuted twice over; witness `demos/7-tiers/t1-reverse.bqn`.

Reverse conjugates translations rather than commuting with them: `rev(shift_t c) = shift_{−t}(rev c)`. And reverse is a reflection, non-central in the dihedral group `D₄`, so it fails to commute with the lattice rotations the spec itself puts in `G₀`. Yet `⌽` is a canonical Tier-1 operation. So the criterion excludes operations the tier must admit.

Worse, the criterion is ill-typed for the operations the tier exists to license. Mask-filter and reduce produce an output index `J` that is a proper — for mask-filter, data-dependent — subset of `I`, and there is no `G₀`-action on `J` against which to state `f(g·c) = g·f(c)`. Equivariance under the geometry group is not the law of Tier 1.

### 2.4 What survives

Two things, both checks at the de/at boundary rather than laws over morphisms. The **frame check**: `pos = φ(k) = o + S·k`, the affine map from lattice key to world coordinate, with `@` fixing `(o, S)` per the resolved frames rule. The **counter identity**: `[world] = [world] + [world/cell]·[cell]`, unit-consistency of the frame equation, which is what the counter-typed numeral (`3mo`, `64 64`) enforces. Tier 1's demand is unit and frame coherence at the boundary, not symmetry of the operator.

## 3. Tier 2 — records

### 3.1 The law: diagonal action on the whole record

The spec typed a write-back `w : (I→V) → (I→V)`, one column in, same column out. That signature outlaws its own flagship line — `Nord & TwoHanded > 60 , Gold += 1000` reads `Nord` and `TwoHanded` and writes `Gold`, three columns. The correct statement uses the **diagonal action** of `Sym(I)` on the whole per-entity record:

```
w : (I → V₁ × ⋯ × V_k) → (I → V_j)
w(ρ ∘ σ) = w(ρ) ∘ σ    for all σ ∈ Sym(I)
```

Relabel the entities and every column relabels together; the write-back must not notice. Witness `demos/7-tiers/t2-diagonal.bqn`.

### 3.2 Characterization

For a single column, `f : V^I → V^I` is `Sym(I)`-equivariant iff

```
f(c)_i = φ(c_i, ⟦c⟧)
```

for some `φ`, where `⟦c⟧` is the multiset of values. Sketch: the stabilizer of `i` acts as the full symmetric group on the remaining coordinates, so `f(c)_i` can depend on those only through their multiset; transitivity of `Sym(I)` forces one `φ` for every `i`. In the linear case over `ℝ^I` the commutant of the permutation representation is spanned by the identity and the all-ones matrix (Schur), so `f(c) = a·c + b·(Σc)·1` — pointwise work plus broadcast aggregates, nothing else.

### 3.3 Consequence: no canonical previous

Under the proposed equivariance law, `prev` over an unordered record selection depends on an unstated key order and fails under relabeling. Entity scans therefore need a declared order such as `scan(f) … along`.

### 3.4 Ties break the law; value-only rank repairs it

Stable tie-breaking and equivariance are mutually inconsistent. Witness `demos/7-tiers/t2-ties.bqn`. Take `c = [5,5,3]` and `σ = swap(0,1)`. Then `c ∘ σ = c`, so stable ascending rank gives `rank(c ∘ σ) = rank(c) = [1,2,0]`; but the law demands `rank(c) ∘ σ = [2,1,0]`. Stability breaks ties by index, and index-dependence is precisely what `Sym(I)`-equivariance forbids. The repair: Tier-2 write-backs use **value-only tie-breaking** — dense or fractional rank — under which tied values receive equal ranks and the counterexample dissolves. Stable grade remains available as a read (it exits the tier) and over space (where the index carries intrinsic order).

### 3.5 Conjugation, the intended construction

Conjugating by a *fixed* `σ` does not reconcile order-work with the alignment demand: a constant permutation is not `Sym(I)`-equivariant. The construction the design intends instead conjugates by the **data-derived grade** `σ_c` (the permutation sorting `c`):

```
h(c) = f(c ∘ σ_c) ∘ σ_c⁻¹        -- sort, act, unsort
```

The intent is that `h` respects relabeling on tie-free data, resting on `σ_{c∘τ} = τ⁻¹ ∘ σ_c`, which needs the grade to ignore index (value-only ties, 3.4). `demos/7-tiers/t2-conjugation.bqn` exercises it on distinct keys and `Unit , Rank = rank(Gold)` is this shape. None of this is proved — it is a hand-argument checked on cases, not a theorem. The tied case has no such grade at all: a swap of equal values fixes the input while permuting their indices, so general sort-act-unsort needs unique keys or an operation defined on tie blocks. Value-only rank stays fine regardless, returning equal values for ties. A 2026-07-13 review called the tie-free argument unsound but asserted it rather than exhibiting a counterexample; asserting is not disproving, so the construction stays as intent, unproven either way.

## 4. Tier 3 — opaque

### 4.1 The law: naturality in V

The native algebra does not inspect `V`. For fixed index sets, require each internal map `f_V : (I→V) → (J→V)` to be natural in `V`.

By Yoneda, a fixed natural transformation `Hom(I,−) ⇒ Hom(J,−)` is reindexing by some `u : J → I`. Fixed-mask selection is such a reindexing. Value-dependent selection needs its predicate structure stated separately. Dispatch exits the native algebra through the registry and is not a Yoneda corollary.

### 4.2 The view pun stays

The adjacency example is unchanged: `Node , OutDeg = +/ Adj@row` treats the bits as `𝔹`-columns and sits in Tier 1; `Hostile , shortestPath via Adj` asserts graph meaning, and by 4.1 no algebra map may act on it — the host runs Dijkstra and writes a column back. The functor asserting or stripping the meaning is the whole distance between the tiers.

## 5. Three obligations

The current tiers use different obligations:

- **Tier 1** — geometry of the index: unit and frame coherence at the de/at boundary (2.4).
- **Tier 2** — full symmetry of the index: `Sym(I)`-equivariance under the diagonal action (3.1).
- **Tier 3** — abstraction of the value: naturality in `V` for internal fixed-index maps (4.1).

These obligations do not yet form one proved monotone chain.

## 6. The algebra

### 6.1 σ, ⋈, π do not suffice; γ is primitive

The selection sublanguage was claimed to be σ (predicate), ⋈ (relationship hop), π (component access). That algebra cannot express grouped or correlated aggregation, and three farm lines need it:

```haskell
Pen , Headcount = #/ (livestock' & Cattle)
Plot & !Planted & #/ (neighbors' & Planted) >= 2 , +Planted
Plot , Moisture = avg/ neighbors'.Moisture
```

So **γ**, the grouped fold, is first-class. Let `r : I → J` be a functional relationship; its fibers `r⁻¹(j)` partition the sources over the targets. For a fold `f` and column `c`:

```
γ_f(r, c)_j = fold_f { c_i : i ∈ r⁻¹(j) }
```

one value per target `j`, the result a column aligned to the target selection, written back under the ordinary Tier-2 alignment rule. A forward set-valued relationship is the transpose reading of the same relation; on the surface the fiber at the selected entity is `r'` (the tick), `r'.Comp` gathers across it, `r' & pred` filters it, and a fold prefix collapses it. `fold/ col @ scope` remains the scoped-global fold and is always one scalar; `@` never groups. This is q's `by` and Datalog's grouped aggregation, expressed as fold-under-each over fibers.

Empty fiber: the fold's registered identity when it has one (`#/` and `+/` give 0, `|/` false, `&/` true); a reducer with no identity (`avg`, `max/`, `min/`) fails the row — the left-join-null rule extended from the dangling link to the empty fiber, the entity drops out of the selection and no write lands.

### 6.2 A reduction is a catamorphism, not a projection

The spec called a reduction a projection. False in every standard sense: `+/` is not idempotent, is not an endomorphism (`ℝⁿ → ℝ` changes the carrier), and relational π never aggregates — `SELECT SUM` is γ, not π. A reduction is a **fold**, a catamorphism over the finite column. The true content survives untouched: reads are non-destructive; a fold consumes nothing, the column persists.

### 6.3 avg and count are fold-and-finish, not reductions

§12's reduction contract demands an associative operator with an identity. `avg/` is not associative and has no identity; `#/` is not a binary operator at all. Both are **derived forms**: a fold to an intermediate followed by a finisher. `avg = finish(÷) ∘ fold(+,0) △ fold(+1∘const,0)` — sum and count in one pass, divide at the end; `count = fold(+,0) ∘ map(const 1)`. The surface keeps `avg/` and `#/`; the registry records them as fold-and-finish so the empty-fiber rule of 6.1 can distinguish "identity exists" from "row fails."

### 6.4 Comprehension is the θ-join

`[dist(t,c) | t <- Tower, c <- Creep, dist < 50]` is a filtered cross join — `σ_p(Tower × Creep)`, a θ-join in the bag monad. The identification of the double-generator comprehension with `σ_p(A × B)` is correct; the attribution is Trinder and Wadler (1989, 1991: comprehension syntax as relational queries) and Buneman, Libkin, Suciu, Tannen, Wong (1994: nested relational calculus and comprehension syntax), not "Comprehending Monads." And it is not a dependent join: dependent/LATERAL means the second generator's domain is a function of the first, `b <- f(a)`; the independent double generator is the plain product.

## 7. Recurrences and the barrier

`12 , offset = prev.offset + prev.prev.offset` is one barrier step of a two-back stencil. Every read observes pre-state, so `prev` shifts the old column rather than carrying a staged value. Iteration yields stencil steps, not Fibonacci. A read-side scan may carry an internal accumulator and scatter its completed result without observing staged writes. The current Fibonacci form is `offset = fib(index)`, with the recurrence inside a registered function.

Whether to admit a sequential `scan(f) along order` with non-associative `f` remains open. Its computation would stay read-side, require a terminating step, and may scatter the completed column.

## 8. Totality of the native calculus

The native combinators operate over finite columns. The grammar admits no general fixpoint or unbounded iteration, so one native statement terminates over finite inputs. Registered functions and the host scheduler cross this boundary. Whole-program totality is relative to the Ground Registry.
