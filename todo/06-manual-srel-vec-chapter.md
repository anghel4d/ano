# 06 — the manual: set-valued relations and vec columns

Ruling (author, 2026-07-11, on multi-entries-in-one-column): "This is fine, just make sure it is documented in the learnyouanano doc and such."

The learnyouanano doc exists: `ano-manual.md` ("LEARN YOU AN ANO TO BECOME GREAT AND POWERFUL"). Extend it, likely the registry chapter and the grouped-fold chapter, with the settled story, in the manual's register (teach by running code, every block pinned by a suite):

- A one-to-many relationship is another relation, not a ragged cell. `srel targets 3 4 | | 4 5 | …` is fibers per row. In the BQN prototype, a list of lists. In Steel, CSR: one offsets column plus one flat edge array (ano-ecs.md §5). Two flat arrays, still array-like all the way down, the same shape as q's nested columns and Arrow's list columns.
- The mask algebra never sees multiplicity: `Frenzy.targets'` in source position is the image, bits OR'd into a mask, an entity reached twice is one bit. The idempotent-scatter law is free because the representation cannot express multiplicity (ano-ecs.md §4).
- When multiplicity matters you say so with a fold: `Hits += #/ attackers'`, γ, fold-under-each over the fibers (spec §13). Show the image and the in-degree side by side over the same fixture, the manual's two-spellings tradition.
- Vectors as cell values exist in the disciplined form: `col pos vec` holds fixed-arity pairs, `bind pathCells vec` holds index sequences. The discipline, stated: fixed-shape tuples live in vec columns, while ragged, variable-arity data lives behind a relation, named, with fibers, an inverse, and the γ machinery. Both are array-native. Neither leaks raggedness into the mask algebra.

Cross-reference spec §5 (the set hop), §13 (grouped fold), ano-ecs.md §§4-5. If task 02's proto/unique/keyed kinds have landed by execution time, the registry chapter documents those in the same breath. If not, document today's kinds and leave a seam.

## Invariants

- Every code block in the new material cites a runnable file whose post-state is asserted, the manual's own contract ("the manual cannot drift from the language without a suite going red").
- Author's voice. Tighten, don't rewrite existing chapters.
