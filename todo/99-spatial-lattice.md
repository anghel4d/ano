# 99 — verified spatial lattice, habitats, placement, lineage, and Kore views

Status: final implementation task. This file replaces the overlapping spatial outcome, typed handoff, registry-surface, placement-seam, view, and demo-demolition notes. It first fixes the verified current boundary, then states the smallest coherent implementation and acceptance contract. It does not authorize speculative surface syntax or count abstract Lean objects as a finished Steel/Kore implementation.

## Authority and trust order

`docs/ano-language.md` is the language contract. `docs/spatialmaths.md` is the mathematical account. `docs/ano-ecs.md` is the column-store contract. `proofs/foundations.md` and the imported `proofs/Ano/*.lean` modules define the checked mathematical boundary. Steel is the reference compiler and standalone launcher. Kore is the interactive world. BQN siblings are explanatory mathematical witnesses, not semantic or differential oracles.

The implementation order of authority is:

```text
mathematics → denotation → typed domain/lineage plan → grammar → surface → lowering
```

A representation fact such as equal buffer length, equal item width, contiguous storage, or an inferred pair order can never authorize semantic alignment.

## Verified current implementation boundary

The current Rust implementation is a useful prototype but is not the contract described below:

- The registry has one global `lattice <w> <h>` state represented by lattice width/height and fields sized from `w*h`. It does not yet provide independently named field habitats whose identity survives planning and persistence.
- The parsed shape representation is limited to one or two dimensions. Rank 3 is not “the same code with one more axis”; parser, registry, planner, lowering, persistence, diagnostics, and views all need explicit support and tests before a 3-D claim is valid.
- The emitter contains implicit frame/domain categories and recognizes conventional `x`/`y` names. Those conventions are not registry-declared nominal frame, axis, position-role, or lineage evidence.
- Kore's current spatial views infer meaning from raw pairs/buffer shape in places. Painting a fractional point through integer conversion while selecting it for edit by exact floating equality gives different visible and editable cells.
- The Lean umbrella imports the spatial modules and the repository documents a clean-build/axiom-audit procedure. Its own trust notes still leave Steel certificate construction, concrete phyllotaxis and geometry, persistence, general ECS integration, boundary policies, mutable placement, dense lowering, and several service refinements pending. Abstract theorems are therefore implementation licenses conditional on supplied structures, not proof that Steel/Kore already construct them.

Before merge, independently run the full Lean build and axiom/source audit required below. This todo audit does not substitute source inspection for that build result.

### Standing corrections

The following are law-backed and hold on every path:

- presenting unchecked prototype fixtures as certificate evidence;
- auto-minting unnamed/default frames where nominal identity is the semantic license;
- deriving a habitat, position carrier, placement, axis order, or lineage from buffer length or item width;
- treating a physical layout as the habitat or as a semantic row map;
- using the old numbered spatial demonstrations as acceptance evidence before their total rewrite.

### Two implementation paths, decision open for the spatial commit

The earlier surface note proposed a five-world prototype path; this file's default sequencing is certificate-first. Both are preserved here, and the author chooses at implementation time.

The certificate-first path builds nominal IDs, descriptors, sealed tokens, and typed lineage before any surface accumulates. Nothing lands as inert semantic metadata; every admitted noun or adjective maps to a checked descriptor or is refused; rank 3 is not advertised until parser, registry, planner, lowering, persistence, diagnostics, and views all support and test it.

The prototype path stands up five worlds below the certificate line—discrete 2-D grid, discrete 3-D voxels, continuous 2-D euclid, continuous 3-D euclid, continuous 3-D sphere—on the existing lattice machinery. Its lawful shortcuts: the Conway precedent materializes neighborhoods as an explicit `srel` over a fixed field, so the sphere's adjacency is an icosphere or lat-long relation built at load and queried through existing relation machinery; great-circle distance and slerp enter as registered host callables, proof-carrying input rather than derived law; and frame/axis/placement surface is parsed and stored as explicitly labeled unchecked convention while the certificate layer wires in later. Its non-negotiable discipline: every fixture checks rank, shape, and habitat after each tick and runs a second tick as `--! expect` assertions; the sphere trap lives in the fixture, since naive lerp leaves the sphere, so positions renormalize at the barrier or move by registered slerp; every prototype header says convention-not-certificate and never overclaims; and the fixtures are not throwaway—they are the acceptance corpus the certified path must later reproduce byte-identically. Its price is refusal, not function: until the certificate layer lands, the prototype will not refuse a foreign-habitat join, a frame mix, or a buffer-length coincidence—the documented first-tick-plausible, second-tick-incompatible failure. The prototype path also claims rank-3 voxels ride the rank-2 code with one more axis; the certificate-first path disputes exactly that claim, which is part of why the choice is explicit.

