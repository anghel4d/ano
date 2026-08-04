# 05 — registry border and runtime follow-ups

Status: the pre-spatial migration framework and Kore palette are implemented. The general registry taxonomy remains deliberately author-gated; the spatial placement seam and all spatial declaration design remain owned by `99`.

## The registry border

The registry language constructs a validated schema `Σ`. Ano statements execute inside that schema:

```text
Ano_Σ : World Σ → Result (World Σ × Output)
```

The two languages share resolved declaration identities and denotations, not grammar.

- Registry declarations never become Ano statement syntax.
- The comma remains the statement's predicate/effect border; it is not a schema-construction operator.
- Steel keeps separate registry and statement ASTs and joins them only during elaboration against a validated registry.
- Dynamic aliases from `02` are session overlay state, not declarations in `Σ`.

## Mutation taxonomy

Under a fixed `Σ`, the lawful mutations are ruled to be exactly three:

1. value updates permitted by existing column/effect declarations;
2. structural population changes such as spawn/despawn that preserve schema identity;
3. placement and transform values whose declared types stay fixed—a planet moves without touching the schema.

The first two are established in code and proof. The third is ruled but not yet reconciled with the spatial kernel: the current Lean registry keeps frame/offset maps outside mutable world state, so `99` owes the extension—declared-mutable placement slots behind an explicit permission, per-tick transport laws, cache/token invalidation, and matching proofs—before any document calls placement writes implemented. Which placements take the mutable declaration, whether sealed lineage and interpolation tokens survive a placement write, and whether compiled plans dereference placement values at tick time remain open implementation sub-questions in `99`.

Schema replacement is a fourth, distinct transition:

```text
World Σ  ⇝  World Σ′
```

It is a host operation at a world barrier, never an Ano right-of-comma effect.

## Atomic `Σ → Σ′` migration protocol

The implemented host spelling is `kore migrate live.reg candidate.reg migration.map`; the protocol is:

1. Parse the candidate registry independently.
2. Validate it fully and mint a new schema identity/version.
3. Build an explicit declaration migration map from stable old IDs to compatible new IDs, conversions, additions, removals, or refusals.
4. Validate and stage all conversions for declarations that exist before the spatial task: column carriers, component presence, relationship endpoints, persisted handles, and session overlays that target registry entries. Expose a typed migration-extension boundary; `99` adds habitat, frame, placement, lattice, and spatial-service conversions only after those descriptors exist.
5. Invalidate or revalidate compiled plans, registry-resolved callables, dynamic-alias binding targets, view descriptors, and service/token caches affected by the transition.
6. Recompile or lazily mark plans for recompilation against `Σ′`; no plan may retain an unchecked `Σ` handle.
7. Atomically publish `(Σ′, World Σ′)` at a world/statement barrier.
8. On any failure, retain the exact old `(Σ, World Σ)`, undo head, session alias environment, and log boundary.

The migration mechanism lives in Steel/Kore host commands, flags, or APIs. It does not add registry verbs to `.ano`.

### Migration acceptance

[X] DONE — Tests cover compatible rename/ID preservation, explicitly supported carrier widening, live-data removal, relationship endpoint changes, callable incompatibility and stale handles, migrated and removed alias targets, complete cache invalidation, failed conversion and partial-publication rollback, crash recovery, save/reload, and deterministic replay. Spatial endpoint and service cases remain assigned to `99` through the same extension boundary.

## Registry taxonomy, gated by author approval

Finish the general registry taxonomy only after its surface is explicitly approved. The loader needs semantic descriptors for:

- the canonical registry extension (`.reg` versus `.anoreg`) and any migration period;
- registry-resident arrays and their carrier/domain metadata;
- callable signatures, input/output carriers, effects, determinism, and resource/footprint declarations;
- registered input/output services and their trust/version boundaries;
- declared enum value sets and stable serialized identities;
- checked constructors that validate refinements rather than exposing raw carrier fabrication;
- spelling aliases as immutable name translations;
- static `AliasMask` entries as stored registry values.

Every item must specify validation, stable identity/versioning, persistence, diagnostic spelling, and plan-cache invalidation. “Callable” is not an unchecked host function pointer; admission requires a closed signature and the trust/effect metadata consumed by the planner.

The exact adjective/noun syntax proposed by older spatial notes is not approved here. Spatial nouns, frames, habitats, placements, locators, interpolators, and service certificates remain behind the author gate in `99`. Do not parse them as inert metadata: a declaration that grants no checking must not be advertised as a semantic capability.

## Documentation law

- `docs/ano-language.md` records the fixed-schema execution model and barrier-level schema replacement.
- Registry documentation owns declaration syntax and the taxonomy above.
- Any future command spelling remains labeled open until implemented.
- No document claims a surface is supported merely because the loader can retain unknown text or unchecked metadata.
- Every cache or serialized handle says which schema/version it belongs to.

## Kore palette follow-up

Derive Kore's palette from the shell theme without changing language semantics:

1. [X] DONE — Query terminal OSC 10 and OSC 11 at startup with a bounded, nonblocking response path.
2. [X] DONE — Parse and validate supported color response forms; ignore malformed or unsolicited replies.
3. [X] DONE — Derive readable accents and contrast from the reported foreground/background.
4. [X] DONE — Fall back deterministically to the existing forced dark canvas when the terminal does not answer, multiplexers filter the reply, or contrast is inadequate.
5. [X] DONE — Restore terminal state on every exit/refusal path and add pseudo-terminal tests for reply, timeout/no reply, malformed reply, and fallback.

This item may land before `99`. Spatial view behavior itself remains in `99`.

## Completion gate

- [X] DONE — Registry and Ano syntax remain separate.
- [X] DONE — Every ordinary mutation is classified under one of the three ruled fixed-`Σ` arms, and no document claims placement writes are implemented before `99` lands and verifies them.
- [X] DONE — The `Σ → Σ′` framework is validated, cache-safe, atomic, replayable, and rollback-safe for the pre-spatial schema, with a typed extension boundary that `99` must use rather than bypass.
- AUTHOR GATE — The registry taxonomy still needs explicit approval of exact syntax before typed validation can be implemented; no inert metadata has been added.
- [X] DONE — Kore theme probing is bounded and has a deterministic fallback.
