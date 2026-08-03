# TODO

Live items only. `00-historical-rulings.md` is the definitive decision ledger; derive consequences from its answers instead of reopening them. Implement the semantic tasks in this order; do not begin a later task by silently baking in an unfinished earlier one.

1. **Preserve sigil semantics through resolution.** `01-sigil-semantics.md` keeps `!` as mask negation, keeps `^name` distinct from bare `name` through the resolved IR, and separates spelling aliases, static alias masks, and dynamic aliases.
2. **Implement the live dynamic alias overlay.** `02-dynamic-alias-overlay.md` adds overlay-first `^name` lookup with bare fallback, validated binding/mask/deictic-resolver targets, host-side install/rebind/delete at statement barriers, one frozen world/overlay/host-input snapshot per statement, deterministic replay, and independent bare-binding lifetime. Landed: the overlay runtime, the resolver framework, the host boundary, persist-environment replay, the ruled `^cursor` cold default, Kore's `>alias` display, and rewritten demos `005`, `019`, `028`, and `103`. Remaining: the concrete deictic resolvers alongside `99`'s host services, and demos `071` and `093` behind `99`.
3. **Unify folds and scans.** `03-folds-and-scans.md` implements A13's unseeded left recurrence; implements running mean/count through state-plus-projection descriptors in the planner; implements q-style carrier-directed `|`/`&`; converges glyph/long/grouped/along forms; adds `min\`, `avg\`, and `#\`; deletes `scan2`; and makes identityless empty queries produce no row rather than a placeholder. Landed in full on the Steel and Kore sides with demos `029–032` rewritten; the coercion question is ruled — no implicit carrier coercion — and recorded in the charter; `033` remains quarantined behind `99`.
4. **Repair diagnostic domains and seal relationship targets.** `04-diagnostic-domains.md` keeps predicate reports on source domain `X`, restricts effect reports to selected domain `S`, preserves distinct crossings, makes `-1` the sole silent functional no-link sentinel, refuses malformed endpoint-carrier values atomically, and reports every evaluated valid dead link.
5. **Finish non-spatial registry/runtime follow-ups.** `05-registry-and-runtime-followups.md` supplies the atomic pre-spatial `Σ → Σ′` framework and typed extension boundary, gates the general registry taxonomy, keeps registry and Ano grammar separate, and derives Kore's palette safely from OSC 10/11.
6. **Verify and implement the spatial lattice last.** `99-spatial-lattice.md` replaces all overlapping spatial notes. It begins from the actual singleton-2-D Rust boundary and the narrower Lean trust boundary, then requires nominal habitats, declared lattice/axis/boundary capabilities, typed domains and lineage, positions/placements/services, spatial extension of the migration framework, exact-51 atomic spawn, persistence, dense lowering, Kore view/edit repair, proof verification, total demo rewrites, and only then the checked native-kernel measurement.

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

A number is the unit of quarantine: Ano, Nihongo, BQN, registry, expected output, README/test claims, and manifest entries leave or return together. Rewritten and returned on 2026-08-03: `005`, `019`, `028`, and `103` for `02`, and `029–032` for `03`. Cross-task dependencies remain explicit: `033` also needs `99`; `071` and `093` also need `99`; diagnostic-domain obligations are especially relevant to `105–109` and `114–115` but add no new demolition number.

Do not expand the list merely because a still-correct demo uses `&`, `|`, a relationship, a fold, or a scan. Controls `011`, `012`, `038`, and `039` remain valid mask fold/scan witnesses; `040` remains a valid registered-reducer spelling witness; `008`, `015`, `026`, `027`, and `087` remain valid foundness/dead-link result witnesses.

## Landing rule

A task is complete only when code, Ano/Nihongo grammar/docs, Steel behavior, Kore behavior where applicable, exact refusals, persistence/replay implications, and active demonstrations all agree. No backend output, raw buffer coincidence, stale status line, or unchecked metadata counts as semantic evidence.