## Required denotation

### Habitats, layouts, and stored columns

A habitat is nominal. For distinct declarations `H` and `K`, equal cardinality or equal shape does not imply `H = K` and does not authorize alignment.

A total stored field has type:

```text
field : H → V
```

Its habitat identity, rank, shape, and declared layout survive ordinary value updates, masks, spawn/despawn, save/reload, and successful ticks. A physical layout is an isomorphism between row slots and one already-known habitat; it is not the habitat and is never a source-level semantic capability.

An ECS component is partial on the live entity habitat, with explicit presence. A fixed field remains total unless its schema explicitly declares sparsity. Spawning or despawning entities cannot resize a fixed field.

A box/product presentation gives a habitat rank, shape, and coordinate/site chart. It is distinct from physical layout. A shape-changing read produces a derived value/domain and never mutates the source field's stored shape.

### Query domains and carriers

Every planned value has two independent types:

```text
TypedColumn<row domain, value carrier, validity>
```

Examples:

- `Col<EntitySelection, Point<World3>>`
- `Col<GroundCells, f64>`
- `Col<InterpolationEdges, CellRef<Ground>>`

A point carrier, vector carrier, cell reference, entity reference, scalar, and opaque value remain distinct even when their physical encodings have the same width.

Every non-scalar pointwise operation consumes values on one current query domain. Scalar broadcast is explicit. Matching row counts are never consulted for compatibility.

### Lineage

Selection, relationship traversal, product, replication, grade, reshape, generation, location, and interpolation retain enough typed lineage for a later gather or effect.

The accepted lineage constructor set is closed and provenance-bearing. It includes the semantic equivalents of:

- identity;
- selection inclusion;
- product projections;
- replication source;
- typed relationship endpoint;
- component-presence inclusion;
- registered query-to-habitat map;
- locator point-to-cell result;
- interpolation source and cell legs.

It deliberately excludes `FromEqualLength`, `FromEqualShape`, `FromLayout`, `FromCarrierWidth`, raw integer vectors, and unchecked callbacks.

Frame/placement transforms act on value carriers while preserving rows. Reindexing/lineage acts on rows while preserving carriers. A 2-D-to-3-D coordinate transform cannot become field write authority.

### Position and axes

Position semantics are declared on the carrier, not inferred from the column name. A position component is a partial entity column with carrier `Point<Frame>` and explicit presence.

Raw coordinate pairs have no universal `x/y` or `row/column` meaning. The destination point carrier declares ordered axes and Kore renders/edits through those axes. Coordinates may be fractional.

A two-coordinate `to` result is valid whenever the destination carrier has at least two declared spatial axes (A14); it does not require a declared lattice placement. It is legal exactly when registry declarations determine a compatible destination position carrier with at least two spatial axes, and otherwise remains ambiguous. Placement is separately required only for an operation that maps lattice sites into a world frame.

### Lattice, boundary, placement, locator, interpolation

A lattice capability names a nominal logical habitat, rank/shape, coordinate/site chart, and explicit boundary policy for each admitted neighborhood operation. Shrink, constant, clamp, reflect, and wrap are observably distinct; no global default silently chooses one.

A lattice can exist without world placement.

A placement is a registered map:

```text
CellRef<H> → Point<F>
```

It requires declared endpoint identities and enough frame/coordinate/unit data to be meaningful. A locator is a separate partial service:

```text
Point<F> ⇀ CellRef<H>
```

An interpolator is a separate finite weighted support relation from source points to cells of one habitat. Placement does not imply a locator; a general locator does not imply a placement round trip; an interpolator does not imply lawful reverse write-back.

### Effects and atomicity

A plain assignment has at most one value for each destination. Destination uniqueness is checked on valid rows. A colliding effect is admitted only through a declared merge whose required order law is available; otherwise it refuses before commit.

Every successful statement returns a world satisfying the same validated schema and suitable for the next tick. Every refusal returns the exact input world and leaves allocation state, undo history, output, and statement log at the refusal boundary.

## Checked proof boundary versus pending implementation

