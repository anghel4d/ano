# 04 — diagnostic domains and relationship trace parity

Status: phase-domain reporting, the trace harness, and endpoint-carrier sealing have landed in Steel; the full acceptance matrix and Kore parity remain. The result semantics for foundness and the sentinel are ruled: `-1` is the sole silent no-link sentinel, and every other target is ordinary carrier data judged by the found guard.

Foundness, the phase domains, and the dead-link report are implemented and their tests pass. `DEAD` is spelled as the non-sentinel complement of the hop's own found guard rather than derived separately, so the two cannot partition the targets differently — which is the defect that produced the sign bug, where a resolving negative key was neither found nor reported dead and the row simply vanished. The load boundary refuses a target only for leaving its declared carrier, never for its sign.

The write boundary now lowers as stage, assert, publish at the commit itself, and the assembly reconciles the writes it staged against the writes the emitted text performs, so a write emitted in some other shape is refused rather than left unsealed. The `FIBER` reports are observed again now that the fiber fixture's folds render.

## Semantic domains

Let `X` be a statement's source row domain, let `p : X → Bool` be its final predicate, and let:

```text
S = { x ∈ X | p(x) }
i : S ↪ X
```

Predicate expressions are columns on `X`. In a predicate such as `Enemy & Owner.Gold > 10`, both masks are evaluated on `X` and combined pointwise. `&` is commutative mask conjunction, not sequential short-circuit syntax. A predicate-side relationship or fiber crossing therefore reports over `X`, including crossings in a conjunct whose sibling is false.

Effects operate after selection. An effect-side source column `c : X → V` is restricted along `i`, yielding `c ∘ i : S → V`; an effect destination is a partial map whose source is `S`. Effect-side `RELATION` and `FIBER` diagnostics therefore report only crossings actually evaluated on `S`.

This is one phase rule:

| semantic position | row domain for value | row domain for `RELATION`/`FIBER` trace |
|---|---|---|
| predicate | `X` | `X` |
| effect | `S` | `S` |

Presentation may group equivalent messages, but the semantic trace records uses, not only unique dead targets.

## Distinct crossings remain distinct

The same relationship spelling in predicate and effect denotes two evaluation events in different semantic positions. It may correctly produce two trace records:

1. a predicate-side crossing over `X`;
2. an effect-side crossing over `S`.

Do not deduplicate these at the trace-contract layer. A UI may render an aggregate with a count and the underlying event identities, provided machine-readable trace output remains lossless and deterministic.

Within one expression, common-subexpression elimination may share computation only if it preserves the source-use records required by tracing. Optimizer sharing is not permission to erase a semantic use.

## Foundness and sentinel rules

The following result semantics are already closed and must not be reopened while repairing traces:

- A bare keyed relationship is true exactly when its stored target key resolves to a live target in the current world. Declared-but-dead is false, matching a hop's found guard.
- For a functional relationship, `-1` is the distinguished “no link” sentinel and is diagnostically silent. A non-sentinel target must be a finite value admitted by the declared endpoint carrier: an unkeyed row/index target is an exact integer, while a keyed target follows the declared unique key column’s carrier. The carrier alone decides sign (A11): a `nat` key refuses negatives because `nat` does, while an `int` key admits them. A valid target need not currently resolve to a live row—that is what distinguishes a valid dead link from malformed storage.
- A negative non-sentinel target is not a second silent sentinel and not malformed by sign alone: where the endpoint carrier admits it, it is stored as ordinary data, fails the found guard, and reports `DEAD` at each evaluated crossing (A17). Only carrier-invalid values—fractional or nonfinite row indices and keyed values outside the declared key carrier—refuse during registry loading, default/prototype validation, or the atomic commit that would store them.
- Any valid non-sentinel target that fails the found guard is an actual dead link and produces `DEAD` at each evaluated crossing.
- Do not add `MISSING` or `NOT FOUND` states that duplicate the same semantics.
- Tracing never changes masks, row existence, output, effects, or post-state.

Still-correct foundness/dead-link result witnesses `008`, `015`, `026`, `027`, and `087` remain active. They may gain trace assertions, but they are not false demonstrations merely because the richer phase-domain contract was absent.

## Current gap

Steel's `FIBER` reporting already follows the predicate/effect phase split. Effect-side `RELATION` currently risks reporting against the pre-selection source domain. The fix is not to filter every relationship diagnostic by the final predicate: that would incorrectly narrow predicate-side crossings and invent short-circuit behavior.

The found/dead gate must silence exactly `-1`. Every other negative that the endpoint carrier admits is a valid dead link and reports `DEAD` when evaluated; a gate spelled `0 ≤ target` wrongly silences them. The registry boundary refuses only carrier-invalid targets—fractional or nonfinite row indices and keyed values outside the declared key carrier—never a value merely for being negative.

The planner/emitter must carry the semantic use site and its row domain into tracing. A backend-wide relationship report without that information is insufficient.

## Work

