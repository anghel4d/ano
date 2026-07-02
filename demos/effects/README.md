# effects

BQN models of the effect side of the one form: value effects, assignment, structural effects, sequenced effects, derived columns, and the barrier laws.

- 10-value-effects.bqn — ex10: masked add, subtract, multiply, divide down dense columns; off-mask cells identical to pre-state.
- 11-assignment.bqn — ex11: masked overwrite with a backtick-symbol constant, via (const each)-under, never a bare scalar under.
- 12-structural-effects.bqn — ex12: despawn filters every column; +Frenzied lands on the deduplicated set-hop image (3 targets from 4 edges); -Encumbered clears under Burdened.
- 13-sequenced-effects.bqn — ex13: `;`-batched effects read one pre-state and merge at one scatter; the batch commutes.
- 25-derived-columns.bqn — ex25: a def column reused as effect source, predicate, scoped fold, and descending ordering key.
- s07-masked-multiply.bqn — spec §7 correction: doubling is gold × 1+merchant; 2×merchant wipes non-merchants; the two are asserted to differ.
- s10-pre-state.bqn — spec §10: two batched effects both read pre-state, so Gold = Silver ; Silver = Gold swaps; naive sequential application loses the swap.
- s10-barrier-granularity.bqn — decision record barrier / ex13-ex49 ruling: one statement is one barrier, `~` is a new barrier reusing the saved mask, so spawned ghosts survive the despawn; a re-gather would kill them.
