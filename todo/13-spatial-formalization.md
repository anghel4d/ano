# 13 — Spatial formalization implementation handoff

Status: theory-to-implementation handoff. No Steel or Kore implementation is authorized by this file. `todo/12-spatial-formalization.md` states the observable outcome; this file incorporates the audited affine-torsor, scalar-linear embedding, weighted-interpolation, heterogeneous registered-input, partial live-`Position`, exact-spawn, and dependent repeated-tick Lean milestone, then states the remaining typed IR, registry, execution, numeric, geometry, trust, and acceptance work without weakening that boundary.

## Authority

`docs/ano-language.md` is the language contract. `docs/spatialmaths.md` gives the mathematical account. `docs/ano-ecs.md` gives the column-store contract. `proofs/foundations.md` records the proof boundary and remaining obligations. `proofs/Ano/*.lean` is authoritative only for declarations imported by the `Ano` umbrella target and accepted by a clean `lake build` plus axiom audit. Steel is the reference compiler and standalone launcher. Kore is the current interactive world.

The Pious Hierarchy applies throughout: Mathematics > Denotation > domain-and-lineage IR > Grammar > Surface > lowering and backend details. The implementation may choose representations and algorithms below the IR, but it may not infer semantic alignment from a representation fact.

## Checked semantic boundary

The original kernel already checks layout encode/decode round trips, relayout invariance, reindex identity and contravariant composition, selection gather/modify/scatter hit and miss laws, injective assignment determinism, duplicate-free checked assignment schedules, commutative-monoid merge permutation independence, dependent-copy enumeration and source lineage, fresh allocation disjointness, fixed-field preservation under spawn, atomic validation refusal, successful-step well-formedness, and arbitrary finite successful-tick preservation.

The spatial layer adds the following checked objects and laws.

| Checked object | Implementation license | Deliberate limit |
|---|---|---|
| `Box shape` and `Boxed D` | A nominal domain may expose a registered rectangular product presentation; `Boxed.coordinate_site` and `Boxed.site_coordinate` round-trip. | This is not a physical layout and does not yet prove the free-integer chart or bounded lookup laws described in `proofs/foundations.md`. |
| `FrameSchema`, `Point`, and `SpatialVector` | Nominal frame indices keep equal raw carriers foreign. | Concrete coordinates, dimensions, units, and numeric refinement remain separately declared instances. |
| `AffineFrame`, `AdditiveVectorMap`, `AffineMap`, and `AffineEquiv` | Points and vectors obey the affine torsor laws; additive affine maps compose; equivalences have inverse round trips. | Additivity alone supplies neither scalar linearity, metric preservation, orthogonality, nor physical units. |
| `SemiringLaw`, `ModuleLaw`, `LinearVectorMap`, and `LinearAffineMap` | A declared scalar action obeys the module laws and registered linear maps preserve it. | Exact laws do not automatically hold for IEEE arithmetic; dimensionality, isometry, and error bounds are not inferred. |
| `RegisteredLinearAffineEmbedding` | The exact `OffsetMapToken` agrees with one injective anchored scalar-linear embedding, so local offsets become target-frame points only through that registered action. | The token does not prove surjectivity, rigid motion, distance preservation, or the concrete phyllotaxis formula. |
| `Locator` and `Situated` | Successful single-cell lookup is equivalent to one independent functional acceptance relation; a promised placement round trip makes that placement injective. | General placement remains lawful without localization, and geometric nearest construction is not derived from an affine map. |
| `WeightedSupport`, `WeightedSpan`, and `InterpolatorRows` | Each successful interpolation fiber is finite, nonempty, duplicate-free, normalized, typed to one habitat, and reducible through explicit source/cell legs; direct and segmented sampling agree, constants reproduce, and commutative reduction ignores edge order. | Normalization is not nonnegativity: convexity needs its own certificate. Reverse writes need injectivity or an explicit merge, and a concrete interpolator service remains to be supplied. |
| `SpatialRegistry` and `SpatialInterpolationRegistry` tokens | Position, box, direct lineage, frame-map, anchored offset-map, situated, and interpolator capabilities have exact nominal endpoints. | General placement, support, resting-pose, clearance, persistence, and service-version integration remain companion obligations rather than one complete runtime registry. |
| Private-constructor `Lineage` | Structural, registered, located, and registered interpolation source/cell factories authorize gather; raw functions and layouts cannot. | Relationship legs and the companion live-component lineage still need integration with Steel's general compiler/world model. |
| `LocatedRows`, `UniqueRegisteredRows`, and registered interpolation rows | Single-cell and weighted field reads retain their destination habitat; plain field write-back additionally requires injectivity. | Cross-source destination collisions still require merge or refusal; registered lattice assignment is proved at field level, not yet as a schema-valid world action. |
| `RegisteredInput`, heterogeneous `Bundle`, and `PositionMapping` | Any finite family of distinct carriers may feed one typed mapping only after every fixed or located field, registered weighted interpolation, or certified present live position is aligned onto the same query domain. `apply_hit_registered_inputs` proves the evaluated n-ary point is exactly what the entity effect writes. | `PositionMapping.evaluate` is mathematical metalanguage until Steel restricts which registered operations may construct it. |
| `EntityPositionEffect` | Injective live destinations perform an actual partial `Position` update and prove hit, miss, fixed-field, population, and well-formedness laws. | This is one companion component family, not yet the general multi-component/archetype transaction or core schema integration. |
| `SupportSpec`, `SupportProjector`, and `ResolvedSupportBatch` | Declared score and stable-tie laws make `Best` functional; success returns exactly that hit and a resolved batch retains one best hit per seed. | Ray, incidence, interval, unit-normal, metric, completeness, and host candidate generation remain supplied or runtime-validated geometry obligations. |
| `SnapshotSupportProjector`, `RestingPose`, and `FrozenPreparedBatch` | Preparation freezes the exact input world, projector, pattern, resting capability, hits, and supported returned positions into one payload. | Host snapshot versions, collider epochs, finite-coordinate validation, non-penetration, clearance, and service persistence remain outside this abstract certificate. |
| `SpatialSpawn.CheeseCopies`, private-constructor `PlayerPattern`, and private-constructor `LivePlayerAnchor` | The copy domain is exactly `Fin 51`; the selected live key, stored `some Position`, exact pre-world, resolver token, input offsets, and registered linear embedding determine every seed. | The actual golden-angle `sin`, `cos`, `sqrt` generator and its numeric uniqueness proof remain pending. |
| `SpatialWorld`, private-constructor `LiveCheeseRequest`, and `LiveCheeseResolver` | Acceptance adds exactly 51 entities, writes all 51 fresh supported positions, preserves every old `Position` including absence and every fixed field, and preserves well-formedness. Refusal is identical state, and each later tick reconstructs its dependent request from the successor world. | The base schema still delegates general global invariants; prototype/default components, allocator capacity, relationships, save/load, and atomic storage across arbitrary component families remain unmodeled. |
| Spatial negative witnesses | Foreign habitats/frames, local vectors in world slots, rows/layouts/raw interpolation legs used as lineage, unregistered affine embeddings, foreign-frame interpolation, and raw bundle maps fail at elaboration. | These checks guard the Lean API; Steel must reproduce the same refusals before lowering. |

