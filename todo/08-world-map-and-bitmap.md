# 08 — the world map and the bitmap: the great space-mapping problem

Ruling (author, 2026-07-11): fix the space surface's limitations and "actually start Dwarf Fortress-maxxing." Deferred behind 01-05 by the author's own sequencing. The spatial demo series gets its detailed audit only after the rest is ironed out.

## The finding (recorded, verified)

kore draws EVERY positioned entity as a hardcoded bold `@` (kore.c:2481), stamped over the field layer. The 6-space demos are data-sound (every expect passes, both suites green) but the renderer is glyph-blind for entities: 27-lattice-patterns (an entity on every cell) is a solid wall of @, and 34-board-literal is 32 @'s while its `proto` column, which holds the actual piece glyphs, is ignored. "Broken, just shows a grid of @'s" was the renderer, not the demos.

## Work items

- Square cells: a TUI cannot load fonts, so render each map cell two columns wide, giving ~1:1 cells in any font (and trivial mouse mapping). A genuinely square font is a terminal-profile matter, or a later standalone shell drawn from the anoptic-engine parts bin.
- Entity glyphs, unicode: the plumbing exists. put() is wide-glyph-safe with straddle guards. Missing is only the convention: a sym column supplies each entity's glyph (`proto` today, `glyph` by role), `@` the fallback. The chess demo then shows its actual pieces. Case already carries color in that fixture, so case→fg falls out free. Everything else in kore is already unicode-clean.
- The archetype→glyph/color map. The author's options, recorded: a kore-only shadow-reg mapping archetypes to glyphs and colors ("Registry question, perhaps?"), or inference at runtime/bake time. For now kore infers: glyph from the proto/glyph sym column when present, else a stable color per archetype bool column (hash the name into the palette). If task 02's proto construct lands first, the proto IS the archetype noun the map wants: a proto entry could carry `glyph`/`color` fields and the shadow-reg becomes unnecessary. Coordinate. "we keep edging archetypes" — this is where they'd finally pay rent.
- BITMAP mode: a third space view beside table and glyph map. Zero-dep answer: the half-block renderer. `▀` with fg = upper pixel and bg = lower pixel gives two vertical pixels per cell, so the map region becomes an RGB canvas at 2x vertical resolution (the standard terminal-image trick). Entity → colored pixel via the same archetype color inference, fields → shaded background. Kitty graphics protocol / sixel later as progressive enhancement behind the same interface, never a dependency.

## Invariants

- Table view, glyph map, and bitmap are views over one world state. The cell cursor and direct-edit path keep working in all three (the map-edit path routes through the same .reg splice machinery).
- kore --check still walks every registry through both existing views. Extend it to the bitmap.
- Zero deps, raw CSI only. Wide-glyph invariants (no straddle across region edges) hold in the new modes.
- The spatial demo series audit (subtly-off feelings recorded in review) happens here, after the renderer is honest. Suspicions may evaporate once entities render distinguishably.
