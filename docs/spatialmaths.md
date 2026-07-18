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

For rank `r`, a shape and its finite box are:

```text
Shape(r) = Fin(r) → Nat
Box(n)   = ∏_{j : Fin(r)} Fin(n(j))
```

A named habitat `H` has a nominal site type `D_H`. A product presentation is registered as an equivalence:

```text
b_H : Box(n_H) ≅ D_H
```

The name remains part of the type. If `Earth` and `Mars` both have shape `1024×512`, then `D_Earth` and `D_Mars` are still foreign. Their two product presentations make a coordinate-matching bijection mathematically possible; they do not register it as query lineage or authorize a cross-write. A physical layout `Fin(N) ≅ D_H` is separate again: `b_H` explains axes and shape, while the physical layout explains buffer order.

An abstract lattice of rank `r` is a free integer module `Λ_H ≅ ℤ^r`. A lattice habitat registers a chart and its partial inverse:

```text
κ_H      : D_H → Λ_H
lookup_H : Λ_H → Option(D_H)
```

They obey:

```text
lookup_H(κ_H(d)) = some(d)
lookup_H(z) = some(d)  implies  κ_H(d) = z
```

The chart is therefore injective. Its coordinate presentation agrees with `b_H`: the coordinates of `κ_H(b_H(q))` are the integer embeddings of the bounded coordinates `q(j)`. `lookup_H` recognizes exactly the finite window and makes an attempted displacement either an in-habitat site or explicit failure.

A field is `f : D_H → V`. The rank and shape belong to the registered product presentation of `D_H`, not to `f`, and are invariant under value updates to `f`. `Λ_H` supplies translations and offsets; `D_H` supplies the bounded stored cells.

A bare numeric shape may create an anonymous habitat and box. It cannot select a registered field solely by matching rank, shape, or cardinality.

## 14. Affine frames and placement

A frame `F` is nominal. It supplies distinct carriers `Point(F)` and `Vector(F)`. `Vector(F)` is an additive commutative group, and `Point(F)` is a torsor for it. With point-vector addition `+ᵥ` and point subtraction `-ᵥ`, the load-bearing laws are:

```text
p +ᵥ 0 = p
(p +ᵥ u) +ᵥ v = p +ᵥ (u + v)
(p +ᵥ v) -ᵥ p = v
p +ᵥ (q -ᵥ p) = q
```

Points cannot be added to points, and a vector in frame `F` cannot be added to a point in frame `G`. A coordinate presentation of `F` additionally declares its scalar carrier, dimension, axes, units, and coordinate maps. Equal dimension and equal scalar representation do not identify frames.

These equations are laws of the semantic carrier. IEEE floating-point coordinate buffers may implement a deterministic approximation without honestly supplying exact associativity or cancellation; compiler rewrites that use the torsor equations require the corresponding exact or error-bounded witness.

An affine map `T : F → G` has a point map `T_P : Point(F) → Point(G)` and an additive vector part `T_V : Vector(F) → Vector(G)`, linear when the declared scalar actions agree, satisfying:

```text
T_P(p +ᵥ v) = T_P(p) +ᵥ T_V(v)
```

Affine maps compose by composing their point and vector parts. An affine equivalence additionally supplies inverse round trips. Translation, rotation, scaling, and shear are instances only with the laws their declarations actually provide; zero scaling is not an equivalence, and an arbitrary affine map does not preserve distance or nearest points. Such conclusions require an explicit isometry or metric-preservation witness.

A general placement is only a typed map:

```text
Placement(D_H,F) = D_H → Point(F)
```

This includes curved and host-defined placements. An affine placement is the structured special case:

```text
o : Point(F)
β : Λ_H → Vector(F)
χ(d) = o +ᵥ β(κ_H(d))
```

The linear part `β` is additive. When the declaration promises an embedding, it must also be injective; a general placement need not make that promise. The familiar basis vectors are the images of the lattice generators under `β`.

If `T : F → G` is affine, transport of placement is `T_P ∘ χ`. Identity transport leaves a placement unchanged, and transporting first by `T` and then by `U` equals transport by `U ∘ T`. This is the planet-local-to-system composition law. Changing or transporting a placement leaves every field `D_H → V` and the habitat `D_H` unchanged.

The declaration `origin 100 0 200` has no intrinsic meaning. It becomes meaningful only after a three-dimensional frame, scalar carrier, axes, units, and coordinate presentation are declared. An abstract lattice needs no origin. A stored field needs no origin. A placement needs an origin.