The imported Lean work provides abstract checked structures and laws for finite box presentations, layout round trips/relayout invariance, nominal frames, affine torsors and registered linear maps, sealed lineage, selection and assignment laws, normalized finite interpolation fibers, typed heterogeneous inputs, a partial live `Position` component path, support-selection assumptions, exact `Fin 51` copy cardinality, atomic success/refusal at the modeled boundary, and repeated reconstruction from successor worlds.

Those results may be used only with their stated hypotheses. The following remain implementation or theory obligations and must not be described as already proved in Steel/Kore:

- constructing the exact certificates from registry declarations;
- the concrete integer lattice chart and bounded lookup laws beyond the existing finite product presentation;
- concrete 2-D/3-D coordinate, unit, metric, numeric-refinement, and IEEE behavior;
- the actual phyllotaxis/golden-angle formula and its floating distinctness/separation properties;
- concrete locator/interpolator/raycast completeness, incidence, normals, metrics, stable feature identities, and host snapshot epochs;
- nonnegative/convex interpolation where promised, reverse interpolation, and merged spatial deposition;
- boundary-policy totality;
- mutable placement semantics;
- general multi-component/archetype allocation, relationship repair, despawn behavior, and transactional storage;
- save/load observational identity and token reminting;
- dense gather, interpolation, replication, scatter, scatter-reduce, and relayout equivalence;
- Steel/Kore preservation across parsing, planning, callbacks, undo, repeated ticks, and views.

## Required registry and IR capabilities

The exact Rust layout and registry spelling remain design choices, but the validated schema/plan must retain at least these nominal identities and versions:

```text
Schema, Habitat, QueryDomain, Frame, Carrier, Field, Component,
Relation, Layout, Axis, Service, Policy, Prototype, Surface,
SchemaVersion, SnapshotVersion
```

Equal descriptors do not collapse nominal IDs. Persistence restores identity only after matching the saved schema and declaration fingerprints.

Required semantic descriptors include:

- habitat kind, cardinality, optional box presentation, and default physical layout;
- box rank/shape and coordinate/site operations;
- frame dimension, scalar carrier, point/vector carriers, axes, units, and separately declared numeric/metric laws;
- carrier kind: scalar, mask, point, vector, cell reference, entity reference, surface reference, or opaque;
- column habitat, carrier, totality/presence, mutability, layout, and values;
- relationship source/target habitats and key/foundness contract;
- boundary, placement, locator, interpolator, support, resting-pose, clearance, and merge services with exact endpoints and versions.

Authorization handles have private constructors. Registry validation mints them only after endpoint and obligation checks. Saved bytes store stable declaration IDs/versions/fingerprints, not trusted private token values.

A checked expression plan retains the equivalent of:

```text
TypedColumn { domain, carrier, validity, value }
QueryDomain { origin, order, lineage }
LineageEdge { source, target, map, validity, provenance }
```

No semantic member may be erased before validation has consumed it and recorded an equivalent checked operation.

## Checked mappings and field/entity interaction

A registered n-input mapping is a typed operation over one query domain, not an unchecked callback over equally long buffers. Each input records its carrier and its admitted route onto the common rows: fixed field lineage, live-component presence, located field, registered interpolation, or an already-checked derived column.

The accepted mapping body is a closed typed IR containing projections, registered scalar/carrier operations, conditionals on one domain, authorized frame transforms, locator/interpolator nodes, and checked constructors. It does not contain an arbitrary Rust closure, raw function pointer, or buffer-index convention.

The canonical entity-over-field path is:

1. establish entity query rows;
2. validate live `Position<F>` presence;
3. locate or interpolate those points into field habitat `H` through exact tokens;
4. gather field values onto the entity rows or interpolation-edge rows;
5. reduce support fibers back to the entity rows;
6. evaluate the registered mapping;
7. scatter through entity lineage or a fresh allocation map.

For a lattice-target effect, the final plan carries a registered destination into `CellRef<H>`. Plain assignment proves injectivity on valid rows. Noninjective deposition requires an explicit admitted merge. The current field-level Lean assignment result must not be presented as a general schema-valid world transaction until global invariants and storage integration are supplied.

## The mutable-placement seam

The taxonomy is ruled: placement and transform values may change under a fixed `Σ` while their declared types stay fixed. The current Lean kernel models the opposite side—frame/offset maps as static schema data—so the work is reconciliation toward the ruling, not a fresh choice. A placement not declared mutable remains static schema authority whose change requires `Σ → Σ′` migration; a placement declared mutable is a world value written only at statement barriers.

