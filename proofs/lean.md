# Lean semantic kernel

Status: first ten-result milestone machine-checked with Lean 4.30.0, 2026-07-16. The kernel uses Lean `Std` only and lives under `proofs/Ano/`.

Run `cd proofs && lake build` from the repository root. The umbrella target imports every proof and negative compile-time witness.

## Milestone map

1. Layout round-trip: `Layout.decode_encode` and `Layout.encode_decode` prove that a dense buffer and its semantic field encode and decode without loss.
2. Layout-permutation invariance: `Layout.change_encode`, `Layout.decode_change`, `Layout.change_refl`, and `Layout.change_trans` prove that relayout changes storage order but not the decoded field.
3. Reindex identity: `Field.reindex_id`.
4. Reindex composition: `Field.reindex_comp` and `Field.reindex_comp_operator`.
5. Selection gather/scatter correctness: `Selection.gather_scatter`, `Selection.gather_modify_scatter_selected`, `Selection.gather_modify_scatter_unselected`, and `Selection.gather_modify_scatter_exact` use the frozen selection inclusion for both directions.
6. Injective-scatter determinism: `Scatter.injective_scatter_deterministic`, `Scatter.at_most_one_writer`, `Effects.assignment_fiber_subsingleton`, and `Effects.assignment_occurrence_fiber_subsingleton`; `Effects.executeCertifiedAssignment` requires a complete duplicate-free schedule and an injective destination map.
7. Commutative-merge permutation independence: `Effects.commutative_merge_permutation_independent` and `Effects.executeMerge_permutation_independent` require an explicit `CommMonoidLaw` witness.
8. Spawn cardinality and source lineage: `Copies.length_enumerate`, `Copies.enumerate_nodup`, `Copies.mem_enumerate`, `Copies.source_unique`, and the `Copies.gather_*` laws; `Allocation` proves old/fresh disjointness and injective allocation of copy slots.
9. Fixed-field preservation under spawn: `World.spawn_fixed`, `World.spawn_field`, and `World.spawn_wellFormed` leave every registered fixed field definitionally unchanged while advancing the entity population under the schema's spawn-preservation law.
10. World well-formedness through arbitrary finite successful ticks: `perform_ok_wellFormed`, `ticks_wellFormed`, and `Effects.validated_ticks_wellFormed` preserve local field invariants and the schema-wide invariant over population plus all registered fields.

Checked assignment validation is also executable. `Effects.validateRows` traverses the complete duplicate-free query schedule, rejects missing destinations and repeated destination values, and returns evidence from which `CheckedAssignment.destination_injective` follows. `Effects.invalid_destination_atomic`, `Effects.collision_atomic`, and `Effects.validation_rejection_atomic` prove that every such validation failure returns the exact input world.

## Trust and scope

The theorem boundary is a proof-carrying semantic plan. `OrdinaryUpdate` supplies local and global preservation laws plus a conservative footprint frame law; `Schema` supplies its global invariant and spawn-preservation law. The proofs establish that every accepted value at this boundary preserves the same registered schema and that every validation refusal is atomic.

Physical equal cardinality supplies only `Nonempty (Iso A B)`. It does not choose a semantic alignment. The guarded checks in `Ano.NegativeWitness` verify that foreign domains cannot combine pointwise or scatter through identity, while the positive witness uses an independently declared semantic map rather than deriving meaning from layouts.

The commutative merge theorem applies only when the supplied operation satisfies the exact associative, commutative, and identity equations. IEEE floating-point addition does not satisfy those equations and therefore cannot instantiate that theorem honestly.

This milestone does not yet prove that Steel constructs every certificate correctly, that CBQN or another dense-array lowering implements the semantic operations, or that save/load preserves metadata. It models fixed total fields and entity count, not partial ECS components, relationship repair, despawn, lattice rank and shape, placement, boundaries, or surface complexes; those remain later proof layers in `foundations.md`.

The source contains no `sorry`, `admit`, custom axiom, or unsafe declaration. The expected-failure witnesses are non-declaring guarded checks, so they add no theorem or recovered declaration to the environment.