## 15. Localization and typed cell references

A point-bearing ECS component remains a partial column on entities:

```text
P_pos ↪ E
pos : P_pos → Point(F)
```

Its item carrier names `F`; its row habitat remains `P_pos`. It does not become a field on `D_H`, and the word `pos`, a three-element item shape, or the same buffer length as `D_H` supplies no alignment.

A cell reference is typed by its habitat:

```text
CellRef(H) = D_H
```

The registry and typed plan construct such references. A flat row offset, lattice coordinate, entity key, or `CellRef(K)` cannot be reinterpreted as `CellRef(H)`.

A partial locator is registered with exact endpoints:

```text
locate_L : Point(F) → Option(D_H)
```

The function alone says only that lookup is deterministic and may fail. A lawful declaration also supplies an independently stated relation `Chosen_L : Point(F) → D_H → Prop` such that:

```text
locate_L(p) = some(d)  iff  Chosen_L(p,d)
Chosen_L(p,d₁) and Chosen_L(p,d₂)  imply  d₁ = d₂
```

A locator paired with a placement may additionally promise the center round trip `locate_L(χ(d)) = some(d)`. That law is a spatial bridge, not a property of every arbitrary placement.

The word nearest requires more evidence. A nearest locator declares eligible sites, an ordered cost, and an injective stable tie key into a total order; `Chosen_L(p,d)` then means that `(cost(p,d), tie(d))` is lexicographically minimal among eligible sites. The cost may come from a registered metric. Affine structure alone supplies neither distance nor a nearest operation.

Applying `locate_L` to a position column on query rows `X` produces a partial lineage map:

```text
λ_L(x) = locate_L(pos(x)) : X ⇀ D_H
```

That map lawfully gathers a field on `H` or targets it under the effect's collision law. Invalid positions remain invalid rows or cause a declared whole-effect refusal; they never become a sentinel cell.

Accepted plans retain a registry-authorized lineage witness together with the function it denotes. Its constructors are identity and composition, selection inclusions, product projections, replication source maps, declared relation legs, typed cell-reference reads, and registered spatial bridges such as `locate_L`. There is no constructor from physical layout, equal cardinality, equal shape, or an arbitrary ambient function. Lean can define such functions in the metalanguage; Ano plans cannot present them as lawful lineage without registry authority.

## 16. Interpolation is a weighted relation

A locator chooses at most one cell. An interpolator may sample several. For a weight carrier `K`, a successful interpolation at `p : Point(F)` returns a finite duplicate-free support:

```text
W_I(p) = [(d₀,w₀), …, (dₘ,wₘ)]
dᵢ : D_H
Σᵢ wᵢ = 1
```

The support is nonempty when interpolation succeeds. An ordered weight carrier may additionally require `0 ≤ wᵢ`. A scheme that promises exact sampling at registered cell centers owes `W_I(χ(d)) = [(d,1)]`; that is an optional law, not a consequence of having weights.

For positions `p : X → Point(F)`, successful support entries form an edge habitat `R_I` and a span:

```text
X ←src— R_I —dst→ D_H
weight : R_I → K
```

Sampling `f : D_H → V` is gather along `dst` followed by the weighted fiber reduction along `src`:

```text
sample_I(f,p)(x) = Σ { weight(r) · f(dst(r)) | src(r) = x }
```

The carrier `V` must admit the declared scalar action and additive law. This construction returns one value per valid source row while retaining every contributing cell as lineage. Nearest-cell sampling is the singleton support `[(d,1)]`. No equal-length intermediate is involved.

## 17. Surfaces and support projection

A collision surface need not be a lattice. Register its features as a nominal habitat `S`, place its geometry in frame `F`, and declare the incidence and normal information that a surface operation may observe. With a registered norm or inner product, a hit has the typed form:

```text
SurfaceHit(S,F) =
  feature : S
  point   : Point(F)
  normal  : UnitVector(F)
  t       : NonnegativeScalar
```

A support projector has exact surface and frame endpoints:

```text
supportBelow : Point(F) → Option(SurfaceHit(S,F))
```

Its declaration includes a nonzero down direction, a search interval, admissibility such as collision mask and maximum slope, and an injective stable tie key on candidates into a total order. For seed `p`, every returned hit `h` must satisfy:

```text
h.point = p +ᵥ h.t · down
h.t is inside the declared interval
h.point lies on h.feature
h.normal is valid and the hit is admissible as support
```

