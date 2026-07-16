# Foundations

Status: active proof obligations, 2026-07-16. The first ten-result semantic kernel is machine-checked under `Ano/*.lean`; lattice shape, placement, boundaries, save/load, columnar lowering, partial ECS components and relationships, and the Steel certificate bridge remain obligations. The former Tier 1/2/3 development is retired because its write-back laws do not describe Ano or its column store.

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

Commutativity is required only when the fiber has no declared order or when parallel merge must be order-independent. An ordered fiber may fold an associative noncommutative monoid in its declared order. A semigroup without an identity is partial on empty fibers. `avg/` is a monoidal fold of sufficient statistics `(sum,count)` followed by a finisher; it is not itself the monoid operation.

The ordinary fold is reduction along `X → 1`. γ is reduction along a relationship leg. Histogramming and additive scatter are the same fiber reduction with different destination maps. Boolean image uses OR, so duplicate edges collapse by idempotence. A comprehension effect in current Ano likewise takes the Boolean image of the surviving pair relation unless the surface explicitly requests multiplicity.

## 6. Shape-changing reads

Selection changes the current view from `I` to a subobject `S`. Grade produces an ordering or permutation of a view; it does not change stored identity. Outer product produces `I × J`. Replicate produces the dependent sum `Σ(i : I). Fin(count(i))`. These derived domains are first-class plan objects with lineage.

`reshape` has three distinct meanings that the old spec conflated. Exact reshape between equal-cardinality boxes is reindexing along a layout equivalence. APL reshape with repetition or truncation is pullback along the output-to-input cycling map and is not an equivalence. Ano's current `pos = to h w` does neither to the entity habitat: it constructs a coordinate column over the selected entity view and scatters that column back to `pos`. The three operations must not share a proof merely because their surface flavor is `⍴`.

A rank-changing result may be queried, folded further, passed to a host function, or materialized into a separately declared compatible habitat. It may not silently replace a stored field on another domain.

## 7. Effects and the barrier

An effect over query rows `X` carries a destination map `d : X ⇀ H` into the target habitat, a value column over the valid part of `X`, and a merge family. All reads come from pre-state. Commit groups effect rows by destination and reduces each fiber according to that family.

Plain assignment requires `d` to be injective on the written rows, or a separately declared deterministic conflict rule. Additive, minimum, maximum, Boolean OR, and other certified families may merge collisions by their commutative monoids. A masked write to the current source uses the selection inclusion as `d`; gather-compute-scatter is `ι*` followed by update along `ι`.

The target column retains its declared habitat before and after commit. Neither mask compression nor a rank-changing intermediate mutates that declaration. This one law applies to entity columns and lattice fields alike.

A statement plan denotes a pure function `World Σ → EffectBuffer × Output`; commit applies the buffer to obtain another `World Σ`. A performed statement is consequently a partial state transformer `World Σ → Result (World Σ × Output)`, since invalid registered behavior, arithmetic refusal, or allocation failure may reject the tick without committing.

Spawn and despawn do not contradict schema closure. They change the current live entity set inside `World Σ`, not `Σ`. If `C = Σ(s : S). Fin(count(s))` is the copy view, allocation supplies fresh nominal keys `a : C ↪ E'`; source fields reach copies through `π*`, and entity-valued parent links are legal only when `S` itself maps to the entity habitat. A lattice cell index is not an entity parent merely because both are integers.

## 8. Lattice habitats

An abstract lattice is a free `ℤ`-module `Λ` of rank `r`. A dense finite field needs more: a box `D = ∏_{j<r} Fin(n_j)` and a lattice chart `κ : D → Λ`. For the canonical lattice `Λ = ℤ^r`, `κ` is the ordinary integer coordinate inclusion. The shape `(n_0,…,n_{r-1})` and rank `r` are permanent habitat data.

A field is `f : D → V`. The rank belongs to `D`; the item dimension belongs to `V`. A scalar field over a 2-D ground has field rank 2 and item rank 0. A velocity field over the same ground still has field rank 2 even if each value is a 3-vector.

Placement is optional extra structure. Let `A` be an affine ambient space over translation module `T`, choose `o ∈ A`, and choose a linear map `β : Λ → T`. Then `χ(d) = o + β(κ(d))` places the lattice window in `A`. Requiring `β` injective prevents collapsed axes. A written origin and basis matrix are a coordinate presentation of `(o,β)`, not part of an unplaced field.