No implementation document may collapse a deliberate limit in the last column into a checked theorem. The current proof does establish abstract affine/module laws, normalized interpolation, aligned heterogeneous inputs, and one actual partial live `Position` spawn/update path. It does not establish concrete floating geometry, the Fibonacci formula, a general accepted-plan language, arbitrary ECS component transactions, relationship repair, save/load identity, boundaries, host raycast correctness, dense-backend equivalence, or Steel certificate construction.

## Governing implementation invariant

Every planned value has two independent types: a row domain on which it varies and a carrier describing one value. A position component is therefore not “a spatial array”; it is a partial entity-domain column whose carrier is `Point<Frame>`. A lattice field is a total fixed-habitat column whose carrier may be scalar, point, vector, cell reference, or opaque. The only way those rows interact is through an authorized query-domain map, locator support relation, interpolator span, typed relationship, or another exact registry bridge.

An ordinary effect on stored field `H → V` returns another `H → V`. Structural effects may change the live entity population, but they do not resize `H`, change its rank or shape, alter its box presentation, or synthesize cross-habitat lineage. Every refusal returns the input world and leaves the undo/log transaction without a committed prefix.

## Required nominal identifiers

Steel should intern the following semantic identities before expression planning. Integer encodings are implementation details; equality is by interned declaration identity and schema version, not by descriptor structure.

```rust
struct SchemaId(u64);
struct SchemaVersion(u64);
struct HabitatId(u32);
struct QueryDomainId(u32);
struct FrameId(u32);
struct CarrierId(u32);
struct ComponentId(u32);
struct FieldId(u32);
struct RelationId(u32);
struct SurfaceId(u32);
struct PrototypeId(u32);
struct LayoutId(u32);
struct ServiceId(u32);
struct PolicyId(u32);
struct SnapshotVersion(u64);
```

Two declarations with equal contents still receive different nominal IDs unless the registry explicitly aliases them as the same declaration. Deserialization may restore an identity only after matching the saved schema identity and declaration fingerprint; it may not rebuild identity from rank, shape, item width, name spelling, or byte length.

## Required schema descriptors

The following shapes are illustrative Rust IR, not a ruling on source or registry syntax. Fields may be normalized into arenas rather than nested literally, but no semantic member may be discarded before planning and validation finish.

```rust
enum HabitatKind {
    LiveEntities,
    ComponentPresence { entity_habitat: HabitatId, component: ComponentId },
    Fixed,
    RelationEdges { relation: RelationId, source: HabitatId, target: HabitatId },
    AnonymousDerived,
}

struct HabitatDesc {
    id: HabitatId,
    schema: SchemaId,
    kind: HabitatKind,
    cardinality: Cardinality,
    box_capability: Option<BoxToken>,
    default_layout: LayoutId,
}

struct BoxDesc {
    habitat: HabitatId,
    rank: usize,
    shape: Vec<usize>,
    coordinate_map: ServiceId,
    site_map: ServiceId,
}

struct LayoutDesc {
    id: LayoutId,
    habitat: HabitatId,
    physical_rows: usize,
    encode: PhysicalPermutation,
    decode: PhysicalPermutation,
}

struct FrameDesc {
    id: FrameId,
    dimension: usize,
    scalar: CarrierId,
    point_carrier: CarrierId,
    vector_carrier: CarrierId,
    units: Vec<UnitId>,
    axes: Vec<AxisId>,
}

enum CarrierKind {
    Scalar,
    Point { frame: FrameId },
    Vector { frame: FrameId },
    CellRef { habitat: HabitatId },
    EntityRef { entity_habitat: HabitatId },
    SurfaceRef { surface: SurfaceId },
    Opaque,
}

struct CarrierDesc {
    id: CarrierId,
    kind: CarrierKind,
    physical_item_shape: Vec<usize>,
    refinements: Vec<RefinementId>,
}

enum Totality {
    Total,
    Partial { presence_habitat: HabitatId },
}

struct ColumnDesc {
    habitat: HabitatId,
    carrier: CarrierId,
    totality: Totality,
    mutability: Mutability,
    layout: LayoutId,
    values: BufferId,
    validity: Option<BitmapId>,
}
```

`BoxDesc` is a semantic product presentation of one nominal habitat. `LayoutDesc` is a physical bijection between row slots and that same habitat. A `BoxToken` may expose coordinate/site round trips. A `LayoutId` may permit encode, decode, and relayout only. There is no conversion from `LayoutId`, equal cardinality, or equal `shape` into semantic lineage.

A field's row rank and shape come from its habitat's `BoxDesc`. A point or vector dimension comes from its carrier's `FrameDesc`. A 64×64 field of `Point<World3>` therefore has row shape `[64,64]` and item shape `[3]`; neither shape substitutes for the other.

## Sealed authorization tokens

Authorization handles must have private constructors in the compiler/registry crate. Parsing, type inference, lowering, host callbacks, saved data, and plugin code may carry or reference a token but may not mint one. The registry validator mints tokens only after endpoint lookup and obligation validation succeed.

