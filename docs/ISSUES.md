# Implementation limits

This list describes current code boundaries. Feature decisions belong in [todo/TODO.md](../todo/TODO.md).

- Spatial storage still has one unnamed 2-D lattice. General nominal habitats, frames, localization, interpolation, certified support projection, and mixed-habitat rule barriers are not implemented. Legacy neighbor lowering does not implement distinct boundary policies.
- Legacy numeric-pair shape can be inferred from payload width. Empty data cannot supply that evidence. It is not a nominal product carrier.
- Legacy whitespace registry rows cannot encode every symbol/glyph post-state. Saving an unrepresentable state refuses rather than losing data. Typed resident-array sidecars cover their own payloads; they do not replace the legacy world codec.
- Steel's file runner schedules rule steps around installation runs. It has no positional mission schedule. Temporal `ago`/`window` services and `.mission` manifests are absent.
- Seeded folds and fallible traversal have no surface. [Task 06](../todo/06-seeded-fold-and-traverse.md) preserves the open decision.
- Named reducers over fibers and per-fiber scans are refused. Ragged scan results have no representation.
- Typed pipeline calls are refused until a domain signature is supported. An array declaration alone does not attach a runtime payload.
- There is no implemented native Ano JIT or GPU backend.
- Bounded demo tick checks do not establish termination or successful allocation for an unbounded number of ticks.
