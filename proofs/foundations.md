# Foundations

Status: active proof obligations. Three semantic-kernel milestones are machine-checked under `Ano/*.lean`: the ten-result field/effect/world kernel; the bounded spatial registry-and-command kernel; and the affine, weighted-interpolation, heterogeneous-bundle, partial-live-`Position`, exact-spawn, and dependent-repeated-tick contract. The checked layer proves additive torsors and scalar-linear registered embeddings, normalized duplicate-free interpolation fibers with sealed source/cell lineage, exact live player anchoring, actual fresh `Position` writes for all 51 supported spawn rows, old-presence and fixed-field preservation, and per-successor request reconstruction. Concrete lattice lookup, phyllotaxis numerics, metric and collider refinement, general multi-component schema integration, boundaries, save/load, backend refinement, and the Steel certificate bridge remain obligations.

## 1. Schema and world

Fix a world schema `Σ`. A habitat is a typed index object named by `Σ`. Habitats may be fixed, such as a lattice window, or nominal and state-varying, such as the live entity population. A world is a value of `World Σ`; spawn and despawn may change the members and cardinality of a nominal habitat while the schema remains `Σ`.

For a fixed habitat `H`, a total field with carrier `V` is a family `f : H → V`. A lattice field has this form. An ECS component is generally partial: it is a presence subobject `ι_C : P_C ↪ E` together with a total family `c : P_C → V`. Equivalently it is `E → Option V`; the subobject form exposes the mask that the column engine actually executes.

The same carrier and the same cardinality do not imply the same habitat. `Health : E → Num` and `Elevation : Ground → Num` remain foreign even when `|E| = |Ground|`. Their buffers may have equal lengths without sharing one row.

## 2. Layout is representation

A finite habitat becomes columnar through a layout `ℓ_H : Fin(n) ≅ H`. Storage holds the ravel `f ∘ ℓ_H : Fin(n) → V`. The layout is an implementation witness, not the semantic identity of `H`.

A partial component may be stored as one `n`-cell value buffer plus an `n`-bit presence mask. Values outside the presence mask are semantically nonexistent even if the buffer contains defaults or junk. A dense lattice field needs no presence mask because its declared domain is total.

Flattening is therefore legal only after the habitat, layout, item carrier, and item shape are known. It may erase none of them from the typed IR. Equal byte counts never license a join, arithmetic operation, or scatter.

## 3. The query view

Every query plan has a current finite row domain `X`. Every computed value is typed `Col X V`, optionally with a validity subobject when a partial read can fail. A scalar is `Col 1 V` and broadcasts only by an explicit diagonal map `X → 1`.

`X` is not necessarily a stored habitat. It may be a selection, an ordered view, a relationship edge set, a product, or a replicated copy set. What makes it useful is lineage: the plan carries the maps from each row of `X` back to the stored habitats from which that row arose.

Examples:

- Selection gives `X = S` with an inclusion `ι : S ↪ E`.
- A functional hop gives a partial target map `r : X ⇀ E`.
- A two-generator comprehension gives `X = A × B` with projections `π_A : X → A` and `π_B : X → B`.
- Replication by `count : S → Nat` gives `X = Σ(s : S). Fin(count(s))` with source projection `π : X → S`.

Two nonscalar operands may combine pointwise only when they are already columns over the same `X`, or when an explicit lineage map reindexes one onto the other. This is the alignment law missing from current Steel.

## 4. Reindexing is gather

For `u : J → I` and `f : I → V`, pullback is `u* f = f ∘ u : J → V`. In the column engine this is a gather by an index vector derived from `u`.

Reindexing explains restriction, permutation, duplication, functional relationship reads, shifts, mirror reads, and the source-column reads of replicate. It obeys `(u ∘ v)* = v* ∘ u*` and `id* = id`; these are compiler rewrite laws once domains are typed.

A functional ECS relationship is not normally a total function. It is a partial map, equivalently a span `E ← R → F` whose left leg is injective. A missing link, dead target, or absent target component removes the corresponding row from the valid subobject. The left-join-null rule is therefore typed partial reindexing, not a numeric sentinel rule.

A general relation is a span `I ←p R →q J`. Pulling a target column to `R` produces an edge-aligned column. Producing one value per source or target additionally requires a fold along one leg. A bare set-valued hop cannot pretend that a ragged fiber is a flat `Col I V`; it remains an edge/fiber view until a quantifier, image, or grouped fold consumes it.

