# 03 — folds, scans, Greater/Lesser, and empty results

Status: consolidated implementation task. This replaces the separate scan-parity, Greater/Lesser, and empty-output files because all three depend on one carrier-directed accumulator contract and one validity path.

## Semantic core: the ruled unseeded left accumulation

For a nonempty ordered source `x₀,…,xₙ` and a homogeneous step `f : A × A → A`:

```text
a₀     = x₀
aₖ₊₁   = f(aₖ, xₖ₊₁)
fold f = aₙ
scan f = [a₀, …, aₙ]
```

This is A13's operational law: Haskell supplies the `foldl1`/`scanl1` vocabulary, LINQ's unseeded `Aggregate` is the closer operational precedent for registered dispatch, and q/kdb+ folds and scans are natively this left recurrence. It is not a permission to reassociate.

- Exact left-to-right execution admits any type-compatible step when the source has a declared order.
- Regrouping while preserving order requires associativity.
- Execution that may reorder values requires associativity and commutativity.
- For a homogeneous reducer, an identity is relevant to empty input and seeded execution; it is not required for a nonempty unseeded fold.
- An empty unseeded scan is an empty column.
- An empty unseeded fold returns its registered identity or registered empty result when the registry declares one and otherwise has no result row (A12, A13). Count declares `0`; mean declares none and yields no row.
- A13 already provides for a possible explicit seeded form; that surface is separate. This task does not silently introduce one, and an unseeded scan never emits an extra seed row.

Subtraction, division, and a compatible registered non-associative step are therefore legal only for an exact ordered traversal. They must be refused when the source has no order or when the chosen lowering would regroup.

## Two operation descriptors in the implementation

A homogeneous reducer and a stateful aggregate need different descriptor metadata. This is implementation architecture serving A13's registry-proved dispatch, not an additional ruling.

```rust
struct ReducerDesc {
    input: CarrierId,
    output: CarrierId,             // equal to input for an unseeded homogeneous reducer
    step: RegisteredBinaryOpId,    // A × A → A
    identity: Option<TypedValue>,
    associativity: LawStatus,
    commutativity: LawStatus,
}

struct PrefixMachineDesc {
    input: CarrierId,              // A
    state: CarrierId,              // S
    output: CarrierId,             // B
    start: PrefixStart,            // from first input or explicit empty state
    step: RegisteredStateOpId,     // S × A → S
    emit: RegisteredProjectionId,  // S → B, applied for each prefix
    finish: RegisteredProjectionId,// S → B, applied for a fold result
    empty_fold: EmptyFoldLaw,
}
```

The concrete Rust representation may differ. What the planner must retain:

- `+`, `*`, mask OR/AND, numeric maximum/minimum, and compatible registered binary reducers use the homogeneous recurrence.
- `avg` uses state `(sum,count)` and projects `sum/count` at each prefix. `avg\` is not `scanl1` over a numeric accumulator whose type is the input type.
- count consumes selection presence rather than a numeric payload. Its state starts at `0`, advances by `1` for each admitted/true row and by `0` for each false mask element, and emits the state at each prefix. Thus `#/` returns cardinality and `#\` maps `1,0,1,1` to `1,1,2,3`; an implicit all-true membership stream yields `1,2,…,n`. Empty `#\` is empty. Count is a prefix machine, not a homogeneous binary reducer.
- A registered operation advertises its fold and scan instances explicitly. A callable name is not admitted merely because its arity happens to be two.

The registry may share implementation callbacks between the two descriptor classes, but the planner must retain their input, state, result, empty, and law metadata.

## q/kdb+-inspired Greater and Lesser

Ano adopts q's carrier-directed Greater/Lesser convention only over Ano's admitted mask and numeric carriers:

```text
mask:    a | b = a OR b       a & b = a AND b
numeric: a | b = max(a,b)     a & b = min(a,b)
```

There is no mask-number coercion and no `||`. Mixed carriers refuse before lowering. Existing precedence and pointwise alignment rules remain unchanged.

The same instance is selected for direct dyads, folds, scans, grouped fibers, and long forms:

| spelling | mask carrier | numeric carrier | empty fold |
|---|---|---|---|
| `|` | OR | maximum | n/a |
| `&` | AND | minimum | n/a |
| `|/` | any | maximum | mask `false`; numeric no row |
| `&/` | all | minimum | mask `true`; numeric no row |
| `|\` | running any | running maximum | empty column |
| `&\` | running all | running minimum | empty column |
| `max/`, `max\` | refuse | exact numeric bridge to `|/`, `|\` | no row / empty column |
| `min/`, `min\` | refuse | exact numeric bridge to `&/`, `&\` | no row / empty column |

Ano's numeric carrier is finite `float64`; this task must not manufacture positive or negative infinity as an extrema identity. Bridge spellings must share the same resolved operation and therefore the same guards, NaN policy, scalar extension, grouping, lineage, and output behavior.

## Other required instances

| operation | descriptor | empty fold | empty scan |
|---|---|---|---|
| `+` | homogeneous numeric reducer | `0` | empty |
| `*` | homogeneous numeric reducer | `1` | empty |
| `avg` | state `(sum,count)`, projected mean | no row | empty |
| `#` | presence/mask prefix machine; state is cardinality | `0` | empty |
| compatible registered reducer | declared descriptor | declared identity or no row | empty |

