# Spatial mathematics

This document collects the spatial formalization developed during the 2026-07-15 audit, including the preliminary proposals, their failures, and the corrected model. `docs/ano-language.md` remains the language specification. This document gives the mathematics behind its habitats, fields, relations, effects, lattices, and columnar lowering.

## 1. The preliminary proposal and its correction

The first proposal was to define a field by an origin and basis vectors of arbitrary dimension. That is too much structure for a field and too little structure for a placed space.

A field needs only a domain and values. For a finite habitat `H` and carrier `V`, a total field is a function:

```text
f : H → V
```

The origin and basis do not belong to `f`. They belong to an optional placement of a structured habitat into an ambient affine space. A health component, a chessboard glyph field, and a heightmap are all fields in this sense; only some carry spatial structure.

The minimal semantic object is therefore not “origin plus basis.” It is a named habitat `H`. Structure is added only when an operation requires it.

## 2. Habitats

A habitat is a finite index type whose elements identify the places at which a column may vary. Two habitats with the same cardinality remain different.

```text
H ≠ K  even when  |H| = |K|
```

`Health : E → Num` and `Elevation : Ground → Num` do not align merely because both buffers contain 64 numbers. Alignment is semantic identity or an explicit map, never equal length.

A habitat is not necessarily spatial. Current entities, present instances of one component, relation edges, event rows, history rows, lattice cells, and derived query rows are all habitats.

A named stored habitat is persistent schema. A derived habitat is a value produced by a query operation. The distinction is lifetime and authority, not physical representation.

## 3. Physical layout

Columnar execution requires an enumeration of a finite habitat. A layout is a bijection:

```text
ℓ_H : Fin(n) ≅ H
```

A semantic column `c : H → V` is stored as the dense buffer:

```text
buffer_c[i] = c(ℓ_H(i))
```

The buffer and `n` are insufficient metadata. The runtime must also know which habitat the layout enumerates. Alternative layouts of one habitat are related by a permutation, but a permutation between layouts is not an identification between foreign habitats.

This preserves structure-of-arrays execution. Habitats are compile-time or plan-time descriptors; buffers, masks, offsets, and lineage maps remain ordinary contiguous arrays.

## 4. ECS components are partial fields

Let `E` be the finite habitat of currently live entity identities. An ECS component `C` is generally partial on `E`.

```text
P_C ↪ E
c   : P_C → V_C
```

`P_C` is the presence habitat and the inclusion says which entities own the component. The value buffer is total and dense on `P_C`.

Equivalently, the component may be viewed extensionally as:

```text
c? : E → Option(V_C)
```

The presence-habitat form is the better column-store representation. It separates dense values from cross-component raggedness.

A spatial field is ordinarily total on its declared habitat:

```text
Water : Ground → Num
```

Sparse spatial data is still possible, but its sparsity must be explicit as a subobject or relation. It must not arise from accidental buffer truncation.

## 5. Query views and the current row habitat

The central execution object is a query view. A view has a finite row habitat `X` and lineage maps to every stored habitat it touches.

```text
View X
λ_E      : X ⇀ E
λ_Ground : X ⇀ Ground
λ_R      : X ⇀ R
```

A mandatory lineage is total. Optional or left-joined data uses a partial map and validity mask.

Every expression consumed together by a column kernel is aligned on `X`:

```text
Col(X,V) = X → V
```

A scalar `a : V` broadcasts as the constant column `x ↦ a`. A predicate is `X → Bool`. A pair-valued column remains one item per `X`; item shape belongs to `V`, not to the habitat.

Selection by `p : X → Bool` creates the subobject:

```text
X_p = { x : X | p(x) }
ι_p : X_p ↪ X
```

Every existing lineage composes with `ι_p`. This is how a masked write can later return to the correct stored rows.

Equal row counts do not establish a shared `X`. The view descriptor does.

## 6. Reindexing is gather

For a map `u : J → I` and a column `c : I → V`, pullback or reindexing is:

```text
u* c = c ∘ u : J → V
```

Physically this is a gather by the integer vector implementing `u`.

Identity and composition give the basic laws:

```text
id* c = c
(v ∘ u)* c = u*(v* c)
```

Exact selection, permutation, layout conversion, functional relationship hops, and exact reshape are instances of reindexing.

A partial map `u : J ⇀ I` lowers to a gather index plus validity mask. Invalid rows fail a predicate or propagate null according to the operator contract.

## 7. Relations are spans

A general relationship is not a function. For endpoint habitats `A` and `B`, use an edge habitat `R` and a span:

```text
A ←src— R —dst→ B
```

A functional relationship is the special case in which each source has at most one live outgoing edge. A set-valued hop keeps `R`, or a view derived from `R`, until an image, quantifier, or grouped fold consumes it.

A boolean matrix `M : A × B → Bool` represents the same extensional relation:

