# TODO

Live items only. `00-historical-rulings.md` is the definitive decision ledger; derive consequences from its answers instead of reopening them. Implement the semantic tasks in this order; do not begin a later task by silently baking in an unfinished earlier one.

1. **Preserve sigil semantics through resolution.** `01-sigil-semantics.md` is complete: `!` remains mask negation, `^name` remains distinct from bare `name` through the resolved IR, and spelling aliases, static alias masks, and dynamic aliases retain separate lifetimes.
2. **Implement the live dynamic alias overlay.** `02-dynamic-alias-overlay.md` is complete for the non-spatial runtime: overlay-first `^name` lookup with bare fallback, binding/mask/service-v1 deictic targets, cold `^cursor`, statement-frozen world/overlay/host input, host lifecycle, deterministic replay, migration revalidation, Kore display, and rewritten demos `005`, `019`, `028`, and `103`. Alias demos `071` and `093` remain quarantined only because they also require `99`.
3. **Unify folds and scans.** `03-folds-and-scans.md` is complete for every non-spatial path: A13's left recurrence, state-plus-projection mean/count, carrier-directed `|`/`&`, operator-specific mixed-carrier promotion, strict destinations, extended-real `num` with NaN-sealed publication, converged glyph/long/grouped/along forms, `min\`/`avg\`/`#\`, no `scan2`, typed descriptor refusals, schema-bound operation handles, and zero-row identityless empties. Demo `033` remains quarantined only because it also requires `99`.
4. **Repair diagnostic domains and seal relationship targets.** `04-diagnostic-domains.md` is complete for every non-spatial introduction path, including migration: predicate reports range over `X`, effect reports over `S`, distinct crossings retain stable use IDs, `-1` is the sole silent functional no-link sentinel, malformed endpoint-carrier values refuse atomically, and Steel/Kore preserve the exact machine trace without changing outputs or post-state.
5. **Finish non-spatial registry/runtime follow-ups.** `05-registry-and-runtime-followups.md` is complete: `.reg` remains canonical; explicit-ID/version resident arrays, typed callables, services, enums, and checked constructors cross one text/programmatic validator; planning consumes signatures, effects, determinism, trust, footprints, and exact reducer carriers; schema-stamped runtime sidecars revalidate and migrate through receipts; resolver services carry declaration versions; exact refusal and evolution suites pass; the atomic journaled `Σ → Σ′` framework, task-`99` extension boundary, and bounded OSC 10/11 palette remain intact.
6. **Decide the seeded-fold and traverse surfaces.** `06-seeded-fold-and-traverse.md` parks the two gaps `03` left explicit: a fold with an explicit seed, and a fallible `traverse`. Neither is ruled. Do not begin them by extending the unseeded recurrence, and do not begin `99` by baking either in.
7. **Verify and implement the spatial lattice last.** `99-spatial-lattice.md` replaces all overlapping spatial notes. It begins from the actual singleton-2-D Rust boundary and the narrower Lean trust boundary, then requires nominal habitats, declared lattice/axis/boundary capabilities, typed domains and lineage, positions/placements/services, spatial extension of the migration framework, exact-51 atomic spawn, persistence, dense lowering, Kore view/edit repair, proof verification, total demo rewrites, and only then the checked native-kernel measurement.

## Demo evidence quarantine

Exactly 56 demo numbers are decommissioned until rewritten from first principles:

```text
033–037, 044, 046–047, 050–053, 055–078,
089–091, 093, 099, 101–102, 104–110, 114–119
```

Primary ownership avoids double counting:

| task | numbers | count |
|---|---|---:|
| `02` dynamic aliases | `071`, `093` | 2 |
| `03` folds/scans | `033` | 1 |
| `99` spatial lattice | `034–037`, `044`, `046–047`, `050–053`, `055–070`, `072–078`, `089–091`, `099`, `101–102`, `104–110`, `114–119` | 53 |
| **total** |  | **56** |

A number is the unit of quarantine: Ano, Nihongo, BQN, registry, expected output, README/test claims, and manifest entries leave or return together. Cross-task dependencies remain explicit: `028` also needs `03`; `033` also needs `99`; `071` and `093` also need `99`; diagnostic-domain obligations are especially relevant to `105–109` and `114–115` but add no new demolition number.

Do not expand the list merely because a still-correct demo uses `&`, `|`, a relationship, a fold, or a scan. Controls `011`, `012`, `038`, and `039` remain valid mask fold/scan witnesses; `040` remains a valid registered-reducer spelling witness; `008`, `015`, `026`, `027`, and `087` remain valid foundness/dead-link result witnesses.

## Landing rule

A task is complete only when code, Ano/Nihongo grammar/docs, Steel behavior, Kore behavior where applicable, exact refusals, persistence/replay implications, and active demonstrations all agree. No backend output, raw buffer coincidence, stale status line, or unchecked metadata counts as semantic evidence.
