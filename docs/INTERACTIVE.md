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

The host registry defines names, carriers, relationships, bindings, callables, prototypes, and spatial capabilities. Steel owns language validation and execution. `.reg` is the repository's text fixture and save boundary. Kore presents that world without becoming a second semantic authority.

## The staged file, zoomed

```text
declarations
columns and presence
relationships and fibers
bindings, aliases, defaults, prototypes, and callables
fields and current spatial data
```

Save and reload must preserve every declaration the loader understands. Spatial formalization requires habitat, layout, frame, carrier, boundary, and bridge identity to survive the same boundary. That metadata is pending in Steel and Kore.

## The entry taxonomy

- `col`, `pres`, and `unique`: stored columns and carrier refinements.
- `rel`, `srel`, and `inv`: functional and set-valued relationships.
- `bind`, `default`, and registry `def`: constants, spawn defaults, and prototypes.
- `fn`: registered callable behavior.
- `as`, `ja`, and `role`: derived names, surface names, and system roles.
- Fixture `alias`: stored fixture masks, distinct from the pending live `^name` overlay.
- Spatial declarations: named habitats and capabilities. Equal length never establishes alignment.

## The data model — the ladder

The order is Mathematics > Denotation > domain-and-lineage IR > Grammar > Surface > lowering. Kore may display storage details, but those details do not define Ano.

The table view presents stored columns. Map and bitmap views must use declared habitat, placement, position role, and axes. They must not infer a 2D ground or universal `(x,y)` order from buffer length or a raw pair. The required repair is tracked in `todo/TODO.md`.

The session log and its base registry are the repro artifact. Trace output is observational only and cannot change output or post-state.
