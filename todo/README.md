# todo/

Task files from the 2026-07-11 demo-pass review, one work item per file, numbered in intended execution order (not the order they arose in review). Each file is self-contained for a dedicated agent: findings with code pointers, the author's rulings (binding), open sub-questions (surface at execution, never resolve silently), work items, and invariants that must hold after. 00 is the exception: a Q/A for the author holding only the not-yet-answered decision points. When a file is executed, cross it off in TODO.md and archive the file to .archive/.

- `00-open-rulings.md` — the residue: eight decision points with no recorded answer, Q/A format; the consuming tasks read the A-lines first.
- `01-kore-query-outputs.md` — query results are computed, asserted, and never shown; make every query visible in kore. FIRST agent task, by ruling.
- `02-registry-types-and-hop-integrity.md` — the s10 bundle: unique/keyed registry types, the proto, fallback defaults, the despawn policy, and the functional-hop correctness fix. The deepest item; the first genuine core-design advancement since the initial spec.
- `03-demo-corpus-normalization.md` — global ascending renumbering, the s05 split, the 08 faction→leader rename, and the re-annotation pass.
- `04-fold-scan-formalization.md` — the fold/scan permutation table into the docs verbatim; new `&\` and `|\` demos; the max ruling.
- `05-reducer-surface-and-sigils.md` — `f/` over registered reducers, `fold()` as the long form, the `!` vs `^` tabulation and the `^bind` seam.
- `06-manual-srel-vec-chapter.md` — document set-valued relations and vec columns in ano-manual.md.
- `07-tui-editors.md` — the vim.tiny diagnosis, EDITOR=nvim at project and nix level, the internal editor.
- `08-world-map-and-bitmap.md` — square cells, inferred glyphs, unicode pieces, the half-block bitmap mode.
- `09-numeric-edges.md` — the float64 model documented; the inf load/save asymmetry closed so the seal is airtight.
- `10-debug-observability.md` — the dead-link diagnostic, the per-tick structural trace, the session log as the repro channel. Mission-critical by ruling.
