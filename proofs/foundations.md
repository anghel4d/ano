# Foundations

**Status: TENTATIVE. 2026-07-02.** Everything below is believed true with proof sketches, not yet formally verified. Lean4 infrastructure is planned but not set up; nothing here is load-bearing until mechanized. Executable witnesses for the concrete counterexamples and identities live in `demos/tiers/`: `t1-reverse.bqn`, `t2-ties.bqn`, `t2-conjugation.bqn`, `t2-diagonal.bqn`. This document replaces the mathematical claims formerly in the spec's "The maths" and "Tiers and Algebras" sections; the spec keeps the English and the examples, the mathematics lives here.

## 1. Setting

The store is columnar. Fix a finite index set `I`, the keys of the live world. A component is a column `c : I → V`. An entity is a key; "the entity's components" are the entries under that key across columns. Different columns are defined on different subsets of keys; presence itself is data.

A selection is not an endofunction on `I`. Selection by predicate `P` is the **subobject inclusion** `ι : I_P ↪ I`, the mono picking out the satisfying keys. Writing it `σ_P : I → I` is a type error — there is no sensible value for `σ_P` off the mask, and composing two of them presumes a retraction that does not exist. A selection is a mask; masks have no multiplicity, which is why the image of a set-valued hop applies an effect once per reached target regardless of in-degree.

An effect is a write against pre-state. One statement is one gather-effect-scatter barrier: every read on the right of the comma observes the store as it stood at the statement's gather; the scatter commits at statement end. `;`-batched effects share the barrier and must commute under the registered merge laws. Nothing in this document evaluates mid-statement.

A datum carries no tier on its own. The tier is the **(operation, view)** pair: the same bits read as `V₀ = 𝔹` (an adjacency matrix) sit in Tier 1 under `+/ Adj@row`, and read as `V = Graph` (asserted traversal meaning) sit in Tier 3 under `shortestPath via Adj`. Promotion and demotion are re-viewing, not conversion.

## 2. Tier 1 — space

### 2.1 The real theorem: generable versus nominal keys

The index of space is **regenerable**: `I = ↕shape`, a definable key, recomputable from the shape alone at any time. The index of records is **nominal**: an allocated key, held only by the store, unrecoverable once dropped. This asymmetry is the whole content of "space is indestructible," and it is exactly why rank-changing index operations — `(¬m)/c`, reshape, fold to lower rank — are free over space and forbidden as record write-backs: over space the output index `J` is as definable as the input index was, so no address is lost that `↕` cannot remint; over records a dropped key is gone.

### 2.2 What the old proof got wrong

The spec's two-move proof of indestructibility does not hold and is recorded here so the failure is not re-invented.

Move 1 — "a write cannot consume its own domain" — is true but discriminates nothing: it holds for any keyed store, records included. A write `w : I → V` never acts on `I` whether `I` is a lattice or a set of entity IDs. A premise that both tiers satisfy cannot separate them.

Move 2 — "nothing refers to a space cell durably" — is false in the language's own examples. `Water = 100` at a cell, `+Cliff`, `+MiningNode`, and moisture diffusion all store state at cells and read it back across ticks. Cells are referred to durably; what is special is not that the figure is owed to no one, but that the ground under it is definable (2.1).

### 2.3 The refuted equivariance criterion

The spec asserted: a Tier-1 operation is legal exactly when it is `G₀`-equivariant, `f(g·c) = g·f(c)` for the geometry group `G₀` (translations, uniform scaling, the lattice rotations and reflections). Refuted twice over; witness `demos/tiers/t1-reverse.bqn`.

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

Relabel the entities and every column relabels together; the write-back must not notice. Witness `demos/tiers/t2-diagonal.bqn`.

### 3.2 Characterization

For a single column, `f : V^I → V^I` is `Sym(I)`-equivariant iff

```
f(c)_i = φ(c_i, ⟦c⟧)
```

for some `φ`, where `⟦c⟧` is the multiset of values. Sketch: the stabilizer of `i` acts as the full symmetric group on the remaining coordinates, so `f(c)_i` can depend on those only through their multiset; transitivity of `Sym(I)` forces one `φ` for every `i`. In the linear case over `ℝ^I` the commutant of the permutation representation is spanned by the identity and the all-ones matrix (Schur), so `f(c) = a·c + b·(Σc)·1` — pointwise work plus broadcast aggregates, nothing else.

### 3.3 Corollary: no canonical previous

Any order-dependent operator fails the law immediately: `prev` over an unordered record selection means `f(c)_i` depends on which key happens to precede `i`, and a `σ` that swaps neighbours breaks equivariance. So "you cannot draw a Fibonacci through gold held by Nords" is a theorem, not a taste judgment. The same argument independently derives the scan-ordering rule the spec already has: a scan over entities is legal only along a declared order (`scan(f) … along`), because the order is exactly the extra structure that shrinks `Sym(I)` to the trivial group and dissolves the obstruction.

### 3.4 Ties break the law; value-only rank repairs it

Stable tie-breaking and equivariance are mutually inconsistent. Witness `demos/tiers/t2-ties.bqn`. Take `c = [5,5,3]` and `σ = swap(0,1)`. Then `c ∘ σ = c`, so stable ascending rank gives `rank(c ∘ σ) = rank(c) = [1,2,0]`; but the law demands `rank(c) ∘ σ = [2,1,0]`. Stability breaks ties by index, and index-dependence is precisely what `Sym(I)`-equivariance forbids. The repair: Tier-2 write-backs use **value-only tie-breaking** — dense or fractional rank — under which tied values receive equal ranks and the counterexample dissolves. Stable grade remains available as a read (it exits the tier) and over space (where the index carries intrinsic order).

