# TODO

The [historical rulings](00-historical-rulings.md) preserve the author's decisions. Current behavior is described in [the language reference](../docs/ano-language.md).

## Open work

1. [06: seeded fold and fallible traverse](06-seeded-fold-and-traverse.md). Obtain a ruling before extending grammar or the unseeded recurrence.
2. [99: spatial lattice](99-spatial-lattice.md). Decide the implementation path and establish nominal domains, lineage, placement, persistence, and matching Kore behavior. Do not bake unresolved task-06 choices into it.

## Completed non-spatial work

[Sigils](01-sigil-semantics.md), [dynamic aliases](02-dynamic-alias-overlay.md), [folds/scans](03-folds-and-scans.md), [diagnostic domains](04-diagnostic-domains.md), and [registry/runtime contracts](05-registry-and-runtime-followups.md) are implemented. Their remaining quarantined cases depend on spatial work.

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

## Completion

Code, grammar, reference docs, Steel/Kore behavior, exact refusals, persistence/replay, and active demonstrations must agree. Abstract proofs and BQN examples do not establish compiler agreement by themselves.