```rust
pub struct PositionToken(PrivateToken);
pub struct BoxToken(PrivateToken);
pub struct LineageToken(PrivateToken);
pub struct FrameMapToken(PrivateToken);
pub struct OffsetMapToken(PrivateToken);
pub struct PlacementToken(PrivateToken);
pub struct LocatorToken(PrivateToken);
pub struct InterpolatorToken(PrivateToken);
pub struct SituatedToken(PrivateToken);
pub struct SupportProjectorToken(PrivateToken);
pub struct RestingPoseToken(PrivateToken);
pub struct ClearanceToken(PrivateToken);

struct PositionAuth { component: ComponentId, frame: FrameId }
struct BoxAuth { habitat: HabitatId, rank: usize, shape: Vec<usize> }
struct LineageAuth { source: HabitatId, target: HabitatId, map: ServiceId }
struct FrameMapAuth { source: FrameId, target: FrameId, point_map: ServiceId }
struct OffsetMapAuth { offset_frame: FrameId, anchor_frame: FrameId, scalar: CarrierId, linear_map: ServiceId, place: ServiceId, additive: WitnessId, scalar_compatible: WitnessId, agrees: WitnessId, injective: WitnessId }
struct PlacementAuth { habitat: HabitatId, frame: FrameId, place: ServiceId }
struct LocatorAuth { frame: FrameId, habitat: HabitatId, service: ServiceId, choice: PolicyId }
struct InterpolatorAuth { frame: FrameId, habitat: HabitatId, weight: CarrierId, service: ServiceId, laws: WitnessId }
struct SituatedAuth { habitat: HabitatId, frame: FrameId, placement: PlacementToken, locator: LocatorToken, round_trip: WitnessId }
struct SupportAuth { snapshot_kind: SnapshotKindId, surface: SurfaceId, frame: FrameId, score: CarrierId, service: ServiceId, policy: PolicyId }
struct RestingAuth { prototype: PrototypeId, surface: SurfaceId, frame: FrameId, service: ServiceId, law: WitnessId }
struct ClearanceAuth { prototype: PrototypeId, frame: FrameId, service: ServiceId, policy: PolicyId }
```

`OffsetMapToken` is load-bearing. Its operation is anchored and typed:

```text
place : Point<AnchorFrame> × Vector<OffsetFrame> → Point<AnchorFrame>
```

It is not a raw cast from `Vector<PlayerPlane2>` to `Vector<World3>`, and it is not a row-domain map. The cheese plan must carry the exact token that relates `PlayerPlane2` offsets to points anchored at the selected player's `Point<World3>`.

### Affine and linear runtime contract

The registry certificate must preserve the same layering as Lean. A frame names an additive commutative vector law and a point torsor action. A scalar-capable frame additionally names one semiring and module action. An affine map proves compatibility with point translation; a linear affine map additionally proves preservation of scalar multiplication. Injectivity, equivalence, isometry, dimension, and numeric refinement are separate flags backed by separate witnesses, never consequences of a three-number representation.

```rust
struct AffineFrameLawDesc { frame: FrameId, additive_group: WitnessId, torsor: WitnessId }
struct ModuleLawDesc { frame: FrameId, scalar: CarrierId, semiring: WitnessId, module: WitnessId }
struct LinearMapDesc { source: FrameId, target: FrameId, scalar: CarrierId, evaluate: ServiceId, additive: WitnessId, scalar_compatible: WitnessId }
struct LinearEmbeddingAuth { token: OffsetMapToken, local_frame: FrameId, world_frame: FrameId, scalar: CarrierId, linear: LinearMapDesc, anchored_place: ServiceId, agrees: WitnessId, injective: WitnessId }
```

The load-bearing equalities are `place(anchor,0)=anchor`, `place(anchor,u+v)=anchor+L(u+v)`, `L(u+v)=L(u)+L(v)`, `L(a·u)=a·L(u)`, and agreement of the token callback with `anchor+L(u)`. Steel need not solve these equations. It must accept only a registry declaration whose witness rung is configured to authorize them, retain the exact token and endpoints in the plan, and refuse a rewrite requiring a stronger unregistered property.

For exact carriers the runtime operation may implement these equations directly. For floating point, the registry instead names a deterministic evaluation order and a refinement policy: exact representable subset, tolerance/error budget, interval enclosure, or trusted-host contract. The planner must not apply associativity, cancellation, normalization, distance, or injectivity rewrites to IEEE values merely because the corresponding real-number map has them.

`PlacementToken` permits the covariant read `CellRef<H> → Point<F>`. `LocatorToken` separately permits a partial return `Point<F> ⇀ CellRef<H>`. `SituatedToken` is the stronger paired capability with a promised site-center round trip. General placements must remain usable without pretending to be situated or invertible.

Persisted files store stable declaration IDs, versions, and fingerprints, not the in-memory private token representation. Load revalidates the registry and remints tokens. A missing, changed, or endpoint-incompatible declaration refuses load or plan restoration; it never degrades to an untyped callback.

## Query-domain and lineage planning

Every resolved expression has `TypedColumn { domain: QueryDomainId, carrier: CarrierId, validity }`. A non-scalar pointwise kernel accepts operands only on one current query domain. Scalar broadcast is an explicit map from that domain to the scalar domain. Matching row counts are not consulted for type compatibility.

```rust
struct TypedColumn {
    domain: QueryDomainId,
    carrier: CarrierId,
    validity: Validity,
    value: ValueNodeId,
}

struct QueryDomain {
    id: QueryDomainId,
    cardinality: Cardinality,
    origin: DomainOrigin,
    order: Option<OrderId>,
    lineage: Vec<LineageEdge>,
}

enum LineageTarget {
    Query(QueryDomainId),
    Habitat(HabitatId),
}

struct LineageEdge {
    source_rows: QueryDomainId,
    target: LineageTarget,
    map: IndexMapId,
    validity: Option<BitmapId>,
    provenance: LineageProvenance,
}

enum LineageProvenance {
    Identity,
    SelectionInclusion { parent: QueryDomainId },
    ProductLeft { left: QueryDomainId },
    ProductRight { right: QueryDomainId },
    ReplicateSource { source: QueryDomainId, count: ValueNodeId },
    RelationLeg { relation: RelationId, endpoint: Endpoint },
    CellReferenceRead { component: ComponentId, habitat: HabitatId },
    Registered { token: LineageToken },
    ComponentPresent { token: PositionToken, component: ComponentId },
    Located { token: LocatorToken, source_position: ValueNodeId },
    InterpolationSource { token: InterpolatorToken, source_position: ValueNodeId },
    InterpolationCell { token: InterpolatorToken, habitat: HabitatId },
}
```