## 5. Aggregation is fiber reduction

Given `u : J → I` and a commutative monoid `(V, ⊕, 0)`, define the unordered pushforward `(u_! f)(i) = ⊕ { f(j) | u(j) = i }`. Its column implementation is segmented reduction or scatter-reduce.

Commutativity is required only when the fiber has no declared order or when parallel merge must be order-independent. An ordered fiber may fold an associative noncommutative monoid in its declared order. A semigroup without an identity is partial on empty fibers. That is A12: `max/` over nothing is no row, while `+/` yields `0`. `avg/` is a monoidal fold of sufficient statistics `(sum,count)` followed by a finisher. It is not itself the monoid operation. `avg\` and `#\` are the prefix form of that machine (Haskell `mapAccumL`, a Mealy step `S × A → S × B`), not `scanl1` over the output carrier.

The homogeneous unseeded fold is `foldl1`. The homogeneous scan is `scanl1`. Their law is `last (scan f xs) = fold f xs` on nonempty input. `foldMap` is the same fold after an explicit map into a monoid: `+/ Gold` already is `foldMap Sum` along the Gold projection. Traversal order is a property of the fiber or the `along` witness, not of the operator. `foldl` versus `foldr` is the wrong factoring once `-` is admitted. A fallible step (`traverse`) is not a surface.

The ordinary fold is reduction along `X → 1`. γ is reduction along a relationship leg. Histogramming and additive scatter are the same fiber reduction with different destination maps. Boolean image uses OR, so duplicate edges collapse by idempotence. A comprehension effect in current Ano likewise takes the Boolean image of the surviving pair relation unless the surface explicitly requests multiplicity.

## 6. Shape-changing reads

Selection changes the current view from `I` to a subobject `S`. Grade produces an ordering or permutation of a view; it does not change stored identity. Outer product produces `I × J`. Replicate produces the dependent sum `Σ(i : I). Fin(count(i))`. These derived domains are first-class plan objects with lineage.

`reshape` has three distinct meanings. Exact reshape between equal-cardinality boxes is reindexing along a layout equivalence. APL reshape with repetition or truncation is pullback along the output-to-input cycling map and is not an equivalence. Ano's current `pos = to h w` does neither to the entity habitat: it constructs a coordinate column over the selected entity view and scatters that column back to `pos`. The three operations must not share a proof merely because their surface flavor is `⍴`.

A rank-changing result may be queried, folded further, passed to a host function, or materialized into a separately declared compatible habitat. It may not silently replace a stored field on another domain.

## 7. Effects and the barrier

An effect over query rows `X` carries a destination map `d : X ⇀ H` into the target habitat, a value column over the valid part of `X`, and a merge family. All reads come from pre-state. Commit groups effect rows by destination and reduces each fiber according to that family.

Plain assignment requires `d` to be injective on the written rows, or a separately declared deterministic conflict rule. Additive, minimum, maximum, Boolean OR, and other certified families may merge collisions by their commutative monoids. A masked write to the current source uses the selection inclusion as `d`; gather-compute-scatter is `ι*` followed by update along `ι`.

The target column retains its declared habitat before and after commit. Neither mask compression nor a rank-changing intermediate mutates that declaration. This one law applies to entity columns and lattice fields alike.

A statement plan denotes a pure function `World Σ → EffectBuffer × Output`; commit applies the buffer to obtain another `World Σ`. A performed statement is consequently a partial state transformer `World Σ → Result (World Σ × Output)`, since invalid registered behavior, arithmetic refusal, or allocation failure may reject the tick without committing.

Spawn and despawn do not contradict schema closure. They change the current live entity set inside `World Σ`, not `Σ`. If `C = Σ(s : S). Fin(count(s))` is the copy view, allocation supplies fresh nominal keys `a : C ↪ E'`; source fields reach copies through `π*`, and entity-valued parent links are legal only when `S` itself maps to the entity habitat. A lattice cell index is not an entity parent merely because both are integers.

## 8. Lattice habitats

For rank `r`, define `Shape r = Fin r → Nat` and `Box n = ∏(j : Fin r). Fin(n(j))`. A named lattice habitat `H` has a nominal site type `D_H` and one registered product presentation `b_H : Box(n_H) ≅ D_H`. Equal rank and shape therefore supply possible coordinate bijections between habitats, not habitat equality or authorized query lineage.

