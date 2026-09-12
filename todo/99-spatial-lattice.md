# 99: spatial lattice

Open. Steel and Kore currently use a singleton two-dimensional lattice and positional conventions. The Lean spatial model is broader; its supplied certificates are not constructed by Steel.

## Implementation path remains an author decision

Two paths remain open: establish nominal descriptors and checked certificates before extending the surface, or first prototype five worlds (discrete 2-D and 3-D, continuous 2-D and 3-D, and a sphere) below that boundary.

A prototype may use explicit relationship adjacency and registered geometry functions. It must be described as unchecked and cannot claim the nominal refusal guarantees. Rank-3 support needs an explicit implementation across parsing, planning, storage, persistence, and views.

Do not settle this choice, the spatial surface, or [task 06](06-seeded-fold-and-traverse.md) by implementation accident.

## Required contract

- Independently named habitats and frames retain identity across planning, storage, and persistence. Equal lengths do not authorize alignment.
- Points, vectors, cells, and entity keys remain distinct. Every gather, interpolation, product, copy, and scatter retains its source and destination lineage.
- Lattice axes, embeddings, boundary behavior, placement permissions, and service contracts are declared. Readonly field rank does not change when entities spawn.
- Mutable placement is explicitly permitted; static placement is the default.
- Exact-51 spawn validates the unique source player, every support hit, and every resting pose before allocation. Any failed requirement commits zero rows.
- Support choices use deterministic semantic keys, not storage order.
- Spatial schema evolution extends the existing validated migration and receipt framework without bypassing it.
- Save/load, the next tick, undo, diagnostics, and visible output preserve the accepted world and atomic refusal.
- Kore views use descriptors. Panning, layers, cursors, and fractional hit policy agree between display, selection, and editing; integer painting and exact-float picking cannot disagree.

## Evidence before completion

Build the Lean umbrella and inspect the relevant theorem hypotheses and axiom dependencies. Construct the corresponding evidence in Steel, test malformed inputs at every introduction path, and check dense lowering against the denotation.

Rewrite the quarantined demonstrations from first principles and restore each number's Ano, Japanese, registry, expected results, and explanatory files together. [TODO.md](TODO.md) owns the exact 56-number quarantine; this task overlaps the alias and fold cases.

BQN output is not an oracle. Native-kernel measurements follow contract closure and require a separate explicit benchmark request. No native backend or performance target is established by this task.

## Descriptors and accepted plans

A descriptor must retain schema and declaration identity, version, habitat, carrier, frame/axes where relevant, presence, mutability, and layout. Relationships and services also retain their source and target endpoints. Equal descriptor contents do not collapse independently declared nominal identities.

A query plan needs its domain, order, validity, and lineage as well as its values. For example, reading ground height for actors first establishes actor rows and valid positions, then locates or interpolates into the ground habitat, gathers the field onto those rows, and finally writes through the actor destination map. Equal-length actor and ground buffers are never a substitute for that route.

A checked multi-input mapping admits each input onto one common query domain through declared lineage. It must retain typed projections, registered operations, conditionals, frame transforms, and checked constructors until validation has consumed them. An arbitrary callback over raw buffers does not constitute that accepted mapping.

## Mutable placement

The existing requirement permits a placement declared mutable to change as world state at a barrier while its type stays fixed. Static placement remains schema authority and requires migration to change. The current mathematical treatment of static maps must be reconciled with the mutable case.

Still open: which declarations expose that permission; whether a placement write invalidates location/interpolation evidence or recomputes it at the barrier; and whether compiled plans dereference current placement or are invalidated. The implementation must also define snapshot versions, save/reload, replay, and matching world-state proofs. These choices do not authorize placement-assignment syntax in advance.

## Services and support

A locator takes points in a declared frame and returns cells in a declared habitat with validity per source row. Validate endpoint identity, failure behavior, bounds, and the declared choice policy. Bounds alone do not prove geometric correctness.

An interpolator returns finite support fibers. Validate offsets, source cardinality, target bounds, nonempty successful fibers, duplicate policy, normalization, required nonnegativity, and both lineage legs. Reducing a fiber must produce one result per valid source. Reverse deposition needs injectivity or an admitted merge.

