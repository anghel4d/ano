# Registry

`.reg` is the text registry format. [registry.rs](../steel/src/registry.rs) loads, validates, and dumps it; [lib.rs](../steel/src/lib.rs) defines its types. Ano source cannot replace its own schema.

## Legacy rows

| Row | Meaning |
|---|---|
| `n N` | Entity count |
| `lattice W H` | One legacy lattice |
| `col NAME TYPE VALUES` | Entity column |
| `field NAME TYPE VALUES` | Lattice field |
| `unique NAME [TYPE] VALUES` | Total injective numeric key column |
| `pres NAME MASK` | Component presence |
| `default NAME VALUE` | Spawn fallback |
| `range NAME LO HI` | Numeric refinement |
| `rel [KEY] NAME TARGETS` | Functional relationship |
| `srel [KEY] NAME FIBERS` | Set-valued relationship |
| `inv NAME REL` | Inverse relationship |
| `bind NAME KIND VALUES` | Entity, mask, point, number, or vector binding |
| `alias NAME MASK` | Static stored mask |
| `as`, `ja` | Spelling alias; the longer `as` form declares a derived tag |
| `role ROLE NAME` | System-role mapping |
| `def NAME COL=VALUE ...` | Spawn prototype |
| `fn NAME BODY` | Legacy callable |
| `reap seal\|host` | Reclamation policy |

Declaration order matters where a row refers to an earlier entry. Headers cannot be redeclared. A unique column cannot be partially present, default-filled, or directly assigned.

Entity bindings hold stable keys resolved against the current key column. A point or vector binding is a distinct declared face; context does not reinterpret a binding as another kind.

## Typed declarations

```text
array NAME id:HHHHHHHHHHHHHHHH v:N CARRIER scalar|entity|fixed:N
service NAME id:HHHHHHHHHHHHHHHH v:N input sig:ARGS->RESULT trust:checked|trusted
service NAME id:HHHHHHHHHHHHHHHH v:N output sig:ARGS->unit trust:checked|trusted
enum NAME id:HHHHHHHHHHHHHHHH v:N CASE=U32 ... reserve:U32,...
ctor NAME id:HHHHHHHHHHHHHHHH v:N range:LO..HI
ctor NAME id:HHHHHHHHHHHHHHHH v:N enum:ENUM
fn NAME id:HHHHHHHHHHHHHHHH v:N sig:ARGS->RESULT fx:EFFECTS det:BOUNDARY trust:trusted read:NAMES write:NAMES use:SERVICES = {BQN}
```

The alternatives above describe the grammar; the vertical bars are not literal declaration syntax. IDs are nonzero sixteen-digit hexadecimal values. Versions are positive integers. References in typed descriptors use earlier canonical names.

Primitive types are `mask`, `nat`, `int`, `num`, `sym`, `char`, and `entity`. Earlier enums and constructors supply nominal carriers. Signatures use `unit->T` for no arguments and `A,B->T` for two. `unit` is an allowed result, not an ordinary stored carrier.

The schema fingerprint covers declarations and their metadata, not changing entity population or row values. An explicit declaration ID remains stable across a supported rename; its local version records semantic changes.

## Resident values

An `array` row declares a slot, not its payload. `scalar` requires one value, `entity` requires the current entity count, and `fixed:N` requires N values. `seal_resident_array` validates every attachment against its descriptor.

Constructed nominal values pass through `construct`. Range constructors check finite inclusive bounds; enum constructors admit declared live discriminants. Equal physical representation does not create a nominal value.

Handles and constructed values carry the schema fingerprint, declaration ID/version, and canonical name. Validation rechecks the payload; a source reference to an unattached array refuses.

Canonical sidecars use these headers and tab-separated fields:

```text
ano-resident-array-v1  SCHEMA DECL VERSION NAME KIND PAYLOAD...
ano-constructed-value-v1  SCHEMA DECL VERSION NAME number F64BITS
ano-constructed-value-v1  SCHEMA DECL VERSION NAME enum ENUMDECL U32
```

Fields are separated by tabs and the record ends in LF. Numeric payloads preserve admitted bits after zero canonicalization. Symbol payloads use UTF-8 hexadecimal; char payloads use Unicode scalar encodings. Decode revalidates against the supplied registry.

## Callables

Typed callables declare their signature, effect set, determinism, trust, read/write footprints, and service use. Empty footprints are `-`. `fx:pure` has no effects; otherwise the corresponding `read`, `write`, and `service` bits must agree with the named footprints.

`det:deterministic` has no service use. `det:snapshot` may use frozen input services. `det:nondeterministic` requires an output service. Raw BQN bodies are admitted under `trust:trusted`; this is not a machine-checked purity proof.

Value calls require exact argument carriers, a non-unit result, and the supported unary/binary value ABI. Effect calls require unit and the supported write/service shape. Typed reducers are pure deterministic homogeneous `A,A->A` functions with exactly matching operands. Typed pipelines remain refused without a domain signature.

A legacy callable does not acquire typed guarantees merely by appearing beside typed entries.

## Services and enums

Services declare input/output direction and a signature, not a registry body. Deictic aliases can bind the built-in resolver implementation to a matching declared input service. Its identity and version then participate in alias validation; undeclared legacy-v1 resolvers remain a separate contract.

Enum discriminants are stable identities. Removing a live case requires reserving its discriminant; a reserved value cannot become live again.

## Aliases and migration

Spelling aliases are schema. Static alias masks are registry data. Dynamic `^name` aliases are separate session state in `.reg.aliases`; they do not overwrite bare declarations.

`kore migrate live.reg candidate.reg migration.map` requires an explicit declaration map. Supported operations include preserve, rename, legacy widening, drop/discard, add, and unalias. Legacy widening is limited to `bool -> nat|int|num`, `nat -> int|num`, and `int -> num`.

A typed semantic change must retain its stable ID and increase its local version. Old plans and runtime values must cross the migration receipt and revalidate under the new schema.

Kore journal-publishes the registry, alias environment, schema manifest, and migration event log. External resident values are resealed through the receipt before their host publishes them. [kore/kore.md](../kore/kore.md) describes recovery. Spatial schema conversion remains unimplemented.

## Reading a declaration

In `col gold num 100 200 300`, `gold` is the declaration name, `num` is its carrier, and the three values correspond to the registry's three entity rows. The script may resolve the name as `Gold`; this case handling does not change symbol payloads.

A presence mask answers whether a component exists on a row. Its stored numeric value answers what that component contains. Removing a component is therefore different from assigning zero. Likewise, a functional relationship's stored key is different from the foundness mask obtained by resolving that key.

For a concrete keyed world, see [138-typed-keys.reg](../demos/registries/138-typed-keys.reg). For typed declarations and their refusal/evolution cases, see [registry_contracts.rs](../steel/tests/registry_contracts.rs). These examples have the surrounding declarations required to interpret their IDs and signatures.

## Trust and replacement

A typed signature controls which carriers a call accepts. Footprints identify the state it may read or write. A determinism declaration controls the permitted service boundary. These checks serve different purposes: matching a signature does not prove that an arbitrary BQN body tells the truth about its effects.

A migration must account for both declarations and live data. Preserving an ID across a rename preserves identity; changing its meaning requires an appropriate version change and revalidation. A stale cached plan or resident value cannot be made current by relabeling its schema fingerprint.

The file transaction covers the registry and its managed sidecars. A host holding external resident arrays or constructed values must separately use the migration receipt before publishing those values. This distinction matters when a file migration succeeds but a host still holds an old handle.
