# fold-scan

Reductions and scans (ex14-17), the §12 fold contract, and the §21 Fibonacci stencil-vs-recurrence pair (the old s11-/s19- prefixes, kept as (was …) header markers, key to the pre-renumbering spec sections).

- `028-reductions.bqn` — scoped-global folds `+/ */ &/ |/ #/`: hand-checked scalars, order-independence, empty-scope identities.
- `029-named-reducer.bqn` — `fold(threat)` as a registered associative reducer; no identity, so the empty scope fails the row (BQN's `¯∞` identity noted as a divergence).
- `030-scans.bqn` — `+\ *\ max\` along generated views in index order; same-length running columns.
- `031-scan-along.bqn` — `scan(+) Weight along pathCells`; along carries the order, the mask spelling of the same cells loses it.
- `032-avg-fold-finish.bqn` — avg is fold+finish, not a raw reduction: pairwise-mean associativity failure witness; count as the fold of ones; empty scope fails.
- `038-ever-any.bqn` — the `|\` latch (`∨` scan): has the fire reached each point along the route; trips on at the first true and stays on.
- `039-still-all.bqn` — the `&\` latch (`∧` scan): the column intact up to here; trips off at the first false and stays off.
- `040-reducer-spellings.bqn` — one registered reducer, three spellings: `threat/`, `fold(threat)`, `threat\` — the two folds pinned against one value, the scan as running peak.
- `034-fib-stencil.bqn` — one barrier step of `offset = prev.offset + prev.prev.offset` is a shift-add stencil over pre-state; k steps give the binomial ridge, never fib.
- `033-fib-scan.bqn` — the true recurrence as an order-carried scan (Version B, host-side `fib(index)`); differs from one stencil step from the same seed.