```text
R_M = { (a,b) | M(a,b) }
```

The edge-list and matrix forms are alternative physical presentations. Neither makes `A` and `B` the same habitat.

The inverse relationship swaps the span legs. No new semantic edge set is required.

## 8. Fiber aggregation

For `u : J → I`, the fiber over `i` is:

```text
u⁻¹(i) = { j : J | u(j) = i }
```

Given `x : J → M`, a grouped reduction produces a column on `I`:

```text
(u_! x)(i) = ⊕ { x(j) | u(j) = i }
```

For an unordered or parallel fiber, `(M,⊕,e)` must be a commutative monoid. Associativity permits regrouping; the identity handles an empty fiber; commutativity removes dependence on edge order.

For a declared ordered fiber, associativity and identity are sufficient. Noncommutative folds may then respect that order.

`avg` is not a primitive associative binary operation on averages. It reduces sufficient statistics:

```text
(sum,count) ⊕ (sum',count') = (sum+sum', count+count')
finish(sum,count) = sum / count
```

The statistic is a commutative monoid. `finish` is undefined at count zero unless the language declares another policy.

Grouped relationship folds, in-degree, neighborhood sums, and collision merges all share this construction.

## 9. Effects are scatter with law

An effect evaluated on query habitat `X` carries:

```text
target      H → V
destination d : X ⇀ H
values      v : X → V
merge       μ
```

Plain assignment is deterministic when `d` is injective on valid rows. If several source rows reach one destination, assignment must refuse or use an explicit conflict resolver.

Commutative effects such as addition, min, max, boolean or, and boolean and may accept collisions when their registered merge law is associative, commutative, and has the required identity. Each destination fiber is reduced before scatter.

A masked update is the common injective case:

```text
X ↪ H
```

Gather occurs through the inclusion and scatter returns through the same inclusion.

The target habitat is invariant under an ordinary value effect:

```text
before : H → V
after  : H → V
```

Values change. `H` does not.

## 10. The barrier

A compiled statement is a pure plan over one snapshot. Every read observes the pre-state, effects accumulate, and commit occurs once.

```text
plan    : World(Σ) → EffectBuffer × Output
perform : World(Σ) → Result(World(Σ) × Output)
```

The schema `Σ` is fixed across the call. The set of live entities inside the world may change. A registered field habitat named by `Σ` remains fixed unless a separate schema operation explicitly replaces it.

The barrier gives snapshot semantics. It does not by itself prove destination injectivity, affine access, absence of relationship aliasing, or deterministic collision behavior. Those require lineage and algebraic evidence.

## 11. Structural effects

Spawn driven by counts `count : X → Nat` creates the dependent sum:

```text
Copies = Σ(x : X). Fin(count(x))
```

Each copy remembers its source:

```text
source : Copies → X
```

Allocation then creates fresh nominal entity keys:

```text
alloc : Copies ↪ E_after
```

Spawned values gather through `source`; positions gather through the source's placement lineage. A cell index is not an entity parent. A parent component may be copied only if `X` carries entity lineage and the program requests that relationship.

Despawn removes live entity identities and component-presence rows according to generational-key rules. It cannot shrink an unrelated field habitat.

Replication changes the result habitat. It never changes the habitat of the count column that drove it.

## 12. Shape-changing reads

Shape belongs to a habitat presentation, not to a raw value buffer.

There are three distinct operations.

Exact reshape between equal-cardinality presentations is an equivalence:

```text
q : D' ≅ D
f' = f ∘ q : D' → V
```

Cycling or truncating APL reshape uses an output-to-input map:

```text
q : D' → D
f' = f ∘ q
```

It is a gather, not an equivalence. It may duplicate or omit input positions.

Ano placement such as `pos = to 4 16` derives one position value per row of the current query habitat `X`:

```text
position : X → Point
```

It does not reshape `X`, the live entity habitat, or a stored field.

A rank-changing result may be queried, folded, passed to a callable, or materialized as a new value on its own habitat. It cannot silently overwrite a stored target on another habitat.

## 13. Lattices

An abstract lattice of rank `r` is a free integer module:

```text
Λ ≅ ℤ^r
```

A finite dense lattice field uses a finite box:

```text
D = ∏_{j=0}^{r-1} Fin(n_j)
```

and a chart:

```text
κ : D → Λ
```

The field is:

```text
f : D → V
```

The shape `(n_0,…,n_{r-1})` and rank `r` belong to `D`. They are invariant under value updates to `f`.

The abstract lattice and finite box are distinct. `Λ` supplies translations and offsets. `D` supplies the bounded stored cells.

A bare numeric shape may create an anonymous `D`. It cannot select a registered field solely by matching rank, shape, or cardinality.

## 14. Affine placement

An affine space `A` is a torsor for a vector space or module `T`. Points may be subtracted to obtain vectors; vectors may be added to points; points are not canonically vectors.

