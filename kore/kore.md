# Kore

Kore is the Rust terminal editor and viewer in `kore/src/`. It invokes Steel, which invokes CBQN. Build both with `cargo build --release --workspace`; run with `bqn` on PATH.

Steel discovery checks `$STEEL`, a sibling executable, `target/release/steel` under the current directory, then PATH.

## Commands

```text
kore
kore file.ano
kore file.reg
kore --check file.reg ...
kore --edit file.reg seg row col value
kore alias file.reg list|set|mask|resolve|delete|clear
kore migrate live.reg candidate.reg migration.map
```

Bare invocation scans demos from the current directory. An Ano file opens the code/world view; a registry opens a world prompt. `--check` loads and renders to memory without a terminal. `--edit` is a headless splice: use a copy; paths under demos are refused.

## Controls

Outside text entry:

| Key | Action |
|---|---|
| `q` | Quit |
| Tab | Change focus |
| `:` or `>` | Prompt |
| `m` | Cycle table, map, bitmap |
| `r` | Reset/reload |
| `n` | Run the next staged program step |
| `u` | World undo |
| `w` | Snapshot |
| `t` | Toggle trace |
| `E` | External editor |
| arrows or `j/k` | Move/scroll |
| Escape | Leave the current input/focus |

Code focus has its own editing bindings: `u` is code undo, `w` is word motion, and `n` advances an active search. Clear that search before using `n` to tick. In insert mode and prompts, keys are text.

The external editor uses `VISUAL`, then `EDITOR`, then an available terminal editor. Mouse clicks focus/select; the wheel scrolls; dragging world rows or spatial cells forms a selection.

## World and output

Demo mutations use play copies beneath `.kore/play/`; originals remain unchanged. A bare registry outside the demo tree can be edited in place. Ordinary writes stage a file and rename it. Undo and snapshots retain previous registry states under `.kore/`, qualified by path and schema generation.

The interactive `n` program retargets the registry and removes fixture output/post-state assertions. Running the original demo through `steel --run` still checks those assertions.

Queries enter the outputs pane. Compiler messages and trace events enter history. An identityless empty result creates no fabricated output record. The [trace reference](../src/trace/trace.md) defines the channel grammar.

Map and bitmap views still use legacy lattice/position conventions. They are not a validated arbitrary-rank or nominal-frame interface; task 99 owns that repair.

## Aliases and schema changes

Alias administration changes the sidecar environment, not bare registry declarations. Ordinary registry undo is not an alias-history transaction.

Migration stages the registry, aliases, schema manifest, and event log. After validation, durable backups and a prepared journal allow interrupted publication to roll back. Committed or rolled-back journals are cleaned on recovery. The journal is `<live.reg>.migration-journal`.

External runtime arrays and constructed values require receipt revalidation separately. A successful file migration does not automatically update a host's cached values.

## Terminal implementation

Rendering uses ANSI sequences and the Unix terminal boundary in `sys.rs`. The palette probe queries OSC 10/11 with a bounded deadline and falls back when replies or contrast are unsuitable. Source and tests in `palette.rs`, `term.rs`, `ui.rs`, and `main.rs` define the detailed behavior.
