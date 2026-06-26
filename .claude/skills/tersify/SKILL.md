---
name: tersify
description: Tighten prose and naming in the given markdown files
argument-hint: "[files or section]"
---
Simplify the FUCK out of the prose in $ARGUMENTS.
Tersify must always preserve semantic meaning.

Prose:
- One clause each. Cut dragging run-on sentences down to their meaning.
- No sentence should use a ; or - or -- or — to splice two thoughts. Needing one means the sentence is too long, so truncate it and get the meaning across already. a -> b is ok.
- A point needing paragraphs means the idea is unclear. Sharpen the claim instead.
- Use the shorter synonym when a reader reads it identically. eg: "the selection predicate" --> "the predicate"
- Flat prose. No decorative bolding. Bold only academically for load-bearing terms.
- One long line per paragraph or list item. Do not hard-wrap at a column.
- Keep my own headings, section numbering, and my own comments verbatim.
- Preserve semantic meaning.

Naming, in code examples:
- Keep each example idiomatic to the language it cites.
- Rename only where prose was doing a good name's work. Conservative, no meaning change.

After: show `git diff --stat`.
