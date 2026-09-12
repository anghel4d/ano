# Steel's world representation

This page describes the Rust code in [steel/src/lib.rs](../steel/src/lib.rs), [registry.rs](../steel/src/registry.rs), and [emit.rs](../steel/src/emit.rs). Proposed nominal spatial storage belongs to [task 99](../todo/99-spatial-lattice.md).

## Registry

`Registry` contains the entity count, one legacy lattice width and height, an ordered vector of entries, spelling aliases, roles, and the reap policy. It is not an archetype/chunk allocator.

| Entry | Current representation |
|---|---|
| `Col` | Numeric or string vectors, a carrier, optional presence mask, optional range, and a uniqueness flag. |
| `Field` | A vector sized for the singleton lattice, with a carrier and optional range. |
| `Rel` | Numeric target keys and an optional unique-key column. |
| `SRel` | `Vec<Vec<f64>>` fibers, with optional inverse and key-column names. |
| `AliasMask` | Stored numeric mask. |
| `Bind` | One declared binding kind and its numeric payload. |
| `Tag` | A comparison against a declared column. |
| `Proto` | Named column/value pairs used by spawn. |
| `Fn` / `TypedFn` | Legacy body or a body with an explicit typed descriptor. |
| `Array`, `Service`, `Enum`, `Ctor` | Typed declarations with stable IDs and local versions. |

Set-valued relationships are nested vectors here, not a shipped CSR store. Entity keys and many numeric carriers use `f64`; the conceptual distinction between keys and indices does not imply a separate native key representation.

[ano-registry.md](ano-registry.md) owns declaration syntax and runtime sidecars.

## Compilation and execution

Steel lexes source, parses an AST, resolves registry names and operation descriptors, and emits BQN text. The emitter tracks value carriers, validity guards, source domains, and row witnesses. Its current domain distinctions are not the proposed general nominal habitat system.

A statement selects from pre-state, stages effect values, checks compatibility, and commits its writes. The next statement observes the result. Set-hop images become masks, so reaching a target twice does not perform an effect twice.

Functional hops resolve through the declared key column or legacy row indices. `-1` is the silent no-link sentinel. Invalid endpoint carriers refuse; an admissible but missing target fails foundness. See [relationship.rs](../steel/src/relationship.rs).

Spawn extends the entity columns and uses prototype values, defaults, then carrier zero. Unique-key columns receive fresh values. Despawn and relationship repair are implemented by the emitted program. There is no separately shipped generational archetype allocator behind this Rust registry.

## Host state and persistence

Dynamic aliases live in `.reg.aliases`. Declaration identity and schema version live in `.reg.schema`; migration events live in `.reg.migrations`. Resident arrays and constructed values have separate validated sidecars.

Kore stages ordinary registry writes and publishes schema migrations through a rollback journal. External runtime handles must be resealed through migration receipts. Details are in [kore/kore.md](../kore/kore.md) and [ano-registry.md](ano-registry.md).

## Spatial limit

The registry has one `lattice w h`; fields use its product width. The parser admits one- and two-dimensional shapes. Legacy point/vector conventions do not provide nominal frames, arbitrary-rank habitats, or typed localization.

The built-in neighbor path uses a clamp helper. Spelling a different boundary does not establish a different implemented policy. Explicit `srel` fibers describe their actual edges, but quarantined demos are not acceptance evidence.

The required replacement and its unresolved implementation choice live only in [task 99](../todo/99-spatial-lattice.md).