The product presentation is not the physical layout. `b_H` supplies semantic axes and bounded coordinates; `ℓ_H : Fin(N) ≅ D_H` supplies buffer order. Neither is a declaration that another habitat's coordinates or rows align with `H`.

Let `Λ_H` be a free `ℤ`-module of rank `r`. A lattice capability supplies a chart and bounded lookup:

```text
κ_H      : D_H → Λ_H
lookup_H : Λ_H → Option(D_H)
```

They obey `lookup_H(κ_H(d)) = some(d)` and `lookup_H(z) = some(d) → κ_H(d) = z`. Hence `κ_H` is injective, and every attempted lattice displacement either returns one site of `H` or fails explicitly. The integer coordinates of `κ_H(b_H(q))` equal the bounded coordinates `q` embedded in `ℤ`.

A field is `f : D_H → V`. The rank belongs to the registered product presentation of `D_H`; the item dimension belongs to `V`. A scalar field over a 2-D ground has field rank 2 and item rank 0. A velocity field over the same ground still has field rank 2 even if each value is a 3-vector.

## 9. Affine frames and placement

A frame `F` is nominal and supplies distinct carriers `Point F` and `Vector F`. `Vector F` has an additive commutative group law, and `Point F` is its torsor. The kernel must expose point-vector addition and point subtraction only at one frame index and prove `p +ᵥ 0 = p`, `(p +ᵥ u) +ᵥ v = p +ᵥ (u+v)`, `(p +ᵥ v) -ᵥ p = v`, and `p +ᵥ (q -ᵥ p) = q`. A coordinate presentation additionally fixes dimension, axes, scalar carrier, units, and coordinate maps; matching dimensions do not identify frames.

The exact equations are witness obligations, not facts about every physical numeric carrier. IEEE floating-point coordinates may use deterministic evaluation without licensing reassociation or cancellation; an exact or error-bounded lowering theorem is a separate obligation.

An affine map `T : F → G` has `pointMap : Point F → Point G` and an additive `vectorMap : Vector F → Vector G` satisfying `pointMap(p +ᵥ v) = pointMap(p) +ᵥ vectorMap(v)`. Identity and composition must satisfy this law. An affine equivalence adds inverse round trips. Distance or nearest-point preservation requires a stronger registered isometry witness and is not an affine theorem.

A general placement is `Placement D_H F = D_H → Point F`. An affine placement additionally has `o : Point F` and an additive `β : Λ_H → Vector F` with `χ(d) = o +ᵥ β(κ_H(d))`. Injectivity of `β` is required when the declaration promises an embedding; curved or otherwise general placements remain arbitrary typed maps.

Transport along `T` is `T.pointMap ∘ χ`. Identity transport is identity and `U` after `T` equals transport by `U ∘ T`. Consequently a planet-local placement followed by a system transform equals the composed placement. Changing or transporting placement leaves `f : D_H → V`, `D_H`, rank, and shape unchanged.

## 10. Localization and interpolation

A position component remains partial on entities: `P_pos ↪ E` with `pos : P_pos → Point F`. A typed cell reference has carrier `CellRef H = D_H`. Neither the component's name nor its item shape aligns `P_pos` with `D_H`, and a physical offset cannot be coerced into `CellRef H`.

A partial locator has exact endpoints `locate_L : Point F → Option D_H`. Its declaration includes an independent relation `Chosen_L : Point F → D_H → Prop` and proves:

```text
locate_L(p) = some(d)  iff  Chosen_L(p,d)
Chosen_L(p,d₁) ∧ Chosen_L(p,d₂)  implies  d₁ = d₂
```

A bridge paired with placement may additionally prove `locate_L(χ(d)) = some(d)`. A nearest locator owes eligibility, ordered cost, an injective stable tie key into a total order, and minimality under the resulting lexicographic order. Affine placement alone cannot discharge that obligation.

For a position column `p : X → Point F`, pointwise localization gives a partial row-to-cell map `X ⇀ D_H`. An accepted plan stores a registry-authorized lineage witness whose constructors are identity and composition, selection inclusions, product projections, replication source, declared relation legs, typed cell-reference reads, and declared spatial bridges. There is no constructor from physical layout, equal cardinality, equal shape, or an arbitrary function supplied after planning.