The constructor set is closed. There is deliberately no `FromLayout`, `FromEqualLength`, `FromEqualShape`, `FromCarrierWidth`, or `UncheckedFunction` provenance. A backend may fuse or erase a lineage vector only after the planner has consumed its typed provenance and recorded the equivalent destination/gather operation.

Selection adds an inclusion. Products add both projections. Replication adds one copy-to-source projection. A functional relation adds a partial endpoint map. A locator adds a partial point-to-cell destination with its token and validity. An interpolator adds a support-edge domain with source and cell legs. Grade retains the same rows plus an order/permutation witness. Exact reshape uses a declared equivalence; cycling reshape carries an output-to-input gather; anonymous generation creates a fresh query domain ID.

Selection, product, replication, identity, and interpolation-source edges target another query domain; field gathers, located cells, and interpolation-cell edges target a habitat. In particular an interpolation edge domain `R` carries `R → Query(J)` and `R → Habitat(H)` simultaneously. The sum target prevents an entity/query row ID from being reinterpreted as a lattice habitat merely because both lower to integers.

Frame maps, anchored offset maps, and placement act on carriers while preserving the current query domain. Reindexing and lineage act on rows while preserving the value carrier. Keeping those directions separate prevents a 2D-to-3D coordinate transform from becoming accidental field write-back authority.

Effects consume a destination edge into the exact target habitat. Assignment requires injectivity on valid rows. A colliding destination requires an admitted merge and its order law. A successful locator may therefore sample a field immediately, but plain write-back through located rows is rejected unless destinations are injective; otherwise the program names a merge or refuses.

## Registered n-input mappings

A registered mapping is a typed operation over one query domain, not an untyped callback over equally long buffers. Surface specials or registry keywords resolve first to stable declaration IDs. Each of the `n` inputs then records its own carrier and an admitted route onto the one row domain `J`; heterogeneity is in the carriers, alignment is in the shared `QueryDomainId`.

```rust
struct RegisteredInputDesc {
    declaration: RegistryValueId,
    carrier: CarrierId,
    source: InputSource,
    to_rows: LineageEdge,
    presence: Option<BitmapId>,
}

enum InputSource {
    FixedField { field: FieldId, habitat: HabitatId },
    LiveComponent { component: ComponentId, entity_habitat: HabitatId },
    LocatedField { field: FieldId, locator: LocatorToken },
    InterpolatedField { field: FieldId, interpolator: InterpolatorToken, value_law: WitnessId },
    Derived { node: ValueNodeId },
}

struct TypedBundle {
    rows: QueryDomainId,
    inputs: Vec<RegisteredInputDesc>,
}

struct RegisteredMappingPlan {
    rows: QueryDomainId,
    inputs: TypedBundle,
    result: TypedColumn,
    body: CheckedMappingExpr,
}
```

`InputSource::Derived` remains an unproved generic extension. The checked Lean `RegisteredInput` admits sealed fixed fields, total registered locator gathers, registered weighted interpolation through exact `RegisteredInterpolationRows` plus `WeightedValueLaw`, and certified present live `Position`; it does not admit an arbitrary derived column. Steel may admit a generic derived node only when it is already a checked `TypedColumn` on the bundle domain or reaches that domain through sealed row lineage, with a corresponding accepted-plan certificate.

`CheckedMappingExpr` must be a closed, typed operation language: projections by input slot, registered scalar and carrier operations, conditionals with one row domain, registered affine/frame transforms, locator/interpolator nodes, and constructors whose result carrier is declared. It must not contain an arbitrary Rust closure or raw function pointer. Every node checks operand carrier, frame/habitat endpoints, validity, and row domain before lowering. This is the Steel realization still owed by Lean's deliberately generic `PositionMapping.evaluate`.

`RegisteredMappingPlan.result.domain` must equal `rows`, and its carrier and validity are checked outputs of `body`. An entity `Position` assignment requires total validity on all destination rows. Under an exact policy any invalid locator/interpolator/mapping row refuses atomically; dropping invalid rows is legal only by constructing an explicit domain-changing selection with its inclusion lineage.

The canonical entity-over-lattice path is: choose entity rows `J`; prove live `Position<F>` presence on `J`; locate or interpolate those points into habitat `H`; gather each requested lattice field through the registered cell leg onto `J` or its support edges; reduce support fibers back to `J`; evaluate the `n`-input mapping to `Col<J,Point<F>>` or another declared carrier; then scatter through injective entity lineage into `Position<F>`. Lean checks direct fixed-field lineage, total registered localization, registered weighted sampling as a sealed bundle input, and the end-to-end entity write equation. No lattice row becomes an entity key, and no field buffer becomes aligned with entity rows by matching length.

For a lattice-target result, the final plan instead carries a registered destination `J → CellRef<H>`. Plain assignment proves that map injective on valid rows. Noninjective reverse interpolation or many-entity deposition names a commutative merge family and lowers to scatter-reduce; without one it refuses. The current Lean milestone proves entity `Position` effects and field-level lattice assignment separately, so Steel must not present the latter as a general world mutation until schema/global preservation certificates are added.

## `Position<Frame>` components

The registry declares position semantics on the carrier, not by the column name. A component descriptor for `Position<World3>` is a partial entity column with carrier `Point { frame: World3 }`, a presence bitmap or presence habitat, and an inclusion into the live entity habitat.

Reading position first restricts or validates component presence, then returns `Col<X,Point<World3>>` on the current query domain. A missing position invalidates that row under the declared partial-read rule. Writing position targets entity keys through entity lineage or a fresh allocation map. It never targets a lattice cell, raw buffer slot, or surface feature.

Adding a `Vector<World3>` to a `Point<World3>` requires a registered carrier operation. A `Vector<PlayerPlane2>` first passes through the exact anchored `OffsetMapToken`. A `Point<MarsLocal>` first passes through an exact `FrameMapToken` if one exists. Item shape `[3]`, the spelling `pos`, or equal units cannot authorize either conversion.