Any other operator or registered name is admitted only when its descriptor proves carrier compatibility for the requested form. `>` remains comparison; `>/` remains invalid unless a separately registered reducer explicitly owns a non-conflicting name.

## Grammar and planning convergence

Steel currently maintains divergent parser and emitter whitelists for glyph scans, `scan(f) … along …`, and long folds. Replace them with one process:

1. Parse a fold/scan head as an operator token or a registry name.
2. Resolve it against the requested form (`Fold`, `Scan`, or `ScanAlong`) and the operand carrier.
3. Retrieve the homogeneous reducer or prefix-machine descriptor.
4. Validate order and algebraic requirements for the selected execution strategy.
5. Lower the checked descriptor.

Required surface results:

- [X] DONE — `fold(+)` is valid.
- [X] DONE — Every symbolic and long spelling that denotes the same admitted operation resolves to the same descriptor.
- [X] DONE — `min\` is added as the missing glyph bridge.
- [X] DONE — `avg\` is added from the same state/projector used by `avg/`.
- [X] DONE — `#\` is admitted as running cardinality over the same selection-presence semantics as `#/`. This is settled, not an open surface question: q/kdb+'s scan adverb gives every admitted fold its running form, and `#\` is the same convention already carried by `+\`, `|\`, and `&\`. It increments on admitted/true rows, not merely on every physical input slot. Do not add a second built-in `count/` spelling unless it is an ordinary registered name.
- [X] DONE in code, grammar, and demos; the documentation claim awaits the docs triage — `scan2` is deleted end to end. Remove English and Nihongo keywords, parser/AST/emitter flags, documentation, and hardcoded two-axis lowering. A two-axis prefix operation is composition of ordinary scans along explicitly declared axes, not a fixed-rank primitive.

The glyph and long forms may have different syntax, but they must not have separate semantic operation tables.

## Validity and empty output

An identityless fold already needs a validity guard such as `0 < count`. The backend may use a total placeholder behind a false guard, but that placeholder is not an Ano value.

The query path must consume the guard before every observable operation:

```text
resolve/compute → apply validity guard → bind/label → compare --! out → render
```

A false scalar guard produces the same zero-row result as an ordinary query whose predicate selects nothing.

- [X] DONE — Bare query: print nothing.
- [X] DONE — Labeled query: emit no value row; do not print a label followed by `0`.
- [X] DONE — Assignment: write nothing.
- Kore OUTPUTS: show no fabricated scalar.
- [X] DONE — Expectations: compare against the empty-result representation, not the backend placeholder.
- [X] DONE — No option carrier, `none` marker, exception, NaN, infinity, or display-only special case is introduced.

Forms with a declared empty result remain real scalar results on empty input: `+/ → 0`, `*/ → 1`, `#/ → 0`, mask `|/ → false`, and mask `&/ → true`. For `#/`, `0` is the count machine's registered empty result, not evidence that `#` is a homogeneous binary reducer.

## Implementation order

1. [X] DONE — Add carrier-resolved operation descriptors and one fold/scan head resolver.
2. [X] DONE — Make direct `|` and `&` dispatch on the checked operand carrier while preserving mask precedence and behavior.
3. [X] DONE — Lower homogeneous folds/scans by the exact recurrence above; retain declared order in the plan.
4. [X] DONE — Lower `avg` and count through explicit state and projection; count consumes the checked selection-presence stream, not arbitrary numeric payloads.
5. [X] DONE — Add `min\`, `avg\`, and `#\`; converge glyph, long, grouped, and along forms.
6. [X] DONE — Apply identity metadata per carrier rather than per glyph. Remove any carrier-blind assumption that `|` or `&` always has an identity.
7. [X] DONE — Repair query guard consumption before binding, labeling, expectations, and display.
8. [X] DONE in code and demos; the documentation replacement awaits the docs triage — Delete `scan2` and replace its documentation with axis-composition semantics; do not retain it as compatibility sugar.
9. Update Ano/Nihongo grammar tables, registry documentation, BQN explanatory witnesses, and Kore output/inspection behavior.

## Acceptance matrix

Positive tests must include:

