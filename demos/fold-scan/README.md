# fold-scan

Reductions and scans (ex14-17), the §12 fold contract, and the §21 Fibonacci stencil-vs-recurrence pair (the s11-/s19- prefixes key to the pre-renumbering sections).

- `14-reductions.bqn` — scoped-global folds `+/ */ &/ |/ #/`: hand-checked scalars, order-independence, empty-scope identities.
- `15-named-reducer.bqn` — `reduce(threat)` as a registered associative reducer; no identity, so the empty scope fails the row (BQN's `¯∞` identity noted as a divergence).
- `16-scans.bqn` — `+\ *\ max\` along generated views in index order; same-length running columns.
- `17-scan-along.bqn` — `scan(+) Weight along pathCells`; along carries the order, the mask spelling of the same cells loses it.
- `s11-avg-fold-finish.bqn` — avg is fold+finish, not a raw reduction: pairwise-mean associativity failure witness; count as the fold of ones; empty scope fails.
- `s19-fib-stencil.bqn` — one barrier step of `offset = prev.offset + prev.prev.offset` is a shift-add stencil over pre-state; k steps give the binomial ridge, never fib.
- `s19-fib-scan.bqn` — the true recurrence as an order-carried scan (Version B, host-side `fib(index)`); differs from one stencil step from the same seed.