A declared-mutable placement must define:

- effect permission and destination lineage;
- per-tick transport laws or trusted runtime validation;
- snapshot/version semantics;
- invalidation or dynamic dereference of compiled plans;
- invalidation/recomputation of locator/interpolation/support data;
- save/reload and replay behavior;
- matching Lean/world-state proofs.

Open implementation sub-questions, surfaced rather than silently resolved: which placement declarations take the mutable permission, defaulting to static; whether sealed lineage and interpolation tokens survive a placement write or refuse until the barrier recompute; and whether compiled plans reference the placement token and dereference at tick time or are invalidated by the write. Until the reconciliation lands, no Ano assignment syntax for placement is accepted and no document may count placement mutation among implemented operations.

## Locator, interpolation, and support-service validation

A locator accepts a frozen batch of `Point<F>` and returns one `CellRef<H>` plus validity per source row. Bounds checking alone is not a semantic choice certificate. The registration/runtime contract must provide or validate endpoint identity, explicit failure, soundness, and single-valued choice under the named policy.

An interpolator returns finite CSR-like fibers into exactly one habitat, with one weight per edge. Before metadata erasure, validate:

- offsets and source-fiber cardinalities;
- cell bounds and habitat endpoints;
- nonempty successful fibers;
- duplicate-cell policy;
- normalization under the declared numeric contract;
- nonnegative weights when convexity is advertised;
- exact source and cell lineage legs;
- one projected result per valid source row.

Reverse interpolation or many-source deposition is never ordinary assignment by default. It needs injectivity or a declared scatter-reduce merge.

A support projector is snapshot-dependent. Every candidate for one statement uses one immutable world/service snapshot version. Candidate data includes exact frame/surface endpoints, validity, stable feature identity, hit point, any required normal/ray/incidence data, semantic score, and stable semantic tie key.

`Best(seed, hit)` chooses the least semantic score and then the least stable semantic key. The score must be total on admitted values. NaN/incomparable scores, duplicate tie keys for distinct equal-score candidates, endpoint mismatches, stale snapshots, or order-dependent output refuse. BVH row order, pointer address, thread arrival, hash iteration, and backend buffer position are forbidden tie breakers.

A resting-pose service is prototype-, surface-, and frame-specific. It maps a contact hit to an entity origin and validates required support/collider rules. The raw hit is not automatically the entity origin. Clearance is a separate optional whole-batch check; fresh database keys do not prove physical non-overlap.

## Exact 51-copy spatial spawn

The exact policy has a fixed semantic phase order. Reordering these phases is not an optimization.

1. Freeze the input world and all relevant host/service snapshot versions.
2. Resolve `Player` to exactly one live entity carrying `Position<World3>`. Zero, multiple, dead, or missing-position cases refuse.
3. Construct the copy habitat `C = Fin(51)` and indices `0…50`.
4. Evaluate the registered phyllotaxis/pattern service once over `C`, producing finite typed `Vector<PlayerPlane2>` offsets and any declared uniqueness/separation witness.
5. Gather the player's anchor point through copy-to-source lineage and apply the exact registered anchored offset map to produce `Point<World3>` seeds without changing `C`.
6. Evaluate all 51 seeds through one frozen support projector. Validate candidates, endpoints, total score, stable ties, and unique best results. One failed seed refuses the whole batch.
7. Apply the registered resting-pose service to all hits and validate all 51 final origins.
8. When requested, run clearance over the complete 51 proposals and the frozen pre-state.
9. Validate prototype/default data, component schema, effect permissions, destination plans, allocator capacity, and the ability to reserve exactly 51 fresh identities. Allocation remains uncommitted.
10. Build every component/presence column over `C` and scatter all of them through the same injective allocation map.
11. Commit once. Success exposes exactly 51 complete live entities and preserves every unrelated old row and every fixed field/habitat/shape. Any failure aborts the entire reservation and transaction.

All support selection observes the pre-spawn snapshot. A wheel created by this statement is not a candidate surface for another wheel in the same statement. The next statement/tick may observe the full committed batch.

Best-effort “skip failed seeds” behavior is a different policy with a different type and diagnostics. It must not reuse the exact command's contract.

## Refusal matrix

