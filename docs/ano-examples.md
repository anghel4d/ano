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

## Simultaneous and sequential effects

The distinction is whether a later effect can see an earlier effect's writes. `;` shares an incoming state; `|>` passes the resulting state forward. [Section 10](ano-language.md#10-simultaneous-and-sequential-effects) gives the language contract.

Use this `world.reg` for each example independently. Nord, Silver, and Gold are declared columns; the middle row is not selected.

```text
n 3
col Nord bool 1 0 1
col Silver num 30 70 4
col Gold num 10 90 8
```

With `;`, both assignments read the original values and publish their writes together. This swaps the two values in each selected row. This example runs in current Steel:

```haskell
--! registry world.reg
--! expect Silver = 10 70 8
--! expect Gold = 30 90 4

Nord , Silver = Gold ; Gold = Silver
```

With `|>`, the first assignment finishes before the second reads. Both values become the original Gold in each selected row. The following expectations specify the required result; current Steel/Kore reject this effect pipeline:

```haskell
--! registry world.reg
--! expect Silver = 10 70 8
--! expect Gold = 10 90 8

Nord , Silver = Gold |> Gold = Silver
```

For the first row, the difference is:

| Composition | Incoming Silver, Gold | After the effects: Silver, Gold |
|---|---|---|
| `Silver = Gold ; Gold = Silver` | 30, 10 | 10, 30 |
| `Silver = Gold \|> Gold = Silver` | 30, 10 | 10, 10 |

The operator also sequences selection stages on the left of the comma. Using the same registry, this selects the two Nords, orders them by descending Gold, takes the first, and writes only that row's Silver. This example runs in current Steel:

```haskell
--! registry world.reg
--! expect Silver = 10 70 4
--! expect Gold = 10 90 8

Nord |> order by Gold desc |> take 1 , Silver = Gold
```

Selection pipelines and effect pipelines share the left-to-right composition rule. The [missing effect-pipeline implementation](ISSUES.md) does not change that rule.
