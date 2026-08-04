# Ano — issues

## Non-spatial implementation seams

- Legacy item-shape compatibility. High-integrity `array` declarations now retain explicit carrier/domain metadata, stable identity/versioning, bit-exact runtime sidecars, and migration receipt resealing. The remaining seam is the old `vec` convention inside `col`/`field`: it detects pair items from `2×rows`, so an empty legacy world has no witness to recover. Ruling: never infer that witness at zero rows and never treat `vec` as a typed capability; a future product carrier must state item shape nominally and migrate explicitly. The legacy save refusal is therefore a compatibility limit, not a blocker on the new registry taxonomy.
- Legacy text payloads. High-integrity resident arrays and constructed values now use total canonical sidecars, but the legacy whitespace `.reg` rows still cannot represent every empty or boundary-bearing symbol/glyph post-state. Ruling: keep the atomic refusal instead of adding a partial quoting exception; a successor world-state codec must encode every carrier uniformly and land with Steel parsing, save pipe-back, Kore edit/display, migration, and journal recovery in one change. Declaration text remains canonical and no lossy save is permitted meanwhile.
- Standalone rule clock. Language semantics are settled: installing or retracting a rule changes the host rule set at a barrier, while only a host tick schedule fires it. Steel's line-oriented fixture runner still synthesizes one tick after each fresh installation run because it has no positional mission schedule; this is a harness limitation, not Ano semantics. The replacement is the host-side `.mission` schedule, never an Ano `tick` statement.
- Temporal prelude. `ago(k)`, `window(k)`, sealed history partitions, and `.mission` lock manifests are ruled in `ano-time.md` but not yet implemented in Steel/Kore.

## Spatial task 99

- World-to-chunk frames. The anchored frame fills an origin from the world, but the world-to-chunk `(o,S)` conversion has no implemented surface.
- Mixed-habitat rule ticks. All rules sharing a current Steel tick must select in one habitat; a mixed entity-and-lattice barrier remains part of the typed spatial implementation.
