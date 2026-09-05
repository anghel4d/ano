考案: Anghel (Anghel4d)

# Examples

The current authored walkthrough is [tour.md](../tour.md). Runnable programs and their expectations live in [demos/](../demos/); [demos/demos.md](../demos/demos.md) indexes them.

Use a demo with its registry. These names are supplied by fixtures, not built into the language.

| Subject | Example |
|---|---|
| Selection and masked update | [001](../demos/1-selection/001-canonical-masked-update.ano) |
| Scope | [004](../demos/1-selection/004-scoped-selection.ano) |
| Language, commands, and limits | [Language reference](ano-language.md), [running Ano](ano-manual.md) |
| Registry declarations | [Registry reference](ano-registry.md) |
| Japanese reader | [Japanese surface](ano_nihongo.md) |

`Merchant @ Whiterun` and `Merchant & Whiterun` produce the same mask in the current compiler. Fold scope, shape scope, and anchored scope take different emitter paths; the equality does not extend to every use of `@`.

Demo status comes from the quarantine block in [todo/TODO.md](../todo/TODO.md). A file's presence, a BQN sibling, or an old expected-output comment does not make a quarantined example supported.
