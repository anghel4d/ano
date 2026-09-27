# Foundations and implementation obligations

The authority order is mathematics, denotation, domain-and-lineage IR, grammar, surface, then lowering. An implementation shortcut cannot decide an open semantic question.

## Checked model

The [Lean module map](lean.md) identifies the checked laws. The model covers finite layouts, reindexing, effect composition, assignment, allocation, nominal spatial structures, lineage, and conditional accepted-plan execution.

The [spatial account](../docs/spatialmaths.md) defines the common terms. Keep mathematical definitions there and executable syntax in [the language reference](../docs/ano-language.md).

## Bridges still required

- Construct nominal domain, frame, lineage, and destination evidence from validated registry declarations.
- Connect Steel's accepted operations to the corresponding denotation, including empty results, refusal, and publication.
- Check any extensible rewrite law before granting optimization authority; trust metadata and property tests are insufficient.
- Bind capabilities to declaration identity, version, signature, schema, and the proposition/checker that authorized them; invalidate stale capabilities.
- Prove or test concrete geometry and support services against their stated contracts.
- Preserve schema identity and world invariants across save/load, migration, subsequent ticks, and Kore edits.
- Establish dense-lowering agreement without treating buffer length or layout coincidence as semantic identity.

These are obligations, not implemented facilities. Existing typed registry validation and migration receipts provide part of the infrastructure, not a general spatial certificate checker.

## Reduction and failure

Steel currently uses exact left accumulation. An associative model does not justify regrouping an operation whose actual carrier lacks that law. Count and average use state with a projection; they are not homogeneous operations on their output carrier.

The unseeded fold's empty behavior is documented in sections 12–14 of the language reference. Explicit seeds and fallible traversal remain [open decisions](../todo/06-seeded-fold-and-traverse.md).

## Evidence

Use Lean for the stated abstract laws, Rust tests for implemented contracts, and active Ano programs for end-to-end behavior. A successful BQN example, finite tick run, or benchmark cannot establish the missing refinement.

## A bridge needs both directions

A denotation assigns meaning to an accepted operation. The implementation bridge must show that lowering preserves that meaning and that malformed inputs cannot bypass the acceptance boundary. A successful output on one example establishes neither statement by itself.

For selection, retain the inclusion from selected rows to source rows. For a relationship, retain the resolved endpoint and foundness. For replication, distinguish copy occurrences from source entities. For publication, establish valid destinations and the required collision law. These are the facts a dense buffer representation otherwise hides.

## Rewrite authority

A future reusable rewrite capability must identify the law and its version, exact typed signature and endpoints, schema fingerprint, normalized proposition, checker identity/version, and proof payload. Registration checks that evidence before issuing the capability. A change in any dependency invalidates its reuse.

Trusted host declarations can describe callable behavior at the host boundary. Property tests can find counterexamples. Neither grants a general optimizer permission to reassociate or reorder an operation without the required checked law.

The existing registry contracts and migration receipts validate their implemented declarations and runtime values. They must not be described as the future general rewrite checker. [Task 99](../todo/99-spatial-lattice.md) gives the concrete spatial construction and refusal requirements.

## Outstanding mathematical scope

The abstract spatial model does not settle concrete floating-point geometry, phyllotaxis distinctness/separation, raycast completeness, metrics, boundary totality, or placement mutation. A service advertising one of those properties needs the corresponding definition and validation or proof.

Dense gather, segmented interpolation, scatter/merge, allocation, and relayout also need agreement with the accepted model. Save/load and repeated ticks must reconstruct the same authority and observations. Keep these obligations visible when adding backend implementations so a proof of an abstract structure is not mistaken for a proof of the whole runtime.