`supportBelow(p)` returns the admissible candidate with lexicographically least `(t,tie(h))`, and returns `none` exactly when no admissible candidate exists. These laws distinguish a deterministic semantic operation from a host raycast whose result merely happens to fit the output carrier.

“Nearest support below” and “highest surface in a column” are different orderings and therefore different registered projectors. Here top means the supporting, outward-facing side selected by the admissibility law; it does not silently mean maximum world height. Likewise, an arbitrary affine change of frame does not preserve the chosen hit. Equivariance of a nearest projector may be claimed only under a registered isometry together with transported geometry, direction, interval, and tie policy.

Putting an object's origin exactly at the hit point need not put the object on the surface. A registered prototype support shape supplies:

```text
centerOffset : UnitVector(F) → Vector(F)
center(h) = h.point +ᵥ centerOffset(h.normal)
```

The offset law says which point of the collider contacts the surface. Clearance and overlap among several proposed objects are separate validators; destination uniqueness does not imply collision-free geometry.

## 18. The cheese-wheel derivation

Let `X` be the selected player rows and let `pos : X → Point(World3)`. Spawning exactly 51 wheels per player creates:

```text
C = Σ(x : X). Fin(51)
source : C → X
copy   : C → Fin(51)
```

For spacing `s` and golden angle `α = π(3-√5)`, the Fibonacci or phyllotaxis offset is the local-plane vector:

```text
q(k) = s√k (cos(kα), sin(kα)) : Vector(PlayerPlane2)
```

Each player supplies or derives an injective linear plane embedding `e_x : Vector(PlayerPlane2) → Vector(World3)` from registered orientation and up-direction data. The world-space seed is:

```text
seed(x,k) = pos(x) +ᵥ e_x(q(k)) : Point(World3)
```

The type prevents the local two-vector from being added directly to a world point. A degenerate pair of basis vectors cannot instantiate the promised embedding. The analytic formula for `q` is independent of spatial safety: a dependency-light proof kernel may quantify over a total deterministic `q : Fin(51) → Vector(PlayerPlane2)` and verify the trigonometric implementation separately.

Apply the declared projector before allocation:

```text
hit(c) = supportBelow(seed(c)) : Option(SurfaceHit(CollisionSurface,World3))
```

The exact-51 policy traverses the whole copy habitat. If any `hit(c)` is `none`, validation refuses and the world remains exactly unchanged. If all succeed, define:

```text
wheelPos(c) = center(hit(c)) : Point(World3)
```

and only then allocate fresh entity keys `alloc : C ↪ E_after` and scatter `wheelPos` into the new entities' `Position<World3>` component. The successful result has `51|X|` fresh wheels, every wheel has exactly one source player and copy index, every wheel position satisfies the registered support law, and no fixed lattice or surface field changes habitat, rank, shape, or value merely because entities were allocated.

This derivation does not choose an Ano or registry spelling. The eventual language surface may infer names for the plane embedding, projector, exactness policy, and prototype support shape, but the semantic objects and refusal laws cannot be omitted. Repeating the command begins from another well-formed world under the same schema.

## 19. What Ground means

`Ground` is not a space merely because it is 2D. It is spatial only if the registry gives it spatial structure.

Possible declarations include:

```text
Ground is a finite habitat
Ground has product shape 64×64
Ground has a rank-2 lattice chart
Ground has an affine placement in World3
Ground has a partial locator from Point(World3)
Ground has a declared interpolation scheme from Point(World3)
Ground has a metric
Ground has a Moore neighborhood relation
Ground has a boundary policy
```

Each statement is independent. A placement does not automatically supply its inverse, a locator, an interpolator, or a metric. A `Position<World3>` column can orient its entity rows around `Ground` only through one of the declared bridges. A 2D inventory table has product shape but need not be spatial. A road graph is spatial without being a dense 2D lattice. A voxel field may be a 3D lattice. A one-dimensional path may have placement and metric.

The word “ground” is a name, not a proof.

## 20. Neighborhoods and boundaries

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

## 21. Metrics and topology

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

## 22. Capability structure

The retired tiers are replaced by independent capabilities.

```text
finite(H)
ordered(H)
product(H,A,B)
lattice(H,Λ,κ,lookup)
affine_frame(F,Point,Vector,+ᵥ,-ᵥ)
placement(H,F,χ)
affine_placement(H,F,o,β)
affine_map(F,G,T_P,T_V)
locator(L,F,H,Chosen)
interpolator(I,F,H,K)
surface(S,F,incidence,normal)
support_projector(P,S,F,candidate,order)
metric(H,dist)
complex(H,C,∂)
relation(R,A,B,src,dst)
monoid(V,⊕,e)
commutative(V,⊕)
host_callable(f,signature,footprint)
```

