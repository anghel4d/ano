# generate

Generation demos, ex20-24 plus spec §16-18 (the s16- prefix keys to the pre-renumbering section): the outer product as a theta-join, replicate and reshape as the key-creating primitives, and the subset law that separates them from selection.

- `20-outer-product-comprehension.bqn` ex20: the comprehension as sigma_p(A x B) — build the product, filter by the predicate, effect once per surviving pair; self join deduped by a < b.
- `21-outer-product-value.bqn` ex21: `cross dist Tower Creep` materialized; ex20 is a filter over this value.
- `22-replicate-spawn.bqn` ex22: `spawn Minion * Count` — per-source counts mint fresh keys with count-replicated parent links; zero counts emit nothing.
- `23-expand-alias.bqn` ex23: `expand Count` as the flat-map stage; expand-then-spawn-one equals spawn-times-Count row for row.
- `24-reshape-positions.bqn` ex24: `pos = to 8 8` / `to 4 _` / `to 20` — pour the selection into a lattice in key order; repositions values, mints no keys.
- `s16-keygen.bqn` a filter cannot invent a key: exhaustive over all masks of a small world, filtered keys are an order-preserving subset; generation and ↕shape are the only key sources.
- `s61-proto-spawn.bqn` the proto and the three-layer spawn fill: `def Marine soldier=1 hp=100` is the registered archetype; spawn fills each column proto value → registered default → type zero, the unique id mints fresh (injectivity admits nothing else), and the proto-role column records the archetype's noun.
