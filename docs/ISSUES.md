# Ano — issues

## Non-spatial implementation seams

- Typed registry values. The current loader has only flat legacy `col`/`field` rows, so a fixed-shape item column emptied by despawn can lose its item-shape witness and an order cannot yet derive its relation fibers from one typed ground declaration. Save refuses an unrepresentable empty shaped value rather than guessing; the successor descriptor and migration contract remain behind the explicit persisted-schema safety gate.
- Canonical text payloads. The legacy whitespace format cannot represent every empty or boundary-bearing symbol/glyph value, and current untyped storage can admit a part-shaped column. Save refuses those worlds atomically rather than writing a file the loader would reinterpret; the successor text encoding belongs to the same persisted-schema gate.
- Standalone rule clock. Language semantics are settled: installing or retracting a rule changes the host rule set at a barrier, while only a host tick schedule fires it. Steel's line-oriented fixture runner still synthesizes one tick after each fresh installation run because it has no positional mission schedule; this is a harness limitation, not Ano semantics. The replacement is the host-side `.mission` schedule, never an Ano `tick` statement.
- Temporal prelude. `ago(k)`, `window(k)`, sealed history partitions, and `.mission` lock manifests are ruled in `ano-time.md` but not yet implemented in Steel/Kore.

## Spatial task 99

- World-to-chunk frames. The anchored frame fills an origin from the world, but the world-to-chunk `(o,S)` conversion has no implemented surface.
- Mixed-habitat rule ticks. All rules sharing a current Steel tick must select in one habitat; a mixed entity-and-lattice barrier remains part of the typed spatial implementation.