### 3.5 Conjugation, correctly

The spec said conjugation `σ⁻¹ ∘ f ∘ σ` reconciles free order-work with the alignment demand. For a fixed `σ` this is false: conjugating by a constant permutation is not `Sym(I)`-equivariant. The correct construction conjugates by the **data-derived grade** `σ_c` (the permutation sorting `c`):

```
h(c) = f(c ∘ σ_c) ∘ σ_c⁻¹        -- sort, act, unsort
```

`h` is equivariant precisely because of the identity

```
σ_{c ∘ τ} = τ⁻¹ ∘ σ_c
```

which holds exactly when tie-breaking is index-free (3.4): `(c∘τ) ∘ (τ⁻¹∘σ_c) = c∘σ_c` is sorted, and value-only ties make the grade a function of the values alone, so this is *the* grade of `c∘τ`. Then `h(c∘τ) = f(c∘σ_c) ∘ (σ_c⁻¹∘τ) = h(c) ∘ τ`. Witness `demos/tiers/t2-conjugation.bqn`. `Unit , Rank = rank(Gold)` is this construction; the equivariance of the whole line is inherited from the identity, and the identity is why the ties repair in 3.4 is not optional.

## 4. Tier 3 — opaque

### 4.1 The law: naturality in V

The spec claimed `∄ f : (I→V) → (·)` in the algebra — immediately contradicted by the two maps the same section admits, selection and dispatch. The correct statement: no admitted map **inspects** `V`. Formally, demand that every algebra map be **natural in V** — a family `f_V : (I→V) → (J→V)` natural in `V`, i.e. parametric, a free-theorem citizen.

Sketch. By Yoneda, natural transformations `Hom(I,−) ⇒ Hom(J,−)` correspond exactly to functions `u : J → I`, with `f(c) = c ∘ u`. So every natural map is a reindexing: the legal maps over an opaque column are **index manipulations** (precomposition by some `u`), **select** (the special case `u = ι : I_P ↪ I`, the subobject inclusion of §1), and **dispatch** (`h : V ⇝ host`, the typed hand-off at the algebra's boundary, admitted by the envelope, never by inspection). More directly: naturality against the map `V → 1` collapses any `f` that branches on values.

### 4.2 The view pun stays

The adjacency example is unchanged: `Node , OutDeg = +/ Adj@row` treats the bits as `𝔹`-columns and sits in Tier 1; `Hostile , shortestPath via Adj` asserts graph meaning, and by 4.1 no algebra map may act on it — the host runs Dijkstra and writes a column back. The functor asserting or stripping the meaning is the whole distance between the tiers.

## 5. The monotone chain

The repaired ladder is genuinely monotone along one axis:

- **Tier 1** — geometry of the index: unit and frame coherence at the de/at boundary (2.4). Weakest demand, widest algebra.
- **Tier 2** — full symmetry of the index: `Sym(I)`-equivariance under the diagonal action (3.1).
- **Tier 3** — full abstraction of the value: naturality in `V` (4.1). Strongest demand; only reindexing, select, dispatch survive.

The monotonicity principle stays as stated in the spec: for groups `H ⊆ G`, `Equiv_G ⊆ Equiv_H` — demand more symmetry, admit fewer maps. Tier 3 extends the principle past groups: naturality in `V` is invariance under *all* value substitutions, the limit of the demand, and the algebra it leaves is correspondingly the thinnest.

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

`12 , offset = prev.offset + prev.prev.offset` is one barrier step of a two-back **stencil**, not a recurrence. Under §1's evaluation rule every read observes pre-state, so the statement computes `new[i] = old[i−1] + old[i−2]` in parallel over the pre-state column; on a freshly minted line the pre-state of `offset` is undefined-or-zero, so the statement cannot generate Fibonacci. `prev` is a shift, not a carry; no statement may read what it is writing, and iterating the statement `k` times yields `k` stencil steps, never the order-carried sequence. Sequential order-carried evaluation is undefined and forbidden by the one-barrier model. The honest form is Version B, `offset = fib(index)`: the recurrence runs inside a registered host function, outside the calculus.

Whether the language should ever admit a true recurrence — a sequential `scan(f) along order` with non-associative `f`, legal only where the write footprint misses the read footprint — is open, recorded in the spec's Open Questions with the tradeoff surfaced. This document does not resolve it; it only establishes that the current evaluation rule makes the stencil reading the only sound one.

## 8. Totality by construction

The combinator core — reduce, scan, grade, outer product, replicate, reshape — is structural recursion over finite columns: every fold and scan consumes a finite carrier, every generator (`↕n`, `↕w‿h`) produces one, and the grammar admits no general fixpoint and no unbounded iteration. Totality is therefore a corollary of the grammar, by construction — a stronger claim than the borrowed eBPF framing, which is a verifier *checking* boundedness of programs a permissive syntax admits. Here the syntax never admits the unbounded program; there is nothing to verify. The rejected alternatives (general `iterate`, fixpoint rules, the recurrence of §7 as a native form) are rejected precisely to keep this a grammatical fact rather than an analysis.