1. [X] DONE — Attach a stable trace-use identifier and semantic phase to every planned relationship/fiber crossing.
2. [X] DONE — Carry the exact query-domain ID and any selection inclusion needed to map a reported source row back to stable entity/field identity.
3. [X] DONE — Keep predicate crossings on `X` even when another conjunct excludes the row.
4. [X] DONE — Restrict effect crossings to `S` before evaluating and reporting the hop.
5. [X] DONE — Preserve multiple use records when one relationship appears in predicate and effect, or appears twice in one statement.
6. [X] DONE except the migration path, which does not exist yet — Seal functional-relationship targets at every introduction path: registry load, defaults/prototypes, spawn fill, ordinary assignment/update, migration, and save/reload. Admit `-1` and any finite value the declared endpoint carrier admits; require exact integers for unkeyed row/index targets and the declared unique-key carrier for keyed targets, with sign decided only by the carrier (A11). A refused write leaves the exact pre-state.
7. [X] DONE — Keep `-1` silent while reporting every evaluated valid non-sentinel dead key. Set-valued relationships represent no links by an empty fiber; do not introduce `-1` as a silent member convention there.
8. [X] DONE — Define deterministic trace order independent of backend chunking. Prefer source/use order and stable row identity; do not use hash-map iteration order.
9. Keep presentation aggregation downstream of the machine trace. Aggregation must expose a count and cannot change exit status, world state, or expected-output semantics.
10. [X] DONE — Ensure grouped/fiber operations report the source row and group/fiber identity without widening beyond the operation's semantic domain.
11. Document the phase table in the trace/diagnostic reference and make Ano/Nihongo examples semantically identical.

## Sentinel

One executable witness for the category, corrected from the demolition ledger to the current contract; explanatory mathematics, not a semantic oracle. The old sentinel had no relationship targets at all; the corrected one pins `-1` silent, a carrier-admitted `-2` as a loud dead link, predicate reports over `X`, effect reports over `S`, and pointwise commutative conjunction.

```bqn
x ← ↕4
p ← 1‿0‿1‿0
t ← 2‿¯1‿9‿¯2
found ← ((¯1≠t)∧(0≤t))∧(t<4)
dead ← (¯1≠t)∧¬found
! 0‿0‿1‿1 ≡ dead
! 2‿3 ≡ dead/x
! ⟨2⟩ ≡ (p∧dead)/x
! (p∧dead) ≡ (dead∧p)
```

```q
x:til 4;
p:1010b;
t:2 -1 9 -2;
found:((t<>-1)&t>=0)&t<4;
dead:(t<>-1)&not found;
0011b~dead
2 3~where dead
(enlist 2)~where p&dead
(p&dead)~dead&p
```

## Test matrix

Add exact trace tests for:

- [X] DONE — a dead relationship in a predicate conjunct whose sibling mask is false: predicate event still appears;
- [X] DONE — the same dead relationship used only in an effect on a row excluded by `p`: no effect event appears;
- [X] DONE — the same relationship used in predicate and effect: two use records with their respective domains;
- [X] DONE — two textual crossings of one relationship in the same phase: two stable use records;
- [X] DONE — `-1` in predicate and effect: no `DEAD` record;
- [X] DONE except migration — `-2` and another negative non-sentinel through a carrier that admits them: admitted at every introduction path and `DEAD` at each evaluated crossing, never silent and never refused by sign;
- [X] DONE except migration — a fractional unkeyed index, a nonfinite computed target, a negative keyed target through a `nat` key, and a keyed target outside its declared key carrier at registry/default/prototype construction, migration, or effect commit: exact refusal with unchanged state;
- [X] DONE — a nonnegative never-existing key and a formerly-live despawned key: both are `DEAD` when evaluated;
- [X] DONE — keyed and index-keyed worlds, including a dead target after despawn;
- [X] DONE — grouped/fiber diagnostics with source and group identity;
- [X] DONE — tracing disabled versus enabled: identical output and post-state;
- [X] DONE — deterministic ordering across repeated runs and any available parallel/backend mode;
- Kore's diagnostic pane and standalone Steel's machine-readable trace.

The relationship/spatial rewrite campaign must cover this cross-cutting contract especially in demo numbers `105–109` and `114–115`. Those numbers are already part of `99`'s spatial demolition set; this task adds no demolition number and must not inflate the global total above 64.

## Completion gate

- [X] DONE — Predicate diagnostics range over `X`; effect diagnostics range over `S`.
- [X] DONE — Pointwise conjunction remains order-independent and does not acquire short-circuit trace behavior.
- [X] DONE — Distinct semantic crossings remain distinct trace events.
- [X] DONE — `-1` is the only silent functional no-link representation; every evaluated valid non-sentinel dead target is `DEAD`, and only carrier-invalid targets refuse.
- [X] DONE — Enabling diagnostics changes neither denotation nor committed state.
- Steel and Kore agree on event identity, domain, order, and presentation for the acceptance matrix.