Capabilities compose by conjunction. They do not form one linear hierarchy.

An operation's type states the evidence it needs. Scan needs order. Transpose needs product structure. Stencil needs a lattice chart and boundary. Point-vector arithmetic needs one frame. Placement needs exact habitat and frame endpoints. Radius and nearest need a metric or ordered cost. Interpolation needs weighted support and carrier action. Support projection needs surface incidence, direction, admissibility, order, and tie evidence. Grouped parallel fold needs a commutative monoid. Host dispatch needs a registered signature and footprint.

Registry-authorized lineage is the capability that permits a typed map to participate in a plan. It can expose the function denoted by a locator or relation without admitting every function between equal finite types. Physical layout never supplies this capability.

The type is the license for the operation.

## 23. Why the former Tier 1 proof failed

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

## 24. Why the former Tier 2 proof was not universal

Permutation equivariance is a useful law for operations that claim to ignore labels:

```text
F(c ∘ σ) = F(c) ∘ σ
```

Ano does not universally ignore labels. It exposes generational keys, `index`, explicit singleton bindings, relationships, declared order, placement, and structural allocation. These structures restrict which permutations are symmetries.

The correct use is local. An optimizer may use equivariance when an operation and view certify that the relevant relabelling preserves every observed structure.

Stable grade intentionally observes an order witness. A fixed relationship intentionally observes edges. Neither is illegal. Their extra structure simply invalidates the maximal symmetry premise.

## 25. Why the former Tier 3 proof was too broad

For a genuinely parametric family:

```text
F_V : (I → V) → (J → V)
```

natural in every carrier `V`, Yoneda-style reasoning characterizes `F` as reindexing along a map `J → I`.

That theorem applies only when `F` is parametric in `V`. An opaque host operation is usually not parametric: a pathfinder inspects a navmesh, and a behavior-tree runner inspects nodes. Opacity means the native Ano algebra does not contain that operation. It does not mean the host operation cannot inspect the carrier.

Opaque values remain columns on habitats. Dispatch is a registered capability, not a third tier.

## 26. Time and repeated evaluation

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

## 27. Columnar realization

The formal objects lower directly to column-store data.

| Semantic object | Columnar representation |
|---|---|
| habitat `H` | domain ID, cardinality, layout descriptor |
| `Col(H,V)` | typed dense buffer |
| `Point(F)` or `Vector(F)` item | frame and carrier IDs plus fixed-shape item buffer |
| `CellRef(H)` item | habitat-typed index buffer |
| subobject `X ↪ H` | mask or compact row-index vector |
| authorized partial map `X ⇀ H` | index vector, validity bitmap, and lineage provenance |
| relation span | edge buffer plus source and target index columns |
| query view `X` | aligned buffers plus lineage vectors |
| fiber decomposition | offsets plus grouped edge indices |
| scalar broadcast | scalar kernel argument or repeated view |
| dependent sum | prefix sum of counts plus source-row vector |
| effect | target domain, destination vector, value buffer, merge tag |
| product habitat | nominal habitat ID, rank, shape, strides, axis metadata |
| lattice chart | integer coordinate transform plus bounded lookup |
| placement | source habitat ID, target frame ID, point map or affine origin and basis map |
| locator result | habitat-typed index vector plus validity and locator provenance |
| interpolation | source-to-support edge rows, destination cells, weights, and segment offsets |
| surface hit | surface-feature index, point, normal, ray parameter, and validity |

No row objects are required. No per-value habitat tags are required inside a homogeneous kernel. A plan carries domain, frame, and authorization evidence once and passes raw arrays to the kernel.

This is an array language precisely because the denotation gives the compiler aligned finite families before lowering.

## 28. Proof obligations

A conforming plan must establish the following obligations.

Habitat identity: every consumed column is scalar or aligned on the current query habitat.

Layout validity: every layout is a bijection between physical rows and its semantic habitat, and no layout conversion creates semantic lineage between foreign habitats.

Box validity: the registered product presentation is an equivalence `Box(shape) ≅ D_H`; its axes round-trip, and equal shapes do not identify nominal habitats.

Chart validity: lattice chart and bounded lookup are partial inverses, hence every successful lookup returns one site of the declared habitat and the chart is injective.

Frame validity: point-vector addition and subtraction satisfy the affine torsor laws, and operations cannot combine points or vectors carrying foreign frame indices.