An interpolator over weights `K` maps a point to an optional finite duplicate-free nonempty list `[(dᵢ,wᵢ)]` with each `dᵢ : D_H` and declared normalization `Σᵢwᵢ = 1`; nonnegativity and exact center sampling are additional optional laws. Applied to `p : X → Point F`, its support rows form a span `X ← R_I → D_H` with `weight : R_I → K`. Sampling a field gathers along the cell leg and performs the declared weighted fiber reduction along the source leg. The result has at most one value per valid source row, and nearest-cell sampling is the singleton-weight-one case.

## 11. Surface support and exact spatial spawn

A collision surface has a nominal feature habitat `S` in frame `F`, with declared incidence and normal data. Given a registered norm or inner product, a hit records:

```text
SurfaceHit S F =
  feature : S
  point   : Point F
  normal  : UnitVector F
  t       : NonnegativeScalar
```

A support projector `supportBelow : Point F → Option (SurfaceHit S F)` declares a nonzero direction, search interval, admissibility predicate, ordered candidate cost, and an injective stable tie key on candidates into a total order. A returned hit must lie on its feature, satisfy `hit.point = seed +ᵥ hit.t · down`, lie inside the interval, have a valid admissible normal, and minimize the declared `(cost,tie)` key. It returns `none` exactly when no admissible candidate exists. Nearest support below and highest surface in a column are different projectors. Equivariance under change of frame is claimed only for registered isometries with all geometry and policy transported.

A prototype support shape supplies `centerOffset : UnitVector F → Vector F`; a supported object origin is `hit.point +ᵥ centerOffset(hit.normal)`. Collider clearance among several proposals remains a separate validator.

For selected player rows `X`, exact count 51 gives `C = Σ(x:X).Fin(51)`. Let `q : Fin 51 → Vector PlayerPlane2` be the total phyllotaxis pattern, and let each source supply an injective linear embedding `e_x : Vector PlayerPlane2 → Vector World3`. Then:

```text
seed(x,k) = pos(x) +ᵥ e_x(q(k))
hit(c)    = supportBelow(seed(c))
```

The exact policy validates every `hit(c)` before allocation. Any `none` produces the exact input world. If all succeed, `wheelPos(c) = hit(c).point +ᵥ centerOffset(hit(c).normal)`, fresh allocation gives `C ↪ E_after`, and scatter fills only the new `Position<World3>` rows. The target theorem proves `|C| = 51|X|`, unique source and copy lineage, fresh injective allocation, support validity of every position, preservation of all fixed habitats and fields, and world well-formedness for the next tick. The trigonometric identity of the chosen `q` is separate from these spatial safety theorems.

## 12. Neighborhood and boundary

A stencil is a relation on the field habitat. A displacement set in `Λ` induces a partial relation on a finite window because some translated coordinates leave `D`.

Boundary forms complete that partial relation in different ways. Shrink drops missing edges. Zero or another constant extends the value field outside `D`. Clamp and reflect provide explicit retractions from attempted coordinates to `D`. Wrap equips the box with modular coordinates, equivalently a finite quotient-lattice or toroidal structure. These are semantic structures, not consequences of flat indexing, and the selected form must survive save, reload, and every tick.

## 13. Capabilities

Every stored target preserves its declared habitat. What varies is which structure and carrier laws an operation requires.

- Finite habitat: pointwise map, mask, fold with the required carrier law.
- Ordered habitat or ordered view: stable grade and scan.
- Product habitat: axes and outer product.
- Lattice chart: shifts and displacement stencils.
- Affine frame: point-vector arithmetic within one nominal frame.
- Placement: cells mapped to typed points without changing their field.
- Affine map or equivalence: lawful frame transport with only the promised invertibility.
- Locator or interpolator: explicit point-to-cell lineage and sampling.
- Metric: distance, radius, and the cost evidence for a nearest operation.
- Surface projector: typed hits under declared direction, admissibility, order, and tie policy.
- Cell complex: incidence, boundary, and cochains.
- Opaque carrier: only registry-granted operations; opacity is a capability policy.

Permutation equivariance remains a useful theorem for operators that claim to ignore nominal identity, order, bindings, and relationship structure. It is not the law of all entity writes: `index`, declared orders, unique keys, singleton bindings, relationships, and `pos = to …` intentionally observe additional structure. Naturality in the carrier remains the characterization of genuinely parametric maps, not the definition of every opaque value.

