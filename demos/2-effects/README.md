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
- s10-soul-gate.bqn — the loop-safe variant (ruled 2026-07-11): the spawn's fuel (Soul) is zeroed in the same barrier that mints, ghosts take the type-zero soul, so the step is linear and terminating — Step ∘ Step ≡ Step; the barrier-granularity original keeps its exponential growth deliberately.
- s58-hop-after-despawn.bqn — the wrong-entity shape (PROBLEM 1): no id column, so the rel is keyed to the fixture row index; after a despawn the hop resolves by index-of against the surviving fixture-row ids — a positional read of the shifted column lands on the wrong entity, pinned as a counterexample.
- s59-hop-out-of-range.bqn — the out-of-range shape (PROBLEM 2): a stored id at or beyond the post-despawn row count; the index-of resolution reads the right survivor where a positional gather is a hard index fault — staleness stays a mask question, never an error path.