| phase | examples | required result |
|---|---|---|
| registry load | duplicate IDs; missing habitats/frames/carriers; invalid endpoints; changed fingerprints; unvalidated law claims | registry/world load fails; no unchecked downgrade |
| plan typing | foreign habitats/frames; equal-length alignment; layout/raw indices used as lineage; local vector used as world displacement; placement used as locator; missing token | refusal before CBQN or host execution |
| lattice operation | undeclared boundary; invalid chart/coordinate; rank/axis mismatch | refusal before backend execution |
| player resolution | zero/multiple/dead player; missing or foreign-frame position | exact input world; no copy domain or reservation |
| pattern | wrong cardinality/carrier; service failure; nonfinite offset; required uniqueness failure | exact input world; no support call |
| snapshot | unavailable, stale, mixed, or changed service epoch | exact input world; discard preparation |
| support | miss; invalid feature/frame/incidence; NaN score; duplicate semantic tie key; nonunique best | exact input world; zero spawned entities |
| resting/clearance | invalid pose/collider/support; nonfinite origin; overlap/policy failure | exact input world; zero spawned entities |
| allocation | insufficient capacity; key/generation collision; cannot reserve exactly 51 | release reservation; unchanged allocator observation |
| component preparation | missing default/prototype; wrong carrier; inconsistent allocation map | abort; no partial live entities |
| ordinary effect | duplicate destination without injectivity/merge; stale target; schema/service race | rollback every value/structural write |
| persistence | schema/fingerprint/endpoint mismatch; unremintable token; corrupt shape/layout/presence | atomic load refusal; old world/session stays active |

Diagnostics must name both actual and expected habitat/frame/carrier endpoints and the refusal phase. No backend placeholder or partial commit may become observable.

## Persistence, caches, and lowering

A saved spatial world retains enough typed metadata to reject reinterpretation:

- schema ID/version/fingerprint;
- nominal habitats, frames, carriers, axes, units, and declaration fingerprints;
- every field/component's habitat, carrier, totality/presence, rank/item shape, layout, and values;
- lattice box/chart and boundary declarations;
- relationship endpoints and key contracts;
- placement, locator, interpolator, support, resting, clearance, score, tie, merge, and prototype service IDs/versions;
- live entity keys/generations and component presence;
- any declared-mutable placement state, once the ruled placement arm is implemented and proved.

Private authorization tokens are never trusted from bytes. Load validates the active registry, matches endpoints and fingerprints, remints tokens, and only then decodes buffers. A mismatch refuses atomically; equal dimensions or shapes are not a fallback.

Compiled plans are keyed by schema fingerprint and every referenced service/policy version. Changed habitat/frame/carrier/bridge/boundary/prototype/service data invalidates the plan. Snapshot-dependent prepared batches never survive as reusable authority across world or host-service versions.

Extend `05`'s atomic `Σ → Σ′` framework rather than inventing a spatial-only replacement. Supply typed migrations or exact refusals for habitat/frame/carrier identities, lattice rank/shape/chart/boundary changes, placement and service endpoints, stored fields/components, spatial plan caches, and persisted alias/view handles. A failed spatial migration retains the exact old schema, world, session overlay, view descriptors, undo head, and replay boundary.

Typed semantics remain present until lowering has consumed them. The backend may then lower to dense columnar kernels:

- selection → bitmap or compact inclusion;
- lineage/reindex → checked gathers;
- product/replication → paired indices, counts, and prefix sums;
- interpolation → validated CSR, gathers, weights, and segmented reductions;
- effect → checked destination vector and scatter/scatter-reduce;
- allocation → one fresh-key vector shared by every component column.

Steel must reject an invalid plan before emitting CBQN. Backend output is tested against the typed Ano semantics, not accepted because equal-length BQN arrays happened to agree. Dense-lowering equivalence remains a proof/audit obligation.

## Kore requirements

Kore consumes the same validated descriptors and plan results as Steel; it does not reconstruct spatial meaning from buffer shape.

### Inspection and views

- Field inspection displays habitat identity, rank, shape, carrier, axes where applicable, boundary policy, and placement/view availability.
- Equal-length foreign fields remain visibly distinct and cannot collapse into one anonymous “ground.”
- Map and bitmap views use a declared field/lattice habitat and declared spatial axes.
- Entity overlays use a registry-declared position role and `Point<Frame>` carrier. They do not assume a component named `pos`, `x/y` columns, or `pair[0] = x`.
- A view that needs world placement requires a matching placement; a purely logical lattice view may render chart coordinates without inventing world coordinates.
- Fractional positions remain visible and editable. Targeting uses one declared deterministic hit policy—such as the registered locator/cell, nearest visible point with a documented tie rule, or an explicit pixel-to-world tolerance—not integer truncation for paint plus exact floating equality for edit.
- Edits resolve through typed entity/field lineage and normal effect validation, never a raw displayed buffer index.