## 14. Columnar lowering

The semantic objects lower directly:

| object | column-store form |
|---|---|
| `Col H V` | typed buffer plus habitat/layout token |
| partial component | value buffer plus presence bitmap |
| `Point F` or `Vector F` item | frame and carrier tokens plus fixed-shape item buffer |
| `CellRef H` item | habitat-typed index buffer |
| selection `S ↪ H` | bitmap or selection vector plus parent layout |
| authorized reindex `u : J → I` | gather index vector plus lineage provenance |
| product | paired lineage vectors |
| relation span | edge arrays or CSR with source/target habitat tokens |
| replicate | counts, prefix sum, copy-to-source vector |
| pushforward | segmented reduce or scatter-reduce |
| effect | destination indices, values, validity mask, merge family |
| lattice field | buffer plus immutable habitat id, rank, shape, chart, lookup, boundary |
| placement | source habitat and target frame tokens plus point map or affine origin and basis |
| locator | typed destination indices, validity, and registry provenance |
| interpolation | support edge rows, typed destination cells, weights, and segment offsets |
| surface hit | feature indices, points, normals, parameters, and validity |

Dense execution remains ordinary SoA work. Habitat tokens and lineage maps are compile-time or plan-time metadata; the hot loop still sees contiguous buffers, masks, gathers, segmented reductions, and scatters. Typed index sets do not oppose array performance: they determine which low-level operation is legal before the metadata erases.

## 15. Proof accounting

The three checked milestones discharge these statements:

1. `Box(shape)` coordinates round-trip through one registered nominal habitat presentation, and equal rank, shape, cardinality, or storage never erase nominal habitat identity.
2. `Point` and `SpatialVector` carry nominal frame indices. `AffineFrame` proves the torsor laws; additive and scalar-linear maps prove identity, composition, extensionality, and affine action laws; an exact `OffsetMapToken` refines an injective anchored linear embedding without implying an isometry or equivalence.
3. Position views, boxes, semantic lineage, frame maps, offset embeddings, situated habitats, and interpolators require exact registry endpoints. `Lineage` has a private constructor and admits structural, registered, successfully located, and registered interpolation-source/cell legs, never a physical layout or raw function.
4. A `Locator` is sound, complete, and functional against its independent acceptance relation. Successful localization gathers exactly the located field cell, while injective registered destinations support deterministic field scatter.
5. A `WeightedSupport` is finite, nonempty, cell-duplicate-free, and normalized. Its induced `WeightedSpan` retains source and cell maps, enumerates each fiber exactly, agrees with direct weighted sampling, reproduces constants, and reduces independently of support-edge order under the declared commutative value law; nonnegativity is an independent optional certificate.
6. `SupportSpec.Best` and a stable-key tie law make the declared best hit unique. Frozen prepared batches retain one exact input snapshot, pattern, projector, resting capability, resolved hit per seed, and support proof per returned center.
7. The exact copy habitat is `Fin 51`; every copy retains its selected player source. A private-constructor live anchor additionally ties the exact pre-world, live selected key, stored `some Position`, resolver token, and registered scalar-linear embedding to every generated seed.
8. `SpatialWorld` stores one partial live `Position` family as `Fin entityCount → Option (Point frame)`. An accepted exact cheese request either refuses with the identical world or adds exactly 51 entities, preserves every old `Position` value including `none`, writes every fresh row to the exact supported payload point, preserves every fixed field, and preserves well-formedness.
9. `LiveCheeseResolver` reconstructs its dependent request from each successor world. Arbitrarily many successful ticks therefore preserve well-formedness without reusing an obsolete entity carrier or first-tick anchor.
10. A heterogeneous `Bundle` admits `n` carrier types on one shared query domain only through sealed fixed/located-field lineage, exact registered weighted interpolation rows with their value law, or certified present live-position rows. `EntityPositionEffect.apply_hit_registered_inputs` composes that n-ary row into the exact point written through an injective live destination; the effect also proves miss, fixed-field, population, and well-formedness laws. Registered lattice assignment currently proves field-level hit and miss only.
11. Guarded negative witnesses reject foreign equal-shaped habitats and frames, local vectors in world slots, rows as cell references, layouts and raw interpolation legs as lineage, unregistered affine embeddings, foreign-frame interpolation, and raw functions as registered bundle inputs.