Support projection uses one frozen world/service snapshot for the whole statement. A candidate carries the relevant frame and surface, stable feature identity, hit point, required geometric data, semantic score, and stable tie key. Select by score then stable key. Reject non-total scores, ambiguous ties, stale epochs, and mismatched endpoints. Enumeration order, pointer address, and thread arrival cannot choose the result.

The resting-pose service converts a support hit into the prototype's final origin under its collider/support rules. A hit point is not automatically that origin. Optional clearance checks the whole proposed batch; fresh entity keys alone do not establish physical separation.

## Exact-51 phase order

1. Freeze the input world and relevant service snapshots. Resolve exactly one live Player with a valid position in the required frame.
2. Establish 51 copy occurrences, indexed 0 through 50. Evaluate the registered pattern into finite typed local offsets with any promised separation evidence.
3. Gather the player's anchor through copy-to-source lineage and apply the registered anchored map, preserving all 51 occurrences.
4. Project every seed onto support in the same frozen snapshot. Validate candidates and select one valid best hit for each seed.
5. Compute and validate all resting poses. Run whole-batch clearance when requested.
6. Validate prototype/default values, component carriers, permissions, destinations, and capacity to reserve exactly 51 fresh identities. Keep the reservation uncommitted.
7. Build every new component and presence column over the same copy domain and shared injective allocation map.
8. Commit once. Expose exactly 51 complete entities and preserve unrelated rows and fixed fields. Any failed phase releases preparation/reservations and publishes no partial result.

New entities from this batch cannot become support surfaces for another member of the same batch. The next statement can observe the full committed result. Skipping unsuccessful seeds would be a different command policy and cannot satisfy this exact-cardinality contract.

## Refusal and persistence checks

| Boundary | Required cases |
|---|---|
| Registry/plan | Duplicate identity, wrong habitat/frame, equal-length substitution, missing evidence, undeclared boundary, invalid rank/axes |
| Services | Wrong cardinality or carrier, nonfinite required geometry, stale or mixed snapshots, support miss, ambiguous tie, invalid resting pose |
| Publication | Allocation shortfall, invalid default/prototype value, inconsistent destination map, unmerged collisions, commit failure |
| Persistence | Changed schema/service fingerprints, corrupt shape/presence, unavailable evidence reconstruction, incompatible migration |

Each failure must preserve the relevant world, allocation state, component presence, fixed fields, undo head, outputs, aliases, and session/log boundary. Diagnostics identify the phase and actual/expected endpoints. Check failure of one member of the 51, including failures late in preparation.

Saved data carries stable declaration IDs, versions, and fingerprints. Private authorization values are reconstructed after validation, never trusted because their bytes were saved. Plans depend on every referenced schema/service/policy version; prepared snapshot-dependent results cannot remain authority after their snapshot changes.

Extend the existing migration transaction to spatial declarations, state, cached plans, aliases, views, and undo boundaries. A rejected migration keeps the old active world and session usable.

## Acceptance examples

Use two equal-cardinality habitats with different shapes to show that only explicit maps permit cross-domain work. Test a valid injective write and a colliding write with a declared merge. Permute storage and support-candidate order to check that semantic observations stay stable.

Preserve fixed field shape and values outside the selection through entity presence changes, spawn/despawn, save/reload, and two successive ticks. Exercise live positions together with heterogeneous ground fields on one checked query domain.

A14 requires a two-coordinate `to` result to fit any compatible position carrier with at least two declared spatial axes; lattice placement is not required. A15 permits fractional positions. Test those cases through save/load, render, selection, inline edit, and undo, including a non-square layout that exposes axis swaps.

Kore inspection must show habitat, shape/rank, carrier, axes, and policy where applicable. Exercise logical views without placement, world views with placement, panning, layer toggles, visible edit cursors, and a refused tick that leaves the displayed world unchanged.

Test a successful spatial migration and incompatible habitat, frame, rank, placement, and service changes. After a successful exact-51 step, save/reload and run the next step against the reconstructed state. These cases supplement the proof obligations; they do not remove their hypotheses.