### Interaction backlog folded here

- Add viewport panning without changing the semantic axes or stored coordinates.
- Add a ground/layer visibility toggle that selects declared view layers rather than guessing from buffer length.
- Make the edit-mode cursor visible. The terminal hardware-cursor park already exists in `kore/src/term.rs`; edit mode must supply the current visible cell/overlay coordinates through the same viewport transform used for painting.
- Keep an inline-edit overlay legible while preserving the actual fractional value and declared carrier.
- `r`, `n`, `u`, direct edit, save, and reload preserve habitat/frame metadata and transaction boundaries.
- A refused spatial tick leaves the play world, undo head, output pane, and edit/view model at the same atomic boundary as any other refused tick.
- Pressing `n` twice is required acceptance behavior. The second tick consumes the exact first post-state; it is not an optional stress test.

## Surface gate

No spatial declaration syntax is approved merely by this task file. Before parser work, the author must approve a coherent registry surface for the capabilities actually implemented.

The surface must satisfy these constraints:

- declarations live only in the registry language;
- habitats and frames retain unavoidable nominal names;
- layout is never surface syntax;
- syntactic bundles/defaults expand during validation into separate certificate fields and never weaken checking;
- on the certificate-first path, every admitted noun/adjective maps to a checked descriptor or is refused; on the prototype path, unchecked surface is stored only as explicitly labeled convention-not-certificate and is never advertised as a semantic capability;
- rank support is stated exactly; rank 3 is not advertised until end-to-end tests pass;
- placement, locator, interpolator, boundary, support, and mutable-placement permissions remain distinct capabilities;
- the registry extension/noun-order/block-form questions are coordinated with `05`;
- docs show only built and checked surface, with unimplemented proposals remaining explicitly open.

Open surface choices include noun order, line versus block declarations, exact placement sugar, locator/interpolator invocation, axis declaration syntax, error wording, whether an axis-kinded column may default to the sole declared frame, whether rank-3 support lands in the first certified slice, and the certificate-first versus prototype path above. These choices may change spelling, not the denotation or acceptance gates in this file.

## Implementation sequence

0. **Remove false evidence.** Decommission the 64-number global set according to `TODO.md`; quarantine each number across every sibling artifact and remove it from manifests, README counts, and default proof runs. Record current Steel/Kore failures before replacing anything.
1. **Verify the proof and code boundary.** Run the Lean umbrella build/axiom audit, enumerate current Rust registry/parser/emitter/Kore assumptions, and save the results. Do not proceed from a stale “proven” status line.
2. **Obtain the surface decision.** Approve only syntax that validates the first capability slice; reject inert metadata and unsupported rank claims.
3. **Introduce nominal IDs/descriptors.** Schema-versioned habitats, frames, carriers, fields/components, axes, layouts, services, policies, and sealed authorization handles.
4. **Build typed query domains and lineage.** Make every expression carry domain, carrier, validity, order, and closed provenance; reject foreign/equal-length coincidences before lowering.
5. **Replace the singleton lattice.** Support independently named lattice/field habitats, rank/shape/chart, total fields, explicit boundary policies, and physical-layout separation. Rank 3 lands only if implemented and tested through every layer.
6. **Build position and bridge capabilities.** Partial `Position<Frame>`, axis-aware `to`, placement, locator, interpolation, checked n-input mappings, and injective/merged effect destinations.
7. **Implement the placement seam.** Land the ruled mutable-placement arm—declared-mutable slots with static default—consistently in Lean, docs, persistence, caches, and runtime.
8. **Integrate exact services and spawn.** Snapshot/version validation, deterministic best-hit selection, resting/clearance, exact 51 allocation, all-component preparation, and one atomic commit.
9. **Persist and lower.** Token reminting, save/reload identity, plan invalidation, checked dense lowering, and repeated-tick correctness.
10. **Repair Kore.** Descriptor-driven inspection/render/edit, fractional hit policy, panning, layer toggle, edit cursor, undo/save/reload atomicity.
11. **Rewrite demonstrations from first principles.** Reactivate a number only after its new Ano/Nihongo/registry/output siblings prove the current contract and any cross-task dependencies have landed.
12. **Measure only after closure.** Once every semantic, proof, persistence, two-tick, and view gate above is green, measure one checked representative native kernel as specified below; the measurement may select a backend but cannot revise the semantics.