- [X] DONE — direct scalar and column numeric Greater/Lesser, including the chain `1 | 7 | 9 | 8 | 6 | 1 | 9 | 8 | 99 | 1 | 23 | 4 | 5 | 174 | 1 | 2 | 3 → 174`;
- [X] DONE — unchanged mask truth tables and precedence;
- [X] DONE — `|/`, `|\`, `&/`, and `&\` on masks and numbers;
- [X] DONE — exact equivalence of `max`/`min` bridges with the numeric `|`/`&` instances;
- [X] DONE — `fold(+)`, `scan(+)`, named registered reducers, and exact ordered subtraction/division where an order is declared;
- [X] DONE — grouped/fiber folds and scans with domain and guard parity;
- [X] DONE — running mean prefixes; running count on mixed masks (`1,0,1,1 → 1,1,2,3`), filtered fibers, and an all-true control (`1…n`);
- [X] DONE — empty scans for every instance;
- [X] DONE — folds with declared empty results, including homogeneous identities and count's explicit `0` law;
- [X] DONE except the Kore path — empty identityless folds as no result row in bare, labeled, expected-output, assignment, and Kore paths;
- [X] DONE — Ano and Nihongo parity.

Refusal tests must include:

- [X] DONE — mask-number Greater/Lesser mixtures;
- [X] DONE — `max`/`min` on masks;
- [X] DONE — an operation with no fold or scan descriptor;
- an unordered or regrouped non-associative fold;
- a registered identity whose carrier does not match the reducer result;
- a stale or incompatible registered operation handle;
- [X] DONE — `scan2` as an unknown removed keyword;
- [X] DONE — any attempt to observe the guarded backend placeholder.

Property tests should compare exact left recurrence with the emitted backend for arbitrary nonempty vectors, verify scan length preservation, verify the last scan element equals the fold result for homogeneous instances, verify the last emitted prefix equals the finish result for nonempty mean/count machines, and test bridge equality including float edge cases under the declared numeric policy.

## Sentinel

One executable witness for the category, corrected from the demolition ledger to the current contract; explanatory mathematics, not a semantic oracle. The old sentinel witnessed only associative right folds, which cannot distinguish the ruled left recurrence; the corrected one pins the left recurrence on a non-associative step, the running count over presence, and the identityless empty fold. q is the native precedent: its `/` and `\` are exactly this left recurrence.

```bqn
mask ← 0‿1‿0‿1
nums ← 1‿7‿3‿9
! 1 ≡ ∨´ mask
! 9 ≡ ⌈´ nums
! 1‿7‿7‿9 ≡ ⌈` nums
! ¯4 ≡ -˜´ ⌽ 1‿2‿3
! 1‿¯1‿¯4 ≡ -` 1‿2‿3
! 1‿1‿2‿3 ≡ +` 1‿0‿1‿1
NoRow ← {0=≠𝕩 ? ⟨⟩ ; ⟨⌈´𝕩⟩}
! 0 = ≠ NoRow ⟨⟩
! 0 ≡ +´ ⟨⟩
```

```q
mask:0101b;
nums:1 7 3 9;
1b~|/mask
9=|/nums
1 7 7 9~|\nums
(-4)~-/1 2 3
(1 -1 -4)~-\1 2 3
1 1 2 3~+\1 0 1 1
anoMax:{ $[0=count x;0#0j;enlist |/x] };
0=count anoMax 0#0j
```

## Demo decommission and rewrite ownership

The global demolition ledger assigns primary rewrite ownership here to `029`, `030`, `031`, `032`, and `033`. Demo `028` is owned by `02` and also depends on this task. Demo `033` also depends on the spatial contract in `99`; it must not be reactivated until both tasks land.

Decommission each number as one unit across all language twins, registry fixtures, expected output, manifests, README counts, and test claims. New demonstrations must not present a closed syntax whitelist as the operation semantics, require associativity for exact left execution, teach only bridge names while omitting Greater/Lesser, or use `0` as evidence for an empty identityless fold.

Still-correct controls `011`, `012`, `038`, `039`, and `040` remain active. Do not decommission them merely because they use mask folds/scans or a registered reducer spelling.

## Completion gate

- [X] DONE — One semantic descriptor table governs direct dyads, glyph folds/scans, long forms, grouped forms, and along forms.
- [X] DONE — Ordered semantics exactly match the recurrence in this file.
- [X] DONE — Running mean and count are represented honestly as stateful prefix machines.
- [X] DONE — `|` and `&` have carrier-directed q-style behavior without coercion.
- [X] DONE — Empty identityless queries cannot expose a placeholder.
- [X] DONE except docs — `scan2` no longer exists in code, grammar, docs, or active demos.
- Steel, Kore, Ano, Nihongo, and explanatory BQN witnesses agree on all positive and refusal cases.
