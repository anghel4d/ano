# Ano

The spec is `docs/ano-language.md`; current work and demo quarantine are in `todo/TODO.md`. Read relevant directory guidance before editing, and read the spec before changing it.

The Pious Hierarchy: The Mathematics > The Denotation > the domain-and-lineage IR > The Grammar > The Surface > lowering and backend details.

Steel and Kore are the Rust reference implementation. The bytecode VM and JIT target stay fixed. CBQN executes Steel's backend; BQN witnesses are not semantic or differential oracles.

`todo/00-historical-rulings.md` is read-only; preserve the author's rulings. Use live tasks and code for current implementation status. Never resolve an open design question silently.

No heavyweight dependencies or frameworks.

Carry authorized work through to a reviewable result. Make routine implementation choices from context; ask when an answer changes scope or settles an open design decision. User instructions take precedence over skill guidelines. Run checks proportionate to the change, and broaden them only for a failure or unresolved concern.

Keep prose clear and economical, one source line per paragraph or list item. Include enough explanation, examples, and qualifications to preserve meaning. Preserve the author's voice and comments, spec numbering, and idiomatic examples. Keep function comments focused on inputs, outputs, and invariants; inline comments should be terse.

Never credit an agent or AI tool as an author, coauthor, or contributor, or add AI/tool attribution or contributor trailers. As soon as any usable in-scope checkpoint is ready and its relevant verification is complete, commit it and push the current branch immediately unless the user explicitly instructs otherwise. Repeat at each usable checkpoint; do not wait until the entire workflow ends, pile up completed uncommitted work, or request redundant per-commit approval.