## Acceptance suite

### Positive Steel/Kore cases

- Load at least two named field habitats simultaneously, including equal-cardinality habitats with different rank or shape.
- Perform a legal cross-habitat operation only through an explicit registered map/lineage and show the exact endpoint identities in inspection.
- Preserve a field's habitat, rank, shape, layout identity, values outside a mask, and boundary policy through update, spawn/despawn, save/reload, and two ticks.
- Show that ECS component presence changes do not resize a fixed field.
- Exercise exact reshape, cycling reshape, replication, grade, generated domains, and later effects with the required lineage.
- Store and operate on `Point<Frame>`, `Vector<Frame>`, and `CellRef<Habitat>` without accepting equal-width substitutions.
- Accept a two-coordinate `to` into any compatible declared position carrier with at least two spatial axes (A14), with no lattice placement required; preserve fractional coordinates through Steel, save/reload, render, target, and edit.
- Exercise distinct boundary policies with observably distinct edge behavior.
- Use a lattice without placement in a logical view; use placement only for a world-frame operation.
- Validate a locator and an interpolator with exact endpoints, failure rows, CSR fibers, normalization, and deterministic output.
- Resolve at least three heterogeneous registry inputs—for example live `Position<World3>`, `Ground.Height`, and `Ground.Material`—onto one entity query domain through presence plus locator/interpolator lineage, evaluate a checked mapping, and write entity positions without ever aligning the entity and lattice buffers by length.
- Execute an injective spatial write and a colliding write with an admitted merge; preserve result under allowed evaluation-order permutations.
- Execute the exact 51-copy command, observe exactly 51 complete new entities, and preserve every unrelated row and fixed field.
- Run two consecutive Kore ticks where the second consumes the first post-state.
- Save after the first success, reload, remint/revalidate tokens or recompile, and complete the second success with identical observations.
- Exercise one compatible spatial `Σ → Σ′` migration and refusals for incompatible habitat, frame, rank/shape, placement, and service-endpoint changes, proving exact rollback through the `05` framework.
- Relayout physical storage and obtain the same semantic observations.
- Permute host candidate enumeration and obtain the same best-hit results and trace order.
- Exercise Kore inspection, panning, layer toggle, fractional targeting, inline edit, visible edit cursor, undo, and refused-tick atomicity.
- Keep Ano and Nihongo siblings semantically identical. BQN witnesses explain arithmetic only and never decide acceptance.

### Compile/plan refusals before backend execution

- Pointwise combine or cross-write equal-length/equal-shape foreign habitats without a map.
- Use physical layout, raw row indices, item width, rank, or buffer length as lineage.
- Assign a rank-changing derived value into a fixed field without a legal destination map.
- Invoke a neighborhood without a declared boundary policy.
- Mix foreign frames; add a local vector directly to a world point; use a raw numeric vector as `Point<Frame>`; use a point map without the exact token.
- Use placement as locator, a general locator where a situated round trip is required, or a service with wrong frame/habitat endpoints.
- Store a cell index into an entity parent/relationship without explicit entity lineage.
- Plain-scatter duplicate destinations without injectivity or a merge.
- Use a raw function, untyped callback, differently-domained column, raw interpolation leg, invalid normalized/convex weights, or colliding reverse support in a checked mapping.
- Advertise rank 3, a metric, an embedding, an isometry, convex interpolation, or mutable placement without the corresponding descriptor and validation rung.

### Runtime refusal identity

For every failure, assert exact world/state identity including allocator/free-list observation, component presence, fixed-field values and metadata, undo head, output, alias/session boundary where relevant, and statement log boundary.

Cover zero/multiple/dead/missing-position player resolution; any one of 51 pattern/support/resting/clearance failures; stale snapshots; duplicate tie keys; nonfinite scores; wrong endpoints; exact-allocation failure; one bad prototype/default/component; commit failure; failed second tick; and save/load mismatch. The result is always zero committed copies, never 50 or a partial archetype.

## Proof and trust verification before merge

Run from a clean checkout and archive the command output with the change:

