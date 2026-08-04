# Ano registry

The registry is Ano's typed host boundary. `.reg` remains the canonical extension; there is no parallel `.anoreg` dialect or migration period. Legacy world rows and high-integrity declarations coexist in one ordered text grammar, while Ano source remains a separate language elaborated against the validated registry.

## Boundary and order

A registry constructs one schema `Σ`. Ano statements may update values and population under that fixed schema, but they cannot declare, replace, or migrate schema. `kore migrate` is the separate barrier-level host operation for `Σ → Σ′`.

Declaration order is semantic. A named carrier, footprint, constructor enum, alias target, relationship endpoint, prototype field, or other declaration reference must name an earlier entry and use its canonical spelling. Registry lookup remains ASCII case-insensitive for source use; high-integrity stored references are canonical so dump and reload cannot silently retarget.

The new declarations are deliberately narrow:

| declaration | canonical form |
|---|---|
| resident array slot | `array NAME id:HHHHHHHHHHHHHHHH v:N CARRIER scalar` |
| entity-width array slot | `array NAME id:HHHHHHHHHHHHHHHH v:N CARRIER entity` |
| fixed-width array slot | `array NAME id:HHHHHHHHHHHHHHHH v:N CARRIER fixed:N` |
| input service | `service NAME id:HHHHHHHHHHHHHHHH v:N input sig:ARGS->RESULT trust:BOUNDARY` |
| output service | `service NAME id:HHHHHHHHHHHHHHHH v:N output sig:ARGS->unit trust:BOUNDARY` |
| enum | `enum NAME id:HHHHHHHHHHHHHHHH v:N CASE=U32 ... reserve:U32,...` |
| finite range constructor | `ctor NAME id:HHHHHHHHHHHHHHHH v:N range:LO..HI` |
| enum constructor | `ctor NAME id:HHHHHHHHHHHHHHHH v:N enum:ENUM` |
| typed raw callable | `fn NAME id:HHHHHHHHHHHHHHHH v:N sig:ARGS->RESULT fx:EFFECTS det:BOUNDARY trust:trusted read:NAMES write:NAMES use:SERVICES = {BQN}` |

`HHHHHHHHHHHHHHHH` is exactly sixteen hexadecimal digits and cannot be zero. `N` is positive. The canonical dumper emits lowercase IDs, normalized numbers, canonical referenced names, footprint sets in declaration order, sorted enum reservations, and the fixed field order above.

The declaration ID is nominal identity across spelling changes. The declaration version is local semantic evolution. Schema identity is the structural fingerprint of the complete registry: every high-integrity ID, version, descriptor, callable body, legacy declaration descriptor, spelling alias, role, and reap policy participates, while live row values and entity population do not. The length of a legacy numeric-pair payload is live compatibility representation, not a declared capability, and is therefore excluded; an explicit `array` domain is the typed schema route. Explicit high-integrity IDs reserve the manifest namespace before deterministic IDs are derived for legacy entries, so declaration order cannot create an identity collision.

## Registry types

The closed primitive types are `mask`, `nat`, `int`, `num`, `sym`, `char`, and `entity`. An earlier `enum` or `ctor` name is a nominal type. `unit` is admitted only as the signature result or as the spelling for an empty input list. These exact lowercase primitive tokens cannot themselves name an enum or constructor, because canonical dump and reload must preserve whether a type is primitive or nominal.

A signature is `sig:unit->T` for no explicit arguments or `sig:A,B,...->T` otherwise. Registry signatures are exact: ordinary Ano arithmetic may promote within its operator family, but typed callable arguments and typed reducer operands must match the declared registry type. Equal physical representation grants neither a conversion nor a nominal value.

## Resident arrays

An `array` line declares a slot, never its payload. `scalar` seals exactly one value, `entity` seals exactly the current entity count, and `fixed:N` seals exactly `N` values. The carrier chooses the value family and every attachment crosses `seal_resident_array`:

- `mask`, `nat`, `int`, and `num` accept numeric arrays only and recheck their carrier.
- `sym` accepts every UTF-8 string, including empty and boundary-bearing values; hexadecimal sidecar framing makes this total without changing legacy `.reg` rows.
- `char` accepts Unicode scalar values.
- `entity` accepts only live keys that resolve uniquely in the current registry.
- An enum accepts case names or discriminants and stores canonical live discriminants.
- A range constructor accepts numbers and rechecks its finite closed interval; an enum constructor reuses its enum's live discriminants.

A `ResidentArrayHandle` carries the full schema fingerprint, stable declaration ID, declaration version, canonical name, and normalized payload. Validation checks all five dimensions and reseals every element. A source-level reference to an unattached array refuses; registry metadata never fabricates a payload.

Resident arrays persist through a canonical one-line sidecar:

`ano-resident-array-v1<TAB>SCHEMA<TAB>DECL<TAB>VERSION<TAB>NAME<TAB>KIND<TAB>PAYLOAD...<LF>`

`SCHEMA` and `DECL` are sixteen hexadecimal digits. Numeric and entity payloads canonicalize both IEEE signed zeros to Ano `+0`; every other admitted bit pattern is exact, and a sidecar carrying `-0` is noncanonical and refuses. Symbols use UTF-8 hexadecimal, characters use eight-digit Unicode scalar values, and enum discriminants use eight-digit hexadecimal. `save` validates before atomic publication; `load` rejects malformed framing and revalidates against the supplied registry. The format has no unchecked or best-effort load mode.

## Callables and footprints

