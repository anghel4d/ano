# Lean semantic kernel

Run from the repository root:

```sh
cd proofs
lake build
```

The pinned toolchain is in [lean-toolchain](lean-toolchain). [Ano.lean](Ano.lean) imports the semantic modules and guarded negative witnesses. The library uses Lean's standard library.

## Source map

| Modules in Ano/ | Contract |
|---|---|
| Field | Finite layouts, encoding, reindexing, selection, scatter |
| Effects, Assignment | Effect composition and certified assignment |
| Spawn, Allocation, World | Allocation and world updates under stated invariants |
| Validation, Negative | Validation and rejected constructions |
| Space, SpatialRegistry | Nominal spatial domains and registry structures |
| Affine, Interpolation | Affine frames and weighted support |
| Lineage, SpatialQuery | Query provenance and spatial alignment |
| SpatialPlan, SpatialSpawn | Accepted plans and exact-cardinality spawn |
| SpatialWorld, ColumnBundle | Partial live positions and aligned inputs |
| SpatialNegative, SpatialAdvancedNegative | Rejection witnesses at the modeled boundary |

## Trust boundary

A successful build checks the imported theorem statements and proofs. Read the hypotheses before applying a theorem. A supplied lawful merge, layout, locator, or accepted plan is an assumption at that boundary, not evidence that Steel produced it.

The build is not an axiom inventory. Inspect a theorem with Lean's `#print axioms` when its dependencies matter; report that output separately from build success.

The kernel does not establish compiler refinement, concrete geometry services, native code correctness, save/load agreement for a future spatial schema, or editor correctness. Those bridges remain in [foundations.md](foundations.md) and [task 99](../todo/99-spatial-lattice.md).

Negative witnesses check rejection by their modeled constructors. They do not replace malformed-input tests through the Rust loader, planner, emitter, and Kore.

## Reading a result

`Ano.Field.reindex_id` states that gathering a field along the identity map leaves it unchanged. `Ano.Field.reindex_comp` states the composition law for two maps. These laws concern the supplied maps; they do not prove that a parsed relationship chose the right map.

`Ano.Layout.decode_encode` and `Ano.Layout.encode_decode` use a layout with inverse laws. A backend needs to establish that its actual storage layout satisfies those laws before using the result as a correctness argument.

In the spatial kernel, `WeightedSupport` already requires nonempty support, unique cells, and algebraic normalization. A theorem that extracts its normalized weight sum is useful at later proof boundaries, but does not validate a raw host callback. Constructing that support safely is part of the implementation obligation.

## Reviewing a proof change

Read the changed statement, its hypotheses, and the constructors that can supply those hypotheses. Check that the umbrella still imports the module and run `lake build`. When claiming an axiom boundary, inspect the particular theorem's `#print axioms` output; a successful library build alone does not make that claim.

Then identify the consuming implementation boundary. If no Rust loader or planner constructs the required evidence yet, record the result as a model theorem with that bridge pending. Keep runtime acceptance and rejection examples beside the implementation they exercise.