A lattice placement requires:

```text
o : A
β : Λ → T
χ(d) = o + β(κ(d))
```

`o` is an origin point for this placement. `β` maps abstract lattice generators to translation vectors. The familiar basis vectors are the images of those generators.

The declaration:

```text
origin 100 0 200
```

has no intrinsic meaning. It becomes meaningful only after the parent ambient affine space has dimension three, coordinate axes, scalar carrier, units, and a coordinate chart. It then denotes the point whose coordinates are `(100,0,200)` in that chart.

An abstract lattice needs no origin. A stored field needs no origin. A placement needs an origin.

## 15. What Ground means

`Ground` is not a space merely because it is 2D. It is spatial only if the registry gives it spatial structure.

Possible declarations include:

```text
Ground is a finite habitat
Ground has product shape 64×64
Ground has a rank-2 lattice chart
Ground has an affine placement in World3
Ground has a metric
Ground has a Moore neighborhood relation
Ground has a boundary policy
```

Each statement is independent. A 2D inventory table has product shape but need not be spatial. A road graph is spatial without being a dense 2D lattice. A voxel field may be a 3D lattice. A one-dimensional path may have placement and metric.

The word “ground” is a name, not a proof.

## 16. Neighborhoods and boundaries

A neighborhood is a relation:

```text
N ↪ D × D
```

A single directional neighbor may be a partial map. A stencil is generally set-valued and should remain a relation until its fiber is folded.

Boundary behavior is part of the neighborhood or field extension.

Shrink uses a partial relation and gives smaller boundary fibers.

Constant extension evaluates out-of-domain cells at a declared constant.

Clamp is a total map from attempted coordinates to the nearest in-domain coordinate.

Reflect is a total map obtained by reflection.

Wrap requires modular or quotient structure and gives a toroidal or periodic habitat.

These policies differ at corners, under repeated offsets, and for aggregation multiplicity. No policy is the universal default.

Conway's Life with dead borders uses shrink or constant-false extension. Toroidal Life uses wrap. They are different habitats or neighborhood structures, even when the rule expression is identical.

## 17. Metrics and topology

Distance is additional structure:

```text
dist : H × H → R_{\ge 0}
```

It requires the metric laws if operations depend on them. An affine placement does not automatically choose Euclidean distance, and a lattice chart does not automatically choose Manhattan distance.

Incidence and topology require more than points. A cell complex or cubical complex may declare cells of several dimensions and boundary maps:

```text
∂_k : C_k → C_{k-1}
∂ ∘ ∂ = 0
```

This supports contours, connected boundaries, surfaces, and homology. It is not required for ordinary fields or stencils. Ano should make no topological assumption until an operation asks for it.

## 18. Capability structure

The retired tiers are replaced by independent capabilities.

```text
finite(H)
ordered(H)
product(H,A,B)
lattice(H,Λ,κ)
placed(H,A,o,β)
metric(H,dist)
complex(H,C,∂)
relation(R,A,B,src,dst)
monoid(V,⊕,e)
commutative(V,⊕)
host_callable(f,signature,footprint)
```

Capabilities compose by conjunction. They do not form one linear hierarchy.

An operation's type states the evidence it needs. Scan needs order. Transpose needs product structure. Stencil needs a lattice chart and boundary. Radius needs a metric. Grouped parallel fold needs a commutative monoid. Host dispatch needs a registered signature and footprint.

The type is the license for the operation.

## 19. Why the former Tier 1 proof failed

The old argument said spatial indices were regenerable, therefore rank-changing operations were free.

Coordinates are regenerable:

```text
D ≅ ∏ Fin(n_j)
```

Persistent field state is not:

```text
f : D → V
```

Recreating `D` does not recreate the association between each `d` and `f(d)` after values have been filtered, graded, replicated, or reshaped. A derived read may change domain. A write into `f` must return through a destination map to `D`.

The valid theorem is:

```text
ordinary field update : (D → V) → (D → V)
```

not:

```text
arbitrary array operation : (D → V) → (D' → V)
```

The latter produces a new value, not an update to the old field.

## 20. Why the former Tier 2 proof was not universal

Permutation equivariance is a useful law for operations that claim to ignore labels:

```text
F(c ∘ σ) = F(c) ∘ σ
```

Ano does not universally ignore labels. It exposes generational keys, `index`, explicit singleton bindings, relationships, declared order, placement, and structural allocation. These structures restrict which permutations are symmetries.

The correct use is local. An optimizer may use equivariance when an operation and view certify that the relevant relabelling preserves every observed structure.

Stable grade intentionally observes an order witness. A fixed relationship intentionally observes edges. Neither is illegal. Their extra structure simply invalidates the maximal symmetry premise.

## 21. Why the former Tier 3 proof was too broad