A typed callable owns a closed signature, effect row, determinism boundary, trust boundary, three explicit footprints, and one raw BQN dfn. The effect row is `pure` or a comma-separated subset of `read,write,service`. `read:-`, `write:-`, and `use:-` spell empty footprints. Each nonempty footprint must name earlier canonical declarations of the appropriate capability, contain no duplicates, normalize to declaration order, and agree exactly with the corresponding `fx` bit.
The BQN backend admits no inert footprint promises. `read:` may name emitted columns, fields, relationships, static masks, and materialized numeric, mask, or vector bindings; `write:` may name only non-unique columns and fields. Resident arrays remain host attachments behind `seal_resident_array`, and relationships remain behind their checked command path until a lowering can carry those capabilities explicitly.


The determinism boundary is one of:

| boundary | law |
|---|---|
| `det:deterministic` | no service footprint |
| `det:snapshot` | may use input services frozen for the statement, never an output service |
| `det:nondeterministic` | must use at least one output service |

Raw BQN is admitted only under `trust:trusted`, must be one comment-free dfn on one registry line, and cannot claim a nominal result. Nominal results cross only a checked constructor.

Steel consumes the descriptor before lowering. Value calls require the exact argument types, a non-`unit` result, no write or output-service effect, and the current unary or binary value ABI. Effect calls require `unit`, at most one mutable column write target, or a declared output service when there is no write target. A typed reducer is exactly a pure deterministic homogeneous `A,A->A` callable, and its operand must be exactly `A`; declaration of a numeric refinement is not erased to the broad runtime number family. Typed pipeline calls refuse until a domain signature exists, rather than borrowing the legacy untyped pipeline ABI. These refusals keep descriptor fields from becoming decorative metadata.

A legacy `fn NAME [BQN]` remains loadable for old fixtures. It retains its historical untyped numeric-reducer and raw verb conventions, but it does not acquire a high-integrity identity or silently count as a typed capability.

## Services

A service is a versioned host boundary with no registry body. An input service must return a value; an output service must return `unit`. `trust:checked` says the host adapter enforces the signature, while `trust:trusted` admits an implementation outside Steel's checked core. Callable `use:` footprints name services explicitly.

The deictic resolver adapter is concrete today. If a built-in resolver ID resolves to an earlier service declaration, it must be an input `unit->entity` or `unit->mask` service matching the implementation's carrier. The dynamic alias stores that declaration's stable ID and version. Changing either, or removing the declaration even when its version was `1`, stales the alias and forces migration revalidation. Absence of a declaration preserves a distinct legacy built-in service-v1 contract; legacy-v1 and declared-v1 are never interchangeable.

## Enums and constructors

An enum has at least one live case. Case names are unique under registry name equality, live discriminants are unique, and reserved discriminants are strictly ascending in canonical text. A live discriminant cannot also be reserved.

A constructor is the sole nominal fabrication boundary. `range:LO..HI` uses finite ordered inclusive bounds. `enum:NAME` must name an earlier enum. `construct` returns a `ConstructedValue` stamped with schema, constructor ID, constructor version, canonical name, and either an exact numeric payload or an enum ID plus live discriminant. Validation repeats the refinement check; changing a public struct in memory cannot bypass it.

Constructed values use the same persistence law:

`ano-constructed-value-v1<TAB>SCHEMA<TAB>DECL<TAB>VERSION<TAB>NAME<TAB>number<TAB>F64BITS<LF>`

`ano-constructed-value-v1<TAB>SCHEMA<TAB>DECL<TAB>VERSION<TAB>NAME<TAB>enum<TAB>ENUMDECL<TAB>U32<LF>`

Decode is strict and always revalidates. A wrong constructor name, stale schema, wrong enum identity, retired discriminant, NaN, or newly out-of-range number refuses.

## Aliases and identity

The three alias mechanisms retain separate identities and lifetimes.

- A spelling alias (`as` or `ja` with two names) is immutable schema translation. Its source, target, and Japanese flag participate in the schema fingerprint.
- A static `alias` mask is legacy registry data. Its stable identity is the persisted manifest identity derived for that entry; it is not the dynamic overlay.
- A dynamic `^name` alias is session state in `.reg.aliases`. It carries registry schema, target identity, environment version, and any resolver service declaration identity and version. Its sidecar has one canonical LF-terminated UTF-8 byte form and is published by create-new staged atomic replacement.

None of these can mint a nominal enum or constructor value. A spelling alias never creates a second declaration identity, and deleting a dynamic alias never mutates a bare declaration.

## Migration and cache law

`ano-schema-v1` admits one LF-terminated UTF-8 form: canonical decimal versions, lowercase fixed-width fingerprints and nonzero declaration IDs, exact tab framing, and no carriage returns. The manifest, receipt, and validated migration outcome expose read-only data outside Steel; the task-`99` header hook receives immutable registries and can authorize a transition but cannot rewrite checked descriptors in safe Rust.

A high-integrity declaration crosses migration only under `preserve` or `rename` with the same stable ID; `widen` remains limited to the established legacy scalar carrier widenings. Version regression refuses. A changed callable body or descriptor, array carrier or domain, service signature/direction/trust, enum set, or constructor range must increase `v`. A spelling-only rename may retain the version because identity and denotation remain stable.

Enum reservations are permanent. Removing a live case requires reserving its old discriminant, and a reserved discriminant can never become live. Enum and constructor references follow the explicit stable declaration map rather than matching by spelling.

Every plan, declaration handle, resident array, constructed value, resolver alias, and service/cache token is schema-bound. The migration receipt is the only bridge. It proves the exact old and new structural fingerprints, maps the stable declaration, then reseals resident arrays and revalidates constructed values under `Σ′`. A narrower range, retired enum case, changed array extent, stale service identity or version, or incompatible target makes migration fail without publishing a partial world.

Task `99` extends this same declaration and receipt machinery with habitats, frames, placements, locators, interpolators, and spatial services. It may add new descriptors; it may not bypass the final registry validation gate or infer spatial authority from an array extent.