The companion Lean `SpatialWorld` models this one partial component family and proves real hit/miss updates plus exact extension of all 51 fresh rows while preserving every old `some` or `none`. Steel and Kore must generalize that result into the core schema's arbitrary component-presence storage, shared allocation, prototype/default fill, archetype movement, relationship repair, atomic commit, and persistence; they may not claim those broader properties from the companion theorem.

## Locator, interpolator, and support services

A locator service accepts a frozen batch of `Point<F>` and returns one `CellRef<H>` plus validity per source row. The plan node carries `LocatorToken(F,H)`. The registry contract supplies an independent choice relation, soundness, completeness for accepted sites, single-valuedness, and explicit failure. Bounds checking alone is not a choice-law certificate.

An interpolator service accepts `Point<F>` and returns a finite CSR-style support relation into `CellRef<H>` with one weight per edge. The checked Lean contract requires every successful source fiber to be nonempty, duplicate-free by cell, typed to one habitat, and normalized according to the declared weight law; nonnegative weights and exact center sampling are additional independent laws. Sampling gathers field values along the registered cell leg and performs the declared weighted reduction along the registered source leg; direct support sampling equals segmented fiber reduction, constants reproduce, and commutative reduction is edge-order independent. Steel must validate CSR offsets, cell bounds, per-fiber duplicate freedom, normalization under the admitted numeric contract, exact token endpoints, and one result per valid source row before erasing the metadata.

A support service is snapshot-dependent. Freezing it yields one immutable projector for `SnapshotVersion v`; every one of the 51 seeds is evaluated against that same version. The service result is a SoA batch containing validity, stable surface-feature references, hit points, normals, ray parameter or equivalent candidate data, semantic score, and stable semantic tie key. Lean's checked snapshot is currently the exact `World` value passed to `prepare`; `SnapshotVersion` is the host realization still requiring explicit concurrency, staleness, and service-version validation.

`Best(seed,hit)` means the hit is admissible and no admissible candidate has a smaller semantic score; equal scores choose the least stable semantic key. The score comparison must be total for admitted runtime values. Non-finite floats, incomparable scores, or overflow in a score representation refuse before choice. The semantic tie key must be persistent and injective among distinct equal-score candidates; BVH traversal index, buffer row, pointer address, thread arrival, or hash-table iteration order is forbidden. If distinct candidates share both score and tie key, the service refuses rather than choosing by enumeration.

The strongest backend shape returns all admissible candidates and lets one canonical planner kernel compute lexicographic minimum. A trusted host service may return only the winner when registration supplies the `Best` contract and conformance tests permute physical candidate order. Either route remains inside the host/registry trust boundary until a geometry-specific proof validates incidence, ray, interval, normal, and candidate enumeration.

A resting-pose service is prototype-, surface-, and frame-specific. It converts a contact hit into an entity origin and validates the declared support relation, collider offset, finite coordinates, and any required non-penetration predicate. Placing the entity origin directly at the raw hit is not a valid default for a wheel whose origin is its center.

Clearance is a separate optional batch service. Fresh entity destination keys prevent database write collisions but do not prevent two wheel colliders from overlapping or occupying one point. When a command requests clearance, validation considers all 51 proposals together and the frozen pre-state before allocation.

## Exact 51-wheel plan

The exact policy has one order. Reordering these phases changes semantics and is not an optimization.

1. Freeze the statement's world and collision-service snapshot version; every read, projector candidate, resting pose, and optional clearance query in this statement refers to that version.
2. Resolve `Player` to exactly one live entity carrying `Position<World3>`; zero players, multiple players, a dead binding, or missing position refuses before a copy domain exists.
3. Construct `C = Fin(51)` and the canonical copy index column `0 … 50`; this is 51 total copies for the unique player, not 51 per row of an unresolved multirow selection.
4. Evaluate the registered phyllotaxis callable once over `C`, producing `Col<C,Vector<PlayerPlane2>>`; validate the declared totality, finite numeric values, and any required distinct-offset witness. Lean assumes this typed injective offset field and does not prove `sin`, `cos`, `sqrt`, the golden angle, or their floating-point lowering.
5. Gather the unique player's `Point<World3>` through the copy-to-source lineage and apply the exact `OffsetMapToken(PlayerPlane2,World3)` to each `(anchor,offset)`, producing `Col<C,Point<World3>>` seeds without changing `C`.
6. Freeze or retrieve the `SupportProjectorToken` for the same snapshot and evaluate all 51 seeds; validate typed feature/frame results, candidate admissibility, total score, stable tie, and `Best`; one miss or unresolved choice rejects the whole prepared batch.
7. Apply the prototype's `RestingPoseToken` to every hit and validate all 51 final `Point<World3>` origins; no allocation has occurred yet.
8. If requested, run the `ClearanceToken` over the full 51-proposal batch plus the frozen pre-state; one overlap or policy failure rejects the whole batch.
9. Validate prototype existence, component schema, defaults, field mutability, destination plans, allocator capacity, and the ability to reserve exactly 51 fresh keys; then allocate one injective map `a : Fin(51) ↪ E_after` without exposing a committed prefix.
10. Build every spawned component column over `C`, including `Position<World3>`, prototype/tag/default fields, and any source-derived values, and scatter all of them through the same allocation map `a` in one structural transaction.
11. Commit once. Success exposes exactly 51 new live identities and their complete component rows, leaves every old component row unchanged unless separately targeted, and leaves every fixed spatial field, habitat, box, rank, shape, layout identity, and registry declaration unchanged. Any failure through commit aborts the reservation and returns the exact input world, output, undo head, and structural log state prescribed for refusal.

All support selection observes the pre-spawn snapshot. The first wheel never becomes a collision surface for the second wheel in the same statement. A later statement or tick may observe all committed wheels together.

## Refusal matrix