Affine composition: identity and composition preserve the point-vector action; placement transport is functorial, while invertibility and distance preservation are used only when their stronger witnesses are present.

Placement independence: changing or transporting a placement changes neither the field, its habitat, nor its product shape.

Locator validity: every successful result satisfies the independently declared choice relation, the relation is single-valued, failure remains explicit, and a promised placement round trip holds.

Nearest validity: every chosen result is eligible and uniquely minimal under the declared cost and an injective stable tie key; no theorem derives nearest from affine structure alone.

Interpolation validity: every support cell belongs to the declared target habitat, support is finite and duplicate-free, declared normalization and optional center exactness hold, and the induced span reduces to at most one result per valid source row.

Surface-projector validity: every returned hit has the declared surface and frame, satisfies incidence, ray, interval, normal, and admissibility laws, is minimal under the declared projector order, and `none` means that no admissible candidate exists.

Lineage preservation: selection, grade, replicate, relationship hops, reshape, typed cell references, locators, and interpolators produce the correct authorized maps to their source or target habitats.

Target compatibility: every effect destination lands in the target habitat.

Assignment uniqueness: plain assignment has at most one value for each destination unless a resolver is declared.

Merge law: collision reduction uses the registered associative and commutative law required by its execution order.

Field invariance: ordinary field effects preserve habitat, rank, and shape.

Structural separation: entity allocation and reclamation do not resize unrelated habitats.

Exact spatial spawn: validation resolves every requested support hit before allocation; success creates exactly the copy-domain cardinality with fresh keys and supported frame-correct positions, while any miss refuses without a prefix commit.

Boundary totality: every neighborhood access is either valid, masked invalid, or resolved by the declared boundary policy.

Tick closure: every successful post-state satisfies the same schema and can be consumed by the next tick.

Refusal atomicity: failure commits neither values nor structural changes.

## 29. Executable laws

The BQN siblings are executable mathematical witnesses, not semantic oracles. Their useful assertions are laws such as:

```text
shape(after masked field update) = shape(before)
field habitat cardinality is unchanged by spawn
spawn row count = sum(counts on selected source rows)
source projection has one entry per spawned row
exact reshape preserves cardinality
cycling reshape records an output-to-input gather
box coordinates round-trip through the named habitat
lattice chart round-trips through bounded lookup
placement transport composes without changing its field
locating a placed cell center returns that same typed cell when promised
an interpolation support names only cells of its declared habitat
surface hits satisfy the declared ray and incidence equations
nearest-support choice is minimal under its explicit tie policy
an exact 51-copy support miss commits zero copies
an exact 51-copy success allocates 51 fresh supported entities per source
neighborhood boundary matches its named policy
Life step two consumes the output of Life step one
foreign equal-length columns are not treated as aligned
```

A useful negative witness constructs two length-equal columns on foreign habitats and shows that no well-typed binary expression exists without an explicit map. Further negative witnesses reject a `Point(MarsLocal)` at a `World3` locator, a `Vector(PlayerPlane2)` added directly to a `Point(World3)`, a physical layout presented as semantic lineage, a nearest projector without a deterministic choice law, and a prefix allocation after one required support hit fails.

A useful positive witness constructs `X ↪ Ground`, gathers `Water` onto `X`, computes values on `X`, and scatters through the same inclusion. The output remains `Ground → V`.

The worked spatial witness constructs the dependent 51-copy habitat, embeds every phyllotaxis offset from `PlayerPlane2` into `World3`, validates all support hits, and then either allocates exactly all fresh wheels or returns the exact input world. Its field-frame and habitat invariants must survive a second execution.

## 30. The concise model

The entire model can be summarized as follows.

```text
schema names habitats
frames name point and vector carriers
stored columns vary over one habitat
queries construct a current row habitat
authorized lineage maps current rows to stored habitats
array kernels consume columns aligned on current rows
relations are spans
folds reduce fibers
effects scatter through destination maps
structural effects create fresh entity identities
spatial structure is optional capability
placement maps cells to typed points
locators and interpolators map typed points back to cells explicitly
surface projectors return typed, law-bearing hits or explicit failure
exact structural effects validate before allocation
stored field rank never changes by ordinary update
```

Ano is linear algebra in its column kernels, relational algebra in the construction of query habitats and lineage, and affine or topological mathematics only where the registry supplies that additional structure. A position carrier names a frame; only a registered spatial bridge relates its rows to a lattice or surface habitat.
