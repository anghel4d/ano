# demos/1-selection

BQN models of the selection layer: examples 1-9, the spec §8 scatter fix, and the setHop decision record. Each file builds a small column-store world, applies the ano statement, and asserts the exact post-state. Column names are lowercase because BQN reserves uppercase-first spellings for functions; `sym`-prefixed constants stand in for backtick enum symbols.

- 01-canonical-masked-update.bqn — ex1: `Nord & TwoHanded > 60 , Gold += 1000`, the masked add.
- 02-core-sentence-form.bqn — ex2: `source & predicate , effect` as effect-under-mask-compress, checked against the masked-arithmetic form.
- 03-component-masks.bqn — ex3: presence masks under & | !, the `Bandit enum sigil, constant assignment via (v¨)⌾(mask⊸/).
- 04-scoped-selection.bqn — ex4: `Merchant @ Whiterun`, the locative scope as an AND with the region mask.
- 05-target-aliases.bqn — ex5: @cursor/@observer/Player/@selected as small masks driving ordinary effects.
- 06-presence-patterns.bqn — ex6: present-constrained, present-any, absent; the presence guard keeps junk cells out of value predicates.
- 07-value-predicates.bqn — ex7: pointwise column comparisons; sequential statement barriers observed via the greedy mask.
- 08-relationship-hops.bqn — ex8: `rel.Comp` as a guarded indexed gather; dangling links fail the predicate (left-join-null).
- 09-named-selections.bqn — ex9: defs as predicates re-gathered per statement, not saved masks; `;`-batched effects on one mask.
- s05-set-hop.bqn — setHop record: fiber image as a mask (idempotent scatter), |/ &/ #/ quantifiers, empty-fiber identities, in-degree via the inverse-fiber gamma fold.
- s08-masked-scatter.bqn — spec §8 fix: assignment scatter is index-from-mask (/mask), modeled as value-under-mask-compress; the mask-as-index misread demonstrated.