| Phase | Refusal | Required state result |
|---|---|---|
| Registry load | Duplicate IDs, invalid endpoints, missing frame/habitat/carrier, mismatched token endpoints, changed service fingerprint, or unvalidated law declaration. | Registry/world load fails; no downgraded untyped declaration is installed. |
| Plan typing | Foreign habitats, foreign frames, raw local vector used as world displacement, layout used as lineage, placement used backward, or missing authorization token. | Compile/plan refusal before CBQN or host execution. |
| Player resolution | Zero or multiple live players, dead binding, missing `Position<World3>`, or wrong position frame. | Exact input world; no copy rows committed and no allocator reservation. |
| Pattern | Callable failure, wrong cardinality, wrong carrier, invalid/non-finite offset, or required offset-injectivity failure. | Exact input world; no support call or allocation. |
| Snapshot | Collision snapshot unavailable, stale, mixed, or version-changed during preparation. | Exact input world; discard preparation and do not retry under a mixed snapshot inside the statement. |
| Support | No admissible candidate for any seed, invalid feature/frame, invalid incidence/normal/ray data, incomparable score, duplicate semantic tie key, or no unique `Best`. | Exact input world; zero wheels, not a successful shorter batch. |
| Resting pose | Missing prototype support law, invalid collider data, non-finite origin, failed support, or required non-penetration failure. | Exact input world; zero wheels. |
| Clearance | Any proposal pair or pre-state collider violates the declared clearance policy. | Exact input world; zero wheels. |
| Allocation | Capacity failure, key collision, generation mismatch, or inability to reserve exactly 51 fresh identities. | Release any uncommitted reservation; exact input world and unchanged free-list observation. |
| Component preparation | Missing prototype/default, wrong carrier, missing required component, invalid value, or inconsistent allocation map across component columns. | Abort the whole transaction; no live partial entities. |
| Commit | Duplicate plain-assignment destination, unproved merge, target mutation race, stale snapshot/version, or storage failure. | Roll back every value and structural write; world, undo history, and statement log remain at the refusal boundary. |

Best-effort “skip misses” behavior is a different explicit policy and must not reuse the exact command's type or diagnostics. It may be added later only with a surface ruling and separate tests.

## Persistence and versioning

Save must retain schema ID/version/fingerprint; nominal habitat and frame declarations; every column's habitat, carrier, totality, item shape, and layout; every box rank/shape and semantic presentation identity; point/vector frame IDs; cell-reference habitat IDs; relationship endpoint habitats; placement, locator, interpolator, projector, resting-pose, clearance, boundary, score, and semantic tie policy identities and versions; live entity keys/generations; component presence; field values; and enough registry data to revalidate every restored bridge.

Private authorization tokens are process-local evidence and are never trusted from bytes. Load resolves saved declaration identities against the active registry, checks fingerprints and endpoints, remints sealed tokens, and then decodes buffers through their saved layouts. A mismatch refuses load atomically. Reinterpreting a saved point as another equal-dimensional frame or a field as another equal-shaped habitat is forbidden.

Compiled-plan caches are keyed by schema fingerprint and every referenced service/policy version. Any changed habitat, frame, carrier, layout contract, bridge, score, tie key, prototype support rule, or boundary invalidates the plan. Snapshot-dependent prepared batches are never persisted as reusable authorization across world versions.

The target law is observational save/load identity, followed by a successful second tick under the same schema. This law remains pending Lean proof and must be tested before it is claimed for Steel or Kore.

## Columnar lowering

The typed plan remains columnar. A query domain lowers to a row count plus masks/index vectors. Selection lowers to a bitmap or compact inclusion. Reindex lowers to gather. Product lowers to paired index arithmetic. Replication lowers to counts, prefix sums, and a repeated source vector. A heterogeneous bundle lowers to one aligned buffer descriptor per input on the same row count. Located lineage lowers to typed destination indices plus validity and token provenance. Interpolation lowers to validated CSR offsets, typed cells, weights, gathers, and segmented reduction. Support projection lowers to batched seeds and SoA hit/score/tie/validity columns. Exact allocation lowers to one fresh-key vector. Component effects lower to a destination vector shared across aligned presence and value buffers; admitted collisions lower only to the named scatter-reduce.

`Point<F>`, `Vector<F>`, and `CellRef<H>` need no per-element tags in a homogeneous kernel. Their carrier/frame/habitat evidence lives once in the plan and buffer descriptor. Erasure is legal only after the planner has fixed the kernel's endpoints and after persistence/debug metadata has retained the semantic IDs needed outside the hot loop.

Physical relayout may change row order only through encode/decode permutations. It must not change a locator result, support `Best`, semantic tie, effect destination, or output world. Candidate or copy order may be optimized only when the declared operation is invariant under that permutation; stable semantic keys and copy indices remain observable where specified.

CBQN may remain the initial execution backend. Steel must reject an invalid plan before generating CBQN, and tests compare backend output with the typed Ano semantics. A coincidentally successful BQN length operation is not evidence of legality.

## Steel migration sequence

1. Record a failing baseline for the affected Ano/Nihongo demos, equal-length and raw-bridge negative cases, two-tick sentinels, and save/load. Preserve those failures as regression tests rather than patching generated BQN first.
2. Introduce nominal IDs and descriptors for schemas, habitats, query domains, frames, scalar/module laws, carriers, layouts, components, fields, relations, services, and policies. Convert the entity world and lattice into explicit declarations at ingest without changing surface syntax.
3. Make carrier typing structural in the resolved IR: `Point<Frame>`, `Vector<Frame>`, `CellRef<Habitat>`, entity references, and ordinary scalars remain distinct even when all lower to numeric buffers. Add partial component-presence metadata and generalize the checked companion `Position` model into core multi-component storage rather than treating it as a fixed total field.
4. Implement registry validation and private token arenas. Only this pass mints position, box, lineage, affine/frame-map, scalar-linear offset-map, placement, locator, interpolator, situated, support, resting-pose, and clearance tokens; every token stores exact endpoints, witness rung, service version, and numeric contract.
5. Give every resolved expression one `QueryDomainId`, carrier, and validity. Replace `Mode::{World,Sel,Copy}` and implicit length alignment with explicit domain constructors and closed-provenance lineage edges, including component-presence and both interpolation legs. Keep carrier maps separate from row maps.
6. Resolve every registry special or keyword to a declaration ID, then construct a heterogeneous `TypedBundle` whose inputs all reach one domain through admitted lineage. Typecheck its `CheckedMappingExpr`; never admit a raw closure, callback, index vector, or buffer-length coincidence as a `PositionMapping`.
7. Type effects before lowering. Every target names one stored column and destination edge; assignment proves injectivity, merge names a certified commutative family, structural effects carry one exact prepared batch and allocation plan, and lattice world effects additionally prove schema/global preservation.
8. Add batched locator, CSR interpolation, frozen support, resting-pose, and optional clearance nodes. Validate exact endpoint tokens, CSR offsets/cells/fibers/normalization, snapshot version, numeric policy, and reduction law before erasure. Use the registered scalar-linear `OffsetMapToken` for player-relative offsets.
9. Implement the exact-51 preparation state machine in the strict order above. Allocation is the first state-changing phase and remains a reservation until every component effect buffer passes validation; commit component presence, values, liveness, generations, relationships, undo, and log state in one barrier.
10. Extend save/load and plan-cache validation to every nominal descriptor, affine/module/numeric witness, and service version. Remint tokens after registry validation. Add observational save/load and second-tick tests before optimizing layouts.
11. Lower typed nodes to masks, typed index vectors, gathers, prefix sums, CSR segmented reductions, batched host calls, scatters, and scatter-reduces. Prove or differentially check each typed node against the semantic operation; BQN is one backend result, not the oracle.
12. Only after correctness, fuse maps, erase per-plan metadata inside kernels, batch raycasts, reuse scratch buffers, and vectorize phyllotaxis. Every optimization retains a typed diagnostic plan and passes relayout, edge-order, candidate-order, numeric-contract, and repeated-tick invariance tests.

