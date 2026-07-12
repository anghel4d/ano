# todo/

Task files from the 2026-07-11 demo-pass review. One work item per file, numbered in execution order, not the order they arose. Each file is self-contained for a dedicated agent: findings with code pointers, the author's rulings (binding), open sub-questions (surface at execution, never resolve silently), work items, and invariants that must hold after. 00 is the exception: a Q/A for the author holding only the unanswered decision points. When a file is executed, cross it off in TODO.md and move it to done/.

Executed (2026-07-11), now under done/: 01-10. The catalog below stands as record; still live here is 00 (the open rulings), 11-next-steps.md (the blocking residue that needs the author's hand), and 12-unbuilt-scans.md (surfaced 2026-07-13, not started).

- `00-open-rulings.md` — the deferred decision points, in Q/A form; the answered rulings from the 2026-07-11 pass are folded into their consuming task files. Consuming tasks read the A-lines first.
- `01-kore-query-outputs.md` — query results are computed, asserted, never shown. Make every query visible in kore. FIRST agent task, by ruling.
- `02-registry-types-and-hop-integrity.md` — the s10 bundle: unique/keyed registry types, the proto, fallback defaults, the despawn policy, the functional-hop fix. The deepest item and the first core-design advancement since the initial spec.
- `03-demo-corpus-normalization.md` — global ascending renumbering, the s05 split, the 08 faction→leader rename, the re-annotation pass.
- `04-fold-scan-formalization.md` — the fold/scan permutation table into the docs verbatim, new `&\` and `|\` demos, the max ruling.
- `05-reducer-surface-and-sigils.md` — `f/` over registered reducers, `fold()` as the long form, the `!` vs `^` tabulation and the `^bind` seam.
- `06-manual-srel-vec-chapter.md` — document set-valued relations and vec columns in ano-manual.md.
- `07-tui-editors.md` — the vim.tiny diagnosis, EDITOR=nvim at project and nix level, the internal editor.
- `08-world-map-and-bitmap.md` — square cells, inferred glyphs, unicode pieces, the half-block bitmap mode.
- `09-numeric-edges.md` — document the float64 model and close the inf load/save asymmetry so the seal is airtight.
- `10-debug-observability.md` — the dead-link diagnostic, the per-tick structural trace, the session log as the repro channel. Mission-critical by ruling.
- `12-unbuilt-scans.md` — the `min\` glyph, running mean, and running count: ruled by the fold/scan table but unbuilt in either tree. Reconcile the table with the emitter — build them, or footnote it. Not started; needs a scope ruling.