The non-spatial compiler bridge accounts for these statements:

1. OPEN — A canonical certificate checker is the sole constructor of `Rewrite<Law>`; checking binds law/version, exact typed signature and endpoints, schema fingerprint, normalized proposition, checker version, and proof payload, and any mismatch invalidates cached authority.
2. [X] MET — Runtime promotion preserves the operator-specific denotation, destination checking preserves the declared storage carrier, and the extended-real boundary admits finite values and `±∞`, identifies both IEEE signed zeros with the sole Ano zero, and excludes every NaN before world publication.
3. [X] MET — `cross f A B` lowers to the product domain A×B without raveling, every pure wrapper retains that lineage, and no entity-column scatter accepts it without an explicit compatible destination map.
4. [X] MET — Every scan result carries the ordered world-row domain of its value column; inverse grading preserves value/witness alignment, and scatter is admitted only when that domain equals the selected destination rows exactly, not merely in cardinality.
5. [X] MET — Every guard residual crossing a discarded speculative buffer is closed over durable world bindings; a residual naming any probe-local temporary refuses before emission.
6. OPEN — Sealed history partitions are write-disjoint from the live barrier; stable-keyed `ago(k)` is a partial functional reindexing, `window(k)` is an ordered history fiber, and reconstruction within a declared retention horizon is observationally equal to a materialized partition.

The stronger spatial theory and implementation bridge still owe these statements:

1. Bounded integer-lattice chart and lookup round trips, chart injectivity, displacement lookup, and boundary policies.
2. Concrete 2-D/3-D coordinate, scalar, unit, and module instances plus metric, orthogonality, isometry, or dimension laws wherever the language promises distance, nearest, or rigid orientation.
3. The analytic sine, cosine, square-root, and golden-angle phyllotaxis generator, its offset uniqueness property, and exact or error-bounded refinement to the chosen deterministic numeric backend.
4. Concrete registered interpolators and, where convex interpolation is promised, separately certified nonnegative weights; reverse interpolation or write-back additionally needs injectivity or an explicit commutative collision merge.
5. Concrete support geometry: ray parameters and intervals, surface incidence, unit normals, slope/collision admissibility, collider resting poses, clearance, metrics, and proof or validation that engine queries construct the abstract support certificates.
6. An executable unique-player and all-51 preparation validator, explicit host snapshot/service versions, allocation-capacity refusal, prototype/component defaults, and atomic transaction integration.
7. Promotion of the companion `EntitySpatialRegistry` and partial `Position` family into the general schema's multi-component storage, archetype/relationship effects, global well-formedness metadata, and world-level lattice effects.
8. A compiler-facing accepted-plan IR whose constructors are sealed or certificate-checked, including a restricted operation language for `PositionMapping`; public mathematical helpers such as raw scatter and arbitrary ordinary updates are not themselves accepted Ano plans.
9. Save/load observational identity, schema/token/version preservation, relationship/despawn repair, boundary totality, and surface-complex laws where declared.
10. Dense-array refinement for gather, CSR fiber reduction, prefix-sum replication, scatter/scatter-reduce, physical relayout invariance, and proof that Steel and Kore construct only the admitted certificates.

## 16. Current Steel findings

Steel now enforces the non-spatial promotion, numeric-publication, product-lineage, exact scan-domain, and closed-guard boundaries above. It still has no general nominal habitat type in `Ev`: `Frame` is transient statement-emitter state inferred from surface syntax, registry fields flatten to `lat_w * lat_h`, and entity columns and lattice fields still enter through the same value branch. Save preserves the registry's one global `lattice w h` header but no expression carries that spatial identity through the compiler.

The one-step corpus therefore contains false witnesses. Repeating computed-line once grows the entity world from 12 to 44 and the second assignment faults at `12 ≠ 44`. Repeating spatial top-k once grows 64 entities to 72 and the second lattice mask faults against the 72-row column. Conway repeats because its explicit `moore` fibers and 25-cell field buffers reconstruct the needed adjacency; it does not witness rank-preserving field semantics in the compiler.

Until Steel carries habitat and lineage types, the spatial BQN files are design sketches and counterexample material, not conformance oracles.