## Kore migration sequence

1. Consume Steel's schema, domain, carrier, frame, token, and refusal descriptors; do not reconstruct a ground, frame, or position type from buffer lengths, item widths, or names.
2. Display habitat ID/name, row rank/shape, layout, carrier, frame, partial presence, and spatial capability endpoints in field/component inspection. Distinct equal-shaped habitats remain visibly distinct.
3. Implement or bind host locator, support, resting-pose, and optional clearance services behind the exact registry descriptors. Every batched call receives one immutable snapshot version and returns typed SoA data plus precise refusal diagnostics.
4. Apply exact structural transactions to live entities, component presence, archetype storage, fresh generations, undo history, and the session log atomically. A refused exact spawn adds neither a partial entity nor an undo/log state pretending that a mutation occurred.
5. Preserve and inspect all spatial metadata through `r`, `n`, `u`, direct edits, save, and reload. Direct edits validate frame/carrier refinements and cannot change a field's habitat, rank, or shape.
6. Run every spatial acceptance case twice from the first post-state, then through save/reload, then once more. The second tick is a normal required path, not a stress mode.

## Acceptance tests

Positive tests must assert semantic metadata and values, not merely row counts or absence of a backend exception.

- Register `Ground` and `MarsSurface` with equal rank, shape, cardinality, carrier, and even equal physical permutations; update each independently and show that no joint operation exists until a specific registered lineage token is supplied.
- Register a partial entity `Position<World3>` and `locateGround : Point<World3> ⇀ CellRef<Ground>`; sample a ground field through located rows, retain entity and cell lineage, and prove or test that a successful sample equals the selected ground cell's value.
- Exercise a general placement without a locator and show that forward placement is accepted while reverse lookup is unavailable; separately exercise a situated placement/locator round trip.
- Exercise an interpolator with multi-cell support and assert typed habitat endpoints, nonempty duplicate-free fibers, normalization, and exactly one reduced value per valid source row.
- Register an affine `PlayerPlane2 → World3` embedding with explicit scalar/module, additivity, scalar-compatibility, agreement, and injectivity witnesses; verify composition and anchored offset evaluation, then show that no metric or isometry operation is available without a separate certificate.
- Resolve at least three heterogeneous registry inputs—for example live `Position<World3>`, `Ground.Height`, and `Ground.Material`—onto one entity query domain through presence plus locator/interpolator lineage, evaluate a checked mapping, and write entity positions without ever aligning the entity and lattice buffers by length.
- Run the exact cheese command with one player and 51 uneven support heights; assert exactly 51 fresh live keys, one common source player, copy indices `0…50`, frame-correct anchored seeds, one semantic `Best` hit and supported resting origin per copy, complete `Position<World3>` component presence, unchanged old entities, and extensionally unchanged fixed fields.
- Shuffle collision candidate storage, BVH traversal, worker scheduling, and physical surface layout while retaining semantic candidates; assert identical chosen feature keys, hit columns, positions, fresh-key sequence under its declared allocation order, and output world.
- Run the successful exact cheese command on the next tick and assert another complete well-formed transaction; no fixed field or first-tick component loses its habitat, frame, rank, shape, or presence.
- Save after one success, reload, compare schema/registry/spatial metadata and world observations, then run another success; loaded plans either revalidate the exact tokens or recompile, never reuse stale handles.
- Run the named lattice, computed-line, reductions, density, derived-field, board-literal, many-habitat, Life, and tie demos from `todo/12-spatial-formalization.md` for two ticks with their Nihongo siblings semantically identical.

Negative compile/plan tests must refuse before backend execution.

- Combine equal-length foreign habitats pointwise; cross-write equal-shaped fields; use `Fin(n)` or a `LayoutId` as `CellRef<H>` or lineage; write a rank-changing derived value into a fixed field without a destination; or derive a map from equal cardinality.
- Pass `Point<MarsLocal>` to a `World3` locator; add `Vector<PlayerPlane2>` directly to `Point<World3>`; store a raw three-number vector into `Position<World3>`; or use a point map without a `FrameMapToken`.
- Use a placement as a locator, use a general locator where a situated round trip was required, or use a spatial service whose exact habitat/frame endpoints do not match the planned operands.
- Register a nearest/support policy without a total admitted score, stable semantic tie key, tie uniqueness law, explicit failure meaning, snapshot dependency, or exact surface/frame endpoint.
- Plain-scatter two located rows to one cell without an injectivity proof or merge; destination uniqueness is checked on valid rows, not guessed from source cardinality.
- Pass a raw function, physical index vector, differently-domained column, unregistered affine callback, raw interpolation leg, negative normalized weights to a convex-only scheme, or colliding reverse support to a registered mapping/effect; require a typed refusal before backend execution.

Runtime refusal tests must assert exact state identity, allocator/free-list observation, component presence, undo head, and log boundary.

