# 17 — Greater and Lesser

Status. Steel/Kore implementation and tests are pending.

## Ruling

Ano adopts q's Greater and Lesser operations over Ano's admitted carriers.

`a | b` is OR on masks and pointwise maximum on numbers. `a & b` is AND on masks and pointwise minimum on numbers. Masks and numbers do not coerce into one another. Mixed application refuses. Boolean OR remains `|`. There is no `||`.

The derived folds and scans are the same q operations. Mask `|/` is ANY and numeric `|/` is maximum. Mask `|\` is ever-any and numeric `|\` is running maximum. Mask `&/` is ALL and numeric `&/` is minimum. Mask `&\` is still-all and numeric `&\` is running minimum.

`max/` and `max\` remain numeric bridges for `|/` and `|\`. `min/` and `min\` remain numeric bridges for `&/` and `&\`. The paired spellings must compile and evaluate identically.

The empty law is carrier-directed. Mask `|/` has identity false. Mask `&/` has identity true. Ano numbers are finite float64, so numeric Greater and Lesser have no identity in the carrier. Empty numeric `|/`, `&/`, `max/`, and `min/` fail the row. Empty scans yield empty columns. An identityless bare query prints nothing. Steel's placeholder removal is pending in `todo/18-empty-result-output.md`.

## Current implementation

Steel parses `|` and `&` as `Or` and `And` nodes. `steel/src/emit.rs` sends both through mask emission, so direct numeric Greater and Lesser do not exist. The operator folds and scans are also mask operations. `max/` and `max\` provide the numeric maximum bridge. `min/` works, `scan(min) … along` works, and the `min\` bridge remains unbuilt.

## Work

1. Give `|` and `&` carrier-directed value emission while preserving their mask behavior and precedence.
2. Emit pointwise numeric maximum and minimum under the ordinary scalar extension, alignment, guard, and lineage laws.
3. Make `|/`, `|\`, `&/`, and `&\` select their operation from the operand carrier.
4. Keep `max/`, `max\`, `min/`, and `min\` as exact numeric bridges. Coordinate the unbuilt `min\` spelling with `todo/12-unbuilt-scans.md`.
5. Refuse mask-number mixtures instead of importing q's broader coercion tower.
6. Apply false and true only to empty mask folds. Apply the existing identityless row failure to empty numeric extrema.
7. Add direct dyad tests, including `1 | 7 | 9 | 8 | 6 | 1 | 9 | 8 | 99 | 1 | 23 | 4 | 5 | 174 | 1 | 2 | 3` → `174`. Add column, fold, scan, empty, mixed-carrier refusal, grouped-fiber, and bridge-equivalence tests.
8. Add Ano, Nihongo, and BQN witnesses. Exercise the resulting behavior through Kore as well as Steel.

## Invariants

- Ano's `|/` is q's `|/`, and Ano's `&/` is q's `&/`, within Ano's mask and numeric carriers.
- `|` and `&` remain one dyad each. The carrier selects the instance.
- Boolean programs keep their current spelling and truth tables.
- Numeric bridge spellings are exact aliases in behavior, including guards and float edge behavior.
- No infinity is introduced as an empty numeric identity.
- `>` remains comparison and `>/` remains rejected.
- Precedence, alignment, lineage, and barrier semantics do not change.
