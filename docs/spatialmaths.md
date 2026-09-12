# Spatial mathematics

This is the model used by the Lean spatial kernel, not a description of the current Rust storage. [Task 99](../todo/99-spatial-lattice.md) tracks the bridge to Steel and Kore.

## Domains and columns

A column over a finite domain H is a function H → V. Equal cardinality does not identify two domains. Alignment requires an explicit map.

A layout is a bijection Fin(n) ≃ H. Encoding through that bijection changes storage order without changing the semantic column. The layout and reindexing laws live in [Field.lean](../proofs/Ano/Field.lean).

Reindexing along f : X → H turns c : H → V into c ∘ f : X → V. Selection, relationship traversal, and replication need their own maps; matching buffer lengths do not supply them.

## Effects

A query occurrence and its destination are distinct. Plain scatter needs an injective destination map. Colliding writes need the algebra required by the chosen merge.

A commutative monoid permits schedule-independent reduction. IEEE floating-point addition does not supply exact associativity, so that theorem does not authorize reassociation of Steel's numeric folds. Current folds use exact left accumulation.

## Space and placement

A frame identifies the space of points and displacements. Affine translation acts on points; points are not vectors or entity keys. A lattice embedding maps cell coordinates into a frame.

A locator relates points to cells. Interpolation carries support and weights. These are explicit structures with obligations in [Affine.lean](../proofs/Ano/Affine.lean), [Interpolation.lean](../proofs/Ano/Interpolation.lean), and [Lineage.lean](../proofs/Ano/Lineage.lean), not deductions from a pair of numbers.

Support choice must be deterministic in semantic keys. Storage enumeration is not a tie-breaker. A boundary policy must declare its behavior instead of silently clamping every request.

## Planning and publication

The spatial plan and spawn modules model checked destinations and exact-cardinality allocation. Their exact-51 result assumes an accepted plan with the required source, support, and destination evidence.

[SpatialWorld.lean](../proofs/Ano/SpatialWorld.lean) models partial live positions and preservation of untouched values. [ColumnBundle.lean](../proofs/Ano/ColumnBundle.lean) supplies aligned heterogeneous inputs.

Steel does not yet construct these general spatial certificates. Its singleton two-dimensional lattice and positional conventions are documented in [ano-ecs.md](ano-ecs.md). Geometry services, persistent nominal identity, dense lowering, and editor behavior still need implementation and agreement tests.

## Worked domain example

Suppose the entity domain E contains three actors, and a ground domain G contains three cells. Health : E → Num and Height : G → Num both encode as three numbers. Adding their buffers position by position would implicitly assert that actor 0 inhabits cell 0, actor 1 inhabits cell 1, and so on. Neither cardinality nor layout establishes that assertion.

A validated locator can instead supply f : E → G on the actors whose positions locate successfully. Height ∘ f then varies over those actor rows. If two actors occupy one cell, the gather is still well-defined: both read that cell's height.

Writing back reverses the difficulty. The same two actors may propose different values for one cell. A plain assignment needs unique destinations; otherwise the operation must declare a permitted merge. A valid gather does not automatically authorize its reverse scatter.

## Selection, order, and validity

For a predicate p : X → Bool, the selected domain S consists of the rows satisfying p, and its inclusion i : S → X records where each selected row came from. Reading c after selection means c ∘ i. Later assignment uses the corresponding destination map, not the ordinal position within the compacted buffer.

An ordered traversal adds an order without discarding this identity. Replication adds distinct copy occurrences even when several copies share one source. Product rows retain both component identities. These distinctions explain why an equally long result can still be invalid as a world-column assignment.

Partial operations also retain validity. A missing relationship or unsuccessful locator contributes no admitted value for that row. The empty result law then depends on the consumer: an identity-bearing fold has its identity; an identityless fold has no result row; an exact-cardinality spatial command refuses if any required row fails.

## What normalized interpolation proves

The Lean `WeightedSupport` structure contains a finite, nonempty, duplicate-free set of cells and coefficients whose sum is the declared unit weight. Its normalization theorem follows from that supplied structure.

Normalization alone does not establish nonnegative weights, geometric incidence, a nearest-cell policy, or a distance metric. The kernel makes nonnegativity a separate obligation. A concrete interpolator must validate the properties it advertises, including its source/frame/habitat endpoints and numerical behavior.

The accepted mathematical model may use exact algebraic laws. An implementation over float64 needs an explicit account of rounding and publication; it cannot substitute numerical proximity for an exact theorem without changing the contract.