Thus `origin 100 0 200` means a point only after a 3-D affine parent and units are declared. It says nothing intrinsic about a root 2-D ground. `ground` is a habitat because it is a declared index object, not merely because it has two axes.

## 9. Neighborhood and boundary

A stencil is a relation on the field habitat. A displacement set in `Λ` induces a partial relation on a finite window because some translated coordinates leave `D`.

Boundary forms complete that partial relation in different ways. Shrink drops missing edges. Zero or another constant extends the value field outside `D`. Clamp and reflect provide explicit retractions from attempted coordinates to `D`. Wrap equips the box with modular coordinates, equivalently a finite quotient-lattice or toroidal structure. These are semantic structures, not consequences of flat indexing, and the selected form must survive save, reload, and every tick.

## 10. Capabilities, not tiers

The old tiers tried to classify data by how freely a write could change its index. That axis is wrong: every stored target preserves its declared habitat. What varies is which structure and carrier laws an operation requires.

- Finite habitat: pointwise map, mask, fold with the required carrier law.
- Ordered habitat or ordered view: stable grade and scan.
- Product habitat: axes and outer product.
- Lattice chart: shifts and displacement stencils.
- Affine placement: world coordinates and change of frame.
- Metric: distance and radius.
- Cell complex: incidence, boundary, and cochains.
- Opaque carrier: only registry-granted operations; opacity is a capability policy, not a habitat tier.

Permutation equivariance remains a useful theorem for operators that claim to ignore nominal identity, order, bindings, and relationship structure. It is not the law of all entity writes: `index`, declared orders, unique keys, singleton bindings, relationships, and `pos = to …` intentionally observe additional structure. Naturality in the carrier remains the characterization of genuinely parametric maps, not the definition of every opaque value.

## 11. Columnar lowering

The semantic objects lower directly:

| object | column-store form |
|---|---|
| `Col H V` | typed buffer plus habitat/layout token |
| partial component | value buffer plus presence bitmap |
| selection `S ↪ H` | bitmap or selection vector plus parent layout |
| reindex `u : J → I` | gather index vector |
| product | paired lineage vectors |
| relation span | edge arrays or CSR with source/target habitat tokens |
| replicate | counts, prefix sum, copy-to-source vector |
| pushforward | segmented reduce or scatter-reduce |
| effect | destination indices, values, validity mask, merge family |
| lattice field | buffer plus immutable habitat id, rank, shape, chart, boundary |

Dense execution remains ordinary SoA work. Habitat tokens and lineage maps are compile-time or plan-time metadata; the hot loop still sees contiguous buffers, masks, gathers, segmented reductions, and scatters. Typed index sets do not oppose array performance: they determine which low-level operation is legal before the metadata erases.

## 12. Proof obligations

The next formal development owes these statements before stronger optimization claims:

1. Reindex identity and composition.
2. Every pointwise expression is aligned over one query domain.
3. Selection gather followed by scatter through the same inclusion changes exactly the selected target cells.
4. Every collision at commit has an injective destination or a certified deterministic reduction.
5. Stored habitat identity, rank, and shape survive every value-only barrier and save/reload cycle.
6. Spawn and despawn preserve the world schema while changing only declared nominal populations and dependent columns.
7. Backend layout erasure preserves the typed denotation.
8. A repeated tick has the same typing derivation as the first tick; no program is accepted only because initial buffer lengths happen to coincide.

## 13. Current Steel findings

Steel does not yet enforce these obligations. `Frame` is transient statement-emitter state inferred from surface syntax; registry fields are flattened to `lat_w * lat_h`; `Ev` carries value-shape flags but no habitat; `emit_name_val` accepts entity columns and lattice fields through the same branch; assignment checks BQN lengths only when the generated program happens to fault. Save preserves the registry's one global `lattice w h` header but no expression carries that identity through the compiler.

The one-step corpus therefore contains false witnesses. Repeating computed-line once grows the entity world from 12 to 44 and the second assignment faults at `12 ≠ 44`. Repeating spatial top-k once grows 64 entities to 72 and the second lattice mask faults against the 72-row column. Conway repeats because its explicit `moore` fibers and 25-cell field buffers reconstruct the needed adjacency; it does not witness rank-preserving field semantics in the compiler.

Until Steel carries habitat and lineage types, the spatial BQN files are design sketches and counterexample material, not conformance oracles.
