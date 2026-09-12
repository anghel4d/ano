# Rules, ticks, and replay

A command statement executes one barrier. Named standing rules install with `def name = selection => effects` and retract with `undef name`. Rule effects read a shared pre-state; queued commands execute in order.

## Current runner

Steel processes source in order. An unbroken run of fresh rule installations schedules a rule step. A following control or command boundary seals that step; `undef` removes the named rule for subsequent steps. Unknown retractions and duplicate live names refuse.

This file-oriented scheduling is visible in [steel/src/emit.rs](../steel/src/emit.rs) and tested in [rule_retraction.rs](../steel/tests/rule_retraction.rs). It is not a general host mission clock.

Kore's `n` reruns the staged program against the saved world. Fixture expectation directives are removed from that interactive program. The demo tick driver separately checks the pristine first tick and its declared later trajectory. See [tick-induction.md](../proofs/tick-induction.md).

## Reproducibility

Repeating an execution requires the same source, registry schema and values, alias environment, host inputs, callable behavior, and backend behavior. A text statement log alone does not guarantee replay on arbitrary machines.

## Not implemented

There is no built-in `ago(k)`, `window(k)`, sealed history-partition service, `.mission` lock-file parser, or mission hot-swap protocol in Steel/Kore. Earlier descriptions of their formats were proposals, not executable interfaces.

A host may register ordinary relationships and functions for its own time data. That does not make those proposed names or persistence contracts built-ins.

## Installation and retraction example

With Boolean columns `Plot` and `Planted`, and a numeric `Gold` column:

```haskell
def spread = Plot => +Planted
Plot , Gold += 1
undef spread
Plot , Gold += 1
```

The fresh installation schedules `spread` before the first following command boundary. Retraction removes it from subsequent scheduling; both explicit Gold commands still execute. This order is checked in the rule-retraction tests. It does not mean that a stored registry independently runs the rule in the background.

Kore's next-step action stages and executes source again against the current saved world. A host that needs a durable rule scheduler must define that lifecycle explicitly. The registry file alone is not a serialized continuation of the standalone compiler.

## Replay boundary

Distinguish a command log, a saved world, and the environment needed to reproduce execution. The log records requested operations. The world records published registry state. Replay additionally depends on declarations, aliases, frozen inputs, and callable/backend behavior.

A future history or mission service must specify how it captures those dependencies and handles missing history or changed versions. The proposed service names below do not provide that machinery today.
