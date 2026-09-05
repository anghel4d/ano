# Running Ano

The author's WIP tutorial is [tour.md](../tour.md). It is preserved as authored. This page covers commands supported by the current Rust implementation; [ano-language.md](ano-language.md) describes the language.

## Build and run

From the repository root in Linux:

```sh
nix develop
cargo build --release --workspace
target/release/steel --run demos/1-selection/001-canonical-masked-update.ano
target/release/kore demos/1-selection/001-canonical-masked-update.ano
```

A Rust toolchain and CBQN executable named `bqn` on PATH also suffice without Nix. From Windows, enter the checkout through WSL with an explicit distribution and Linux working directory.

`steel --run` checks fixture expectations. It does not save the resulting world unless `--save` is supplied. Run experiments against a separate registry, not a demo's original fixture.

## Steel commands

```text
steel [--tokens] [--emit] [--run] [--label] [--trace]
      [--dump PATH] [--save PATH] [--rt PATH]
      [--registry PATH-OR-NAME] [--aliases PATH] FILE.ano
```

| Option | Behavior |
|---|---|
| no mode or `--emit` | Write generated BQN to stdout. |
| `--tokens` | Print lexer tokens. |
| `--run` | Execute generated BQN with CBQN. |
| `--registry` | Override the source's registry directive. |
| `--aliases` | Select the dynamic-alias sidecar. |
| `--dump` | Write the loaded registry; without another explicit mode, stop after dumping. |
| `--save` | Save the resulting registry; requires `--run`. |
| `--label` | Add query-result tags for Kore. |
| `--trace` | Add diagnostic records without changing results. |
| `--rt` | Select the runtime prelude. |

The command-line parser is [steel/src/main.rs](../steel/src/main.rs). Kore commands and controls are documented in [kore/kore.md](../kore/kore.md).

## Fixture directives

```haskell
--! registry ../registries/001-canonical-masked-update.reg
--! expect gold = 1100 200 300 1400 500 600

Nord & TwoHanded > 60 , Gold += 1000
```

`--! registry` selects the fixture. `--! expect` checks a column, `--! expect-n` checks entity count, `--! out` checks query output, and `--! ja` selects the Japanese reader. Ordinary `--` comments are not directives.

The first fixture is [001-canonical-masked-update.ano](../demos/1-selection/001-canonical-masked-update.ano); its registry supplies the names and values. Names such as `show`, `polar`, and `phyllotaxis` are not universal built-ins. Examples using them require suitable registry entries.

## Checks

Use the checks relevant to the change:

```sh
cargo test --release --workspace
bash src/check-ano.sh
bash src/check-trace.sh
```

`check-ano.sh` checks paired surface emission, refusal fixtures, and the tick classes in [src/tick-classes.txt](../src/tick-classes.txt). It excludes the quarantine in [todo/TODO.md](../todo/TODO.md). BQN files are explanatory witnesses, not independent semantic oracles.

`nix flake check` runs the Lean, explanatory BQN, and Steel demo checks in isolated builds. Use it for changes to the Nix integration; targeted checks usually suffice for local edits. No full benchmark sweep is part of these instructions.

## A complete small program

Create `world.reg` and `grant.ano` in the same working directory. The registry defines three entity rows; the script selects the Nord with a Two-Handed value above 60.

```text
n 3
col nord bool 1 1 0
col twoHanded num 80 55 70
col gold num 100 200 300
```

```haskell
--! registry world.reg
--! expect gold = 1100 200 300

Nord & TwoHanded > 60 , Gold += 1000
```

Run `steel --run grant.ano` using the built executable on PATH, or its absolute path. The expectation checks the resulting Gold column. Without `--save`, the input registry stays at 100, 200, 300, so another invocation starts from the same values.

To inspect generated code, use `steel --emit grant.ano`. To retain the result, use `steel --run --save after.reg grant.ano`. The registry directive still points to `world.reg`; saving does not rewrite that directive. A later execution against `after.reg` also needs expectations appropriate to its new starting state.

Registry paths in source directives are resolved relative to the source file. This is why the repository examples use `../registries/...`. A copied demo needs either the corresponding registry location or an explicit `--registry` override.

## Choosing a check

A fixture expectation checks values for one declared starting world. The refusal harness checks invalid programs and registries. The trace harness checks diagnostic records and state parity. The tick harness checks bounded repeated execution. Choose the layer whose contract changed; passing one does not establish all the others.

For a documentation example, execute that example with its stated registry. For a parser or emitter change, use the relevant Rust tests and active demo/refusal cases. Build the Lean library when a proof changes. Changes to Nix wiring warrant Nix evaluation and, when validating sandbox execution, the affected derivation.
