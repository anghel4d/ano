# todo/

Task files from the 2026-07-11 demo-pass review. One work item per file, numbered in execution order, not the order they arose. Each file is self-contained for a dedicated agent: findings with code pointers, the author's rulings (binding), open sub-questions (surface at execution, never resolve silently), work items, and invariants that must hold after. 00 is the exception: a Q/A for the author holding only the unanswered decision points. When a file is executed, cross it off in TODO.md and move it to done/.

Executed (2026-07-11), now under done/: 01-10. The catalog below stands as record; still live here is 00 (the one open ruling), 11-next-steps.md (the blocking residue that needs the author's hand), 12-unbuilt-scans.md (the ruled scan and long-form parity work), 12-spatial-formalization.md (the observable spatial outcome), 13-spatial-formalization.md (the proof-derived implementation handoff), 14-registry-border.md (surfaced 2026-07-19, the two-language ruling and its residue), 15-spatial-surface.md (the capability ladder, both declaration surfaces, and the five-world prototype path), 16-dynamic-alias-overlay.md (the ruled alias overlay, pending implementation), 17-greater-lesser.md (q's ruled carrier-directed dyads, pending implementation), and 18-empty-result-output.md (identityless empty folds produce no query row, pending implementation).

- `00-open-rulings.md` — the deferred decision points, in Q/A form; the answered rulings from the 2026-07-11 pass are folded into their consuming task files. Consuming tasks read the A-lines first.
- `01-kore-query-outputs.md` — query results are computed, asserted, never shown. Make every query visible in kore. FIRST agent task, by ruling.
- `02-registry-types-and-hop-integrity.md` — the s10 bundle: unique/keyed registry types, the proto, fallback defaults, the despawn policy, the functional-hop fix. The deepest item and the first core-design advancement since the initial spec.
- `03-demo-corpus-normalization.md` — global ascending renumbering, the s05 split, the 08 faction→leader rename, the re-annotation pass.
- `04-fold-scan-formalization.md` — the fold/scan permutation table into the docs verbatim, new `&\` and `|\` demos, the max ruling.
- `05-reducer-surface-and-sigils.md` — `f/` over registered reducers, `fold()` as the long form, the `!` vs `^` tabulation and the `^bind` seam.
- `06-manual-srel-vec-chapter.md` — document set-valued relations and vec columns in ano-manual.md.
- `07-tui-editors.md` — the vim.tiny diagnosis, EDITOR=nvim at project and nix level, the internal editor.
- `08-world-map-and-bitmap.md` — square cells, inferred glyphs, unicode pieces, the half-block bitmap mode.
- `09-numeric-edges.md` — document the float64 model and close the non-finite load/save asymmetry.
- `10-debug-observability.md` — the dead-link diagnostic, the per-tick structural trace, the session log as the repro channel. Mission-critical by ruling.
- `12-unbuilt-scans.md` — the `min\` bridge, running mean, running count, and general operator-or-reducer heads for `fold(f)`, `scan(f)`, and `scan2(f)`. Ruled. Steel implementation and tests are pending. Only the running-count spelling remains open.
- `12-spatial-formalization.md` — the observable Steel/Kore outcome for habitats, domains, lineage, and spatial behavior. Specified and proved at the semantic boundary where stated; not implemented in Steel or Kore.
- `13-spatial-formalization.md` — the proof-derived implementation handoff: typed descriptors, sealed tokens, exact-51 spatial spawn pipeline, migration sequence, acceptance tests, and trust boundary. Specified; not implemented in Steel or Kore.
- `14-registry-border.md` — the 2026-07-19 two-language ruling (registry declarative, `.ano` the sentence, Σ immutable during execution) and its three new items: the Σ→Σ′ migration protocol, the placement schema/world seam the Lean currently contradicts, the gated adjective-noun registry surface.
- `15-spatial-surface.md` — the ten-rung capability ladder mapped to `spatialmaths.md` §22, the maximal and minimal spatial declaration surfaces with the elaboration law (defaults expand to the identical certificate), and the five-world prototype path that lands below the certificate line under the §26 second-tick discipline. Candidate spelling for 14's item 3; gated on the author's go.
- `16-dynamic-alias-overlay.md` — the 2026-07-21 `^name` ruling: live alias first, bare-name fallback, and alias rebind/delete without mutation of the bare binding. Ruled. Not implemented in Steel or Kore.
- `17-greater-lesser.md` — the 2026-07-21 Greater/Lesser ruling: `|` is OR or maximum and `&` is AND or minimum, with folds, scans, bridges, and carrier-specific empty laws. Ruled. Not implemented in Steel or Kore.
- `18-empty-result-output.md` — the 2026-07-21 empty-result ruling: an identityless fold over nothing yields no row, exactly like an unmatched selection. Steel's bare-query `0` placeholder must be removed.
