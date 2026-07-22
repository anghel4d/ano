# 18 — empty results are empty output

Status. Steel/Kore implementation and tests are pending.

## Ruling

An expression whose validity guard rejects every row has no result row. This is the ordinary empty-selection law, not a separate null value.

An identity-bearing fold may still produce its identity from empty input: `+/` and `#/` produce 0, mask `|/` produces false, and mask `&/` produces true. An identityless fold cannot manufacture a scalar. Numeric `|/`, numeric `&/`, `max/`, `min/`, `avg/`, and registered reducers without an identity produce no row on empty input. An assignment writes nothing. A bare query prints nothing. No none marker, error, NaN, infinity, or placeholder `0` is introduced.

The output path must use the same empty-result representation and rendering as any query whose predicate has no matches.

## Current implementation

`steel/src/emit.rs::emit_fold` already attaches `ev.g = 0 < count` to identityless scoped folds. Its BQN staging expression uses `0` only as a guarded placeholder so the backend expression remains total. That placeholder is not a language value.

`emit_query` currently discards `Ev.g`, binds `qN` directly to `Ev.v`, and shows it. An identityless empty fold therefore leaks the backend placeholder as the query result. Effects and grouped folds already consume guards and drop rejected rows correctly.

## Work

1. Make query emission apply `Ev.g` before binding, labeling, displaying, and checking `--! out`.
2. Route a false scalar guard through the same empty-result path used by a zero-match selection. Do not add a new none sentinel or display special case.
3. Preserve identity-bearing empty folds as real scalar results.
4. Add bare and labeled query tests for empty `max/`, `min/`, `avg/`, numeric `|/`, numeric `&/`, and one registered identityless reducer.
5. Add controls for empty `+/`, `#/`, mask `|/`, and mask `&/`, plus an ordinary zero-match predicate query. Pin exact output and `--! out` behavior.
6. Exercise the result through Kore's OUTPUTS pane. Empty results must not become a visible zero.

## Invariants

- The guard remains the only validity mechanism. No option carrier or second null system enters the IR.
- The backend placeholder may exist only behind a false guard and never become observable.
- Query, effect, grouped-fold, label, and expectation paths agree on which rows exist.
- Tracing does not change output or post-state.
- Identity-bearing folds retain their existing empty answers.