For a genuinely parametric family:

```text
F_V : (I → V) → (J → V)
```

natural in every carrier `V`, Yoneda-style reasoning characterizes `F` as reindexing along a map `J → I`.

That theorem applies only when `F` is parametric in `V`. An opaque host operation is usually not parametric: a pathfinder inspects a navmesh, and a behavior-tree runner inspects nodes. Opacity means the native Ano algebra does not contain that operation. It does not mean the host operation cannot inspect the carrier.

Opaque values remain columns on habitats. Dispatch is a registered capability, not a third tier.

## 22. Time and repeated evaluation

Let `Step` perform one statement or rule batch:

```text
Step : World(Σ) → Result(World(Σ) × Output)
```

Repeated ticks compose `Step` sequentially. The entity habitat may grow or shrink at each step. Each registered field habitat remains the one declared by `Σ`.

A spatial test is incomplete if it checks only the first post-state. At minimum it must check:

```text
rank(field_t+1)  = rank(field_t)
shape(field_t+1) = shape(field_t)
habitat(field_t+1) = habitat(field_t)
```

and then execute another step using that field.

The computed-line and spatial top-k sentinels exposed the old failure: the first step produced plausible values while growing an entity-row stand-in; the second step encountered incompatible lengths. The test must preserve and reconsume the same semantic habitat.

Conway's current witness survives because an explicit Moore relation reconstructs the required fibers over a fixed field. That proves the barrier and relation behavior in that fixture. It does not prove a general habitat type system.

## 23. Columnar realization

The formal objects lower directly to column-store data.

| Semantic object | Columnar representation |
|---|---|
| habitat `H` | domain ID, cardinality, layout descriptor |
| `Col(H,V)` | typed dense buffer |
| subobject `X ↪ H` | mask or compact row-index vector |
| partial map `X ⇀ H` | index vector plus validity bitmap |
| relation span | edge buffer plus source and target index columns |
| query view `X` | aligned buffers plus lineage vectors |
| fiber decomposition | offsets plus grouped edge indices |
| scalar broadcast | scalar kernel argument or repeated view |
| dependent sum | prefix sum of counts plus source-row vector |
| effect | target domain, destination vector, value buffer, merge tag |
| product habitat | shape, strides, axis metadata |
| lattice chart | rank, shape, coordinate transform |
| affine placement | ambient ID, origin point, basis map |

No row objects are required. No per-value habitat tags are required inside a homogeneous kernel. A plan carries domain evidence once and passes raw arrays to the kernel.

This is an array language precisely because the denotation gives the compiler aligned finite families before lowering.

## 24. Proof obligations

A conforming plan must establish the following obligations.

Habitat identity: every consumed column is scalar or aligned on the current query habitat.

Layout validity: every layout is a bijection between physical rows and its semantic habitat.

Lineage preservation: selection, grade, replicate, relationship hops, and reshape produce the correct maps to their source habitats.

Target compatibility: every effect destination lands in the target habitat.

Assignment uniqueness: plain assignment has at most one value per destination unless a resolver is declared.

Merge law: collision reduction uses the registered associative and commutative law required by its execution order.

Field invariance: ordinary field effects preserve habitat, rank, and shape.

Structural separation: entity allocation and reclamation do not resize unrelated habitats.

Boundary totality: every neighborhood access is either valid, masked invalid, or resolved by the declared boundary policy.

Tick closure: every successful post-state satisfies the same schema and can be consumed by the next tick.

Refusal atomicity: failure commits neither values nor structural changes.

## 25. Executable laws

The BQN siblings are executable mathematical witnesses, not semantic oracles. Their useful assertions are laws such as:

```text
shape(after masked field update) = shape(before)
field habitat cardinality is unchanged by spawn
spawn row count = sum(counts on selected source rows)
source projection has one entry per spawned row
exact reshape preserves cardinality
cycling reshape records an output-to-input gather
neighborhood boundary matches its named policy
Life step two consumes the output of Life step one
foreign equal-length columns are not treated as aligned
```

A useful negative witness constructs two length-equal columns on foreign habitats and shows that no well-typed binary expression exists without an explicit map.

A useful positive witness constructs `X ↪ Ground`, gathers `Water` onto `X`, computes values on `X`, and scatters through the same inclusion. The output remains `Ground → V`.

## 26. The concise model

The entire model can be summarized as follows.

```text
schema names habitats
stored columns vary over one habitat
queries construct a current row habitat
lineage maps current rows to stored habitats
array kernels consume columns aligned on current rows
relations are spans
folds reduce fibers
effects scatter through destination maps
structural effects create fresh entity identities
spatial structure is optional capability
stored field rank never changes by ordinary update
```

Ano is linear algebra in its column kernels, relational algebra in the construction of query habitats and lineage, and affine or topological mathematics only where the registry supplies that additional structure.