- Resolve zero players, two players, a dead player, or a player without `Position<World3>`.
- Fail one of 51 pattern rows, support queries, `Best` validations, resting poses, or requested clearance checks; assert zero committed wheels rather than 50 or a smaller successful batch.
- Return two distinct equal-score candidates with one duplicate semantic tie key, a NaN score, an out-of-frame hit, an invalid feature, a stale snapshot version, or candidate-order-dependent output.
- Fail exact allocation capacity, freshness/generation validation, one prototype/default component, one component destination, or commit; assert no partially live entity and no component prefix.
- Repeat the computed-line and spatial-top-k sentinels on the first post-state; any second-tick refusal leaves that post-state unchanged and reports the typed domain mismatch before CBQN.

## Trust boundary

Lean proves the semantic laws for supplied structures and certificates. It does not prove that Steel's parser, registry loader, planner, serializer, CBQN emission, Kore host services, physics engine, floating-point kernels, or allocator constructs those certificates honestly. The implementation task is to make those systems produce only the inputs admitted by the checked boundary.

The universal claim is intentionally narrower than “every public Lean helper is lawful”: generic `Plan`, `OrdinaryUpdate`, `PositionMapping`, `WeightedSpan`, and `Scatter.assign` are mathematical metalanguage and can express arbitrary replacement or unresolved collisions. Only private constructors and registered wrappers constitute the accepted-plan boundary; Steel must make that distinction structural.

Registry ground entries are trusted host facts: storage identity, callback behavior, geometry candidates, prototype support data, and service versions. Sky entries are laws used for rewriting or semantic claims and require the configured witness rung. Property tests and runtime validation reduce risk but do not turn an arbitrary raycast into a Lean theorem.

Private token constructors make accidental or plugin-level fabrication impossible inside safe Rust APIs. Unsafe code, FFI, corrupt persistence, or a malicious host remains outside the theorem unless a separately verified boundary validates it. Tokens name evidence; they are not evidence if deserialized or constructed without registry validation.

The support `Best` theorem proves uniqueness from the supplied candidate, order, and tie laws. It does not prove that a host enumerated every collision surface, that a mesh is a sphere, that a normal is physically meaningful, or that floating-point distance equals real distance. Those are additional service-specific obligations.

Before merge, run a clean Lean build importing every spatial module, scan source and compiled artifacts for `sorry`, `admit`, custom axioms, `unsafe`, and `sorryAx`, and print the axiom dependencies of the governing theorems. Then run all Steel/Kore tests required by this file. Neither success set substitutes for the other.

## Pending theory

- Formalize the integer lattice chart and bounded lookup, including their round trips and connection to `Boxed`; the current checked box result covers only the finite product presentation.
- Instantiate concrete 2-D/3-D coordinate, scalar, unit, affine-frame, and module laws; add metric, orthogonality, isometry, dimension, and exact or error-bounded floating refinement only where the language promises them.
- Define the concrete 51-row Fibonacci/phyllotaxis generator and prove its intended distinctness or separation property under the chosen numeric refinement; the checked command still accepts an arbitrary injective registered offset field.
- Supply concrete registered interpolators and nonnegative-weight certificates for schemes advertised as convex. Add injective or explicitly merged reverse interpolation and lattice scatter-reduce before authorizing colliding spatial writes.
- Refine `SurfaceHit` and support candidates with surface incidence, ray direction, interval, ray parameter, unit normal, admissibility, ordered metric cost, stable feature identity, resting-pose construction, and optional clearance; prove or explicitly trust the service-specific construction.
- Add explicit host `SnapshotVersion` and concurrency validation plus registry tokens/versioning for support, resting-pose, and clearance services; the checked snapshot is the exact Lean world but not an external collision-service epoch.
- Construct executable unique-player resolution, all-51 support validation, resting-pose refusal, allocation-capacity refusal, prototype/default validation, and transactional failure paths from the abstract checked interfaces.
- Fold `EntitySpatialRegistry`, the companion partial `Position` family, and entity/lattice effect wrappers into the general multi-component schema, shared fresh allocation, archetype/relationship repair, global well-formedness metadata, and schema-valid lattice world actions.
- Replace generic `PositionMapping.evaluate` at the accepted boundary with the closed typed mapping IR described above, seal the compiler-facing plan constructors, and prove or audit that Steel can create certificates only through registry validation.
- Prove save/load observational identity, schema/token/version preservation, boundary totality, and surface-complex laws where declared.
- Prove dense lowering equivalence for gather, CSR interpolation, prefix-sum replication, scatter/scatter-reduce, component transactions, and lawful physical relayout.
- Prove Steel and Kore preserve the checked endpoints and contracts across parsing, planning, host callbacks, lowering, inspection, undo, repeated ticks, and persistence.

## Non-goals and open surface questions

This handoff does not choose registry syntax for habitats, frames, `Position<Frame>`, `CellRef<Habitat>`, placements, locators, interpolators, situated pairs, support services, resting-pose policies, clearance, or exact-versus-best-effort spawn. It does not decide whether the allative `to` remains placement syntax, whether a locator is called as a function or through a spatial clause, or how errors are phrased beyond carrying the typed endpoints and refusal phase. Those questions stay in “Open Questions, Next Steps” until ruled by the author.

This handoff does not implement Steel, Kore, CBQN, a physics engine, a mesh library, a manifold system, geodesics, smooth derivatives, chart transitions, topology, boundary policies, save/load, or visualization. It does not prove that a collision mesh is a sphere or that a nearest surface always exists.

This handoff does not prove the phyllotaxis formula, offset uniqueness under IEEE rounding, `sin`, `cos`, `sqrt`, golden-angle accuracy, SIMD equivalence, error bounds, collision-query completeness, no-penetration physics, or performance. The current safety theorem is conditional on the registered typed pattern, embedding, candidate, tie, and resting witnesses.

This handoff does not promise collision-free wheel positions unless a clearance policy is explicitly requested. It does not turn spatial sites into entity parents, derive locators from placements, align equal-resolution planets, or treat physical row order as semantic identity.

## Completion gate

Implementation is complete only when Steel represents every named semantic identity and token above, rejects every invalid bridge before lowering, executes the exact-51 phases atomically in order, persists and revalidates spatial metadata, and passes all positive, negative, two-tick, save/load, relayout, candidate-order, and refusal tests; Kore must consume the same descriptors, expose them in inspection, and preserve atomicity through run, next, undo, edit, save, and reload. The full Lean umbrella build and axiom audit must remain clean, and no document, diagnostic, or test may use C or BQN as the semantic oracle.
