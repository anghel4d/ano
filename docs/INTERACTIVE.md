# INTERACTIVE — the ano editor.

Kore is Ano's interactive world. It opens `.ano` and `.reg`, runs Steel, displays the resulting columns and outputs, and edits the world through the text registry boundary. `kore/kore.md` is the operational reference.

## The panel and the commit loop

```text
┌─ demos ─┬─ code ───────────────┬─ world / map / bitmap ─┐
│ rail    │ .ano play copy       │ columns and fields      │
├─────────┴───────────────────────┴─────────────────────────┤
│ outputs                         │ history and trace       │
├─────────────────────────────────┴─────────────────────────┤
│ > statement                                               │
└───────────────────────────────────────────────────────────┘
```

Demo files are immutable. The first mutation creates a play copy under `.kore/play/`. Every successful statement, tick, or cell edit stages the pre-state on the undo ring and commits the new `.reg` by rename. A failed operation leaves the world unchanged.

`r` restores the pristine registry, `n` advances one tick, and `u` restores the previous world. Prompt statements run as Ano programs against the play registry. Query results enter outputs. Compiler output and trace diagnostics enter history.

## The registry, three layers

```text
host declarations → Steel world and typed plan → .reg fixture/save boundary → Kore views and edits
```

The current registry defines names, carriers, relationships, bindings, callables, prototypes, and the archival singleton-lattice fields that task `99` must replace with nominal spatial capabilities. Steel owns language validation and execution. `.reg` is the repository's text fixture and save boundary. Kore presents that world without becoming a second semantic authority.

## The staged file, zoomed

```text
declarations
columns and presence
relationships and fibers
bindings, aliases, defaults, prototypes, and callables
fields and current spatial data
```

Save and reload preserve every declaration the loader understands. Host-owned state lives beside, not inside, this grammar: `.reg.aliases` is the live dynamic overlay, `.reg.schema` is the stable declaration-ID manifest, and `.reg.migrations` is the deterministic schema-event log. Spatial habitat, layout, frame, boundary, and bridge identity remain pending in task `99` and are not inferred from the existing lattice buffers.

## The entry taxonomy

- `col`, `pres`, and `unique`: stored columns and carrier refinements.
- `rel`, `srel`, and `inv`: functional and set-valued relationships.
- `bind`, `default`, and registry `def`: constants, spawn defaults, and prototypes.
- `fn`: registered callable behavior.
- `as`, `ja`, and `role`: derived names, surface names, and system roles.
- Fixture `alias`: stored fixture masks, distinct from the live `^name` overlay in `.reg.aliases`.
- Spatial declarations: named habitats and capabilities. Equal length never establishes alignment.

## Schema barriers

`kore alias world.reg list|set|mask|resolve|delete|clear` administers the overlay between statements. Each successful change publishes one new environment version; a failed target resolution leaves the old file and version untouched. The concrete resolver IDs are `deictic.cursor`, `deictic.observer`, `deictic.selected`, and `deictic.world`, with their host inputs frozen for one statement.

`kore migrate live.reg candidate.reg migration.map` is the pre-spatial `Σ → Σ′` barrier. The separate map must account for every declaration with `preserve`, `rename`, supported `widen`, `drop`, explicit `discard`, `add`, or `unalias`. Kore stages and reload-validates the registry, alias overlay, schema manifest, and event log, then journal-publishes them together. A failure restores the exact old bundle; startup validation restarts an interrupted prepared rollback and cleans durable committed or rolled-back states before opening the world. Old plans and handles cannot cross the new schema without receipt revalidation; Kore's registry-only undo ring is generation-qualified for the same reason. Lattice changes refuse at the typed extension seam until task `99` implements them.

## The data model — the ladder

The order is Mathematics > Denotation > domain-and-lineage IR > Grammar > Surface > lowering. Kore may display storage details, but those details do not define Ano.

The table view presents stored columns. Map and bitmap views must use declared habitat, placement, position role, and axes. They must not infer a 2D ground or universal `(x,y)` order from buffer length or a raw pair. The required repair is tracked in `todo/TODO.md`.

The session log and its base registry are the repro artifact. Trace output is observational only and cannot change output or post-state. Machine events carry a stable use ID, phase, and domain: predicate crossings range over source `X`, effect crossings over selected `S`; Kore strips only the 0x1F channel byte and forwards the remaining line to history without aggregation or OUTPUTS contamination. `src/trace/trace.md` is the exact grammar and fixture reference.