```text
cd proofs
lake build
```

Then inspect source and compiled dependencies for `sorry`, `admit`, `sorryAx`, custom axioms, and `unsafe`; print the axiom dependencies of the governing spatial theorems imported by the `Ano` umbrella. Review every trusted-host declaration and identify its runtime validation and version boundary.

Run the complete Steel/Kore suite, all new positive/refusal/save/reload/two-tick/property cases, and the demo manifest. The Lean result and the Rust/Kore result are independent gates; neither substitutes for the other. No C implementation or BQN output may be used as the semantic oracle.

## Post-closure native-kernel measurement

Only after the full spatial acceptance and proof/trust gates are green, measure one representative masked update whose typed plan exercises the completed habitat and lineage semantics. Record in `docs/ano_jit.md`:

- world sizes and field/component shapes;
- allocation counts and bytes;
- steady-state throughput and latency;
- compile time and amortization point;
- comparison with the checked CBQN lowering;
- identical output and post-state under the same typed plan;
- hardware, compiler, build mode, warm-up, sample count, and repetition methodology.

This measurement chooses among implementations; it is not evidence for the denotation. Equal length, raw layout, benchmark convenience, or faster output may not weaken any habitat, lineage, refusal, persistence, or repeated-tick gate.

## Sentinel

One executable witness for the category, carried from the demolition ledger; explanatory mathematics, not a semantic oracle. Habitats are nominal: equal shape and cardinality never make two habitats one. The q sentinel carries habitat metadata explicitly because an unadorned q vector does not encode Ano's nominal habitat; it witnesses the refusal of length-based alignment, not a claim that q supplies Ano's spatial type system.

```bqn
ground ← ⟨"Ground", 2‿3, ↕6⟩
mars ← ⟨"Mars", 2‿3, ↕6⟩
SameHabitat ← {(0⊑𝕨) ≡ 0⊑𝕩}
! ¬ ground SameHabitat mars
! (≠2⊑ground) = ≠2⊑mars
```

```q
ground:(`habitat`shape`values)!(`Ground;2 3;til 6);
mars:(`habitat`shape`values)!(`Mars;2 3;til 6);
sameHabitat:{x`habitat~y`habitat};
not sameHabitat[ground;mars]
(count ground`values)=count mars`values
```

## Demo decommission and rewrite ownership

The original spatially invalid evidence set contains 56 numbers. Primary ownership in this file is the following 53 numbers:

```text
034–037, 044, 046–047, 050–053, 055–070, 072–078,
089–091, 099, 101–102, 104–110, 114–119
```

Three additional spatially dependent numbers are owned by earlier tasks so the global total is not double-counted:

- `033` — primary owner `03`;
- `071` and `093` — primary owner `02`.

Decommissioning is by number: English Ano, Nihongo Ano, BQN, registry, expected output, README/test claims, and manifest entries all leave the active evidence surface together, including unusual artifact mixes such as `072` and the spatial portion of `076`.

A rewritten number must prove nominal habitat identity, declared axes/boundaries, lawful lineage/effect destinations, repeated-tick behavior where applicable, and exact refusal/atomicity. It cannot be reactivated because its arithmetic still runs on a raw buffer.

Do not expand the demolition list merely because another valid demo uses a relationship, `&`, `|`, a fold, or a scan. Still-correct controls listed in `TODO.md` remain active.

## Completion gate

This final task is complete only when:

- independently named habitats and declared lattice capabilities replace singleton/buffer inference;
- every planned value carries checked domain, carrier, validity, and closed lineage until legality is established;
- positions, axes, placement, locator, interpolation, boundaries, and effects obey the distinctions above;
- the ruled mutable-placement arm is implemented, proved, documented, persisted, and cache-safe, with static-by-default placements requiring migration to change;
- the exact 51-copy transaction follows all phases and is atomic on every refusal;
- save/reload, spatial schema migration/rollback, relayout, candidate-order, and two-tick tests pass;
- Kore renders and edits through the same descriptors, including fractional targeting and edit-mode cursor behavior;
- the full Lean build/axiom audit and the complete Steel/Kore suite are independently green;
- the post-closure native-kernel measurement is recorded without using backend agreement as the semantic oracle;
- all 64 decommissioned demo numbers are either still quarantined or totally rewritten against the current contract;
- no active test or document names C, CBQN, raw length agreement, or unchecked prototype metadata as the semantic authority.
