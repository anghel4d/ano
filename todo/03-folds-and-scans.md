# 03 — folds, scans, Greater/Lesser, and empty results

Status: complete for every non-spatial path. Operator-specific runtime promotion, strict destinations, canonical-zero extended-real `num`, computed-NaN publication sealing, and exact scan-row domain sealing are implemented beside the carrier-directed folds/scans and empty-result law.

The carrier-directed dyads, checked descriptor resolver, exact-left folds and scans, mean/count machines, grouped and along forms, char steps, validity channel, and `scan2` removal are implemented. Runtime arithmetic promotes mask/char to number, Greater/Lesser joins `mask < number < char`, comparison uses code-point/numeric reading with nominal symbols, and storage checks the result against its declared destination. Kore pins both zero-record empty output and exact trace demultiplexing. Demos `029`–`032` returned with left-order, prefix-machine, and explicit-view evidence; `033` remains out only because it also depends on `99`.

The table is deliberately per operator family, not universal subtyping. Arithmetic converts mask bits and chars to numbers and returns number. Greater/Lesser converts upward through `mask < number < char` and returns the joined carrier, so `98 | "a"` is `"b"`. Comparison converts chars to code points, reads masks and numbers numerically, returns mask, and permits symbols only under same-carrier equality or inequality. Unary folds/scans apply their family rule to the operand carrier. The effect border remains strict: number accepts number, mask accepts mask or numeric retraction, char and sym accept only same-carrier SET, and computed NaN refuses before publication. `num` admits finite IEEE values and `±∞`, never NaN or absence, and canonicalizes both IEEE signed zeros to `0`; extrema still declare no empty identity. Scan scatter additionally requires exact row-domain identity after inverse grading, never mere length equality.

## Semantic core: the ruled unseeded left accumulation

For a nonempty ordered source `x₀,…,xₙ` and a homogeneous step `f : A × A → A`:

```text
a₀     = x₀
aₖ₊₁   = f(aₖ, xₖ₊₁)
fold f = aₙ
scan f = [a₀, …, aₙ]
```

This is A13's operational law: Haskell supplies the `foldl1`/`scanl1` vocabulary, LINQ's unseeded `Aggregate` is the closer operational precedent for registered dispatch, and q/kdb+ folds and scans are natively this left recurrence. It is not a permission to reassociate. The vocabulary map lives in `docs/ano-language.md` §12 and §14. `+/` is an unseeded monoid fold, `max/` a semigroup reduce, `+\` is `scanl1`, `avg\` and `#\` are `mapAccumL`, and `along` factors traversal order out of the operator. A seeded fold and a fallible `traverse` live in `06`.

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

- `+`, `*`, mask OR/AND, numeric maximum/minimum, the char extrema, and compatible registered binary reducers use the homogeneous recurrence. A descriptor's step is a BQN dyadic function rather than a glyph, so an instance the backend has no primitive for carries the declaration of its own step; only the char extrema need one today.
- `avg` uses state `(sum,count)` and projects `sum/count` at each prefix. `avg\` is not `scanl1` over a numeric accumulator whose type is the input type. The finish is the state after the last element, so `avg/` accumulates its payload in the machine's order and equals the last prefix of `avg\`; a finish that folded that payload in the backend's order would answer `1/3` where the scan answers `0` over `1, 1e100, ¯1e100`. The count is a length and carries no order of its own.
- count consumes selection presence rather than a numeric payload. Its state starts at `0`, advances by `1` for each admitted/true row and by `0` for each false mask element, and emits the state at each prefix. Thus `#/` returns cardinality and `#\` maps `1,0,1,1` to `1,1,2,3`; an implicit all-true membership stream yields `1,2,…,n`. Empty `#\` is empty. Count is a prefix machine, not a homogeneous binary reducer.
- A registered operation advertises its fold and scan instances explicitly. A callable name is not admitted merely because its arity happens to be two.

The registry may share implementation callbacks between the two descriptor classes, but the planner must retain their input, state, result, empty, and law metadata.

## q/kdb+-inspired Greater and Lesser

Ano adopts q's carrier-directed Greater/Lesser convention over every carrier Ano admits, and it admits three: mask, number, and char.

```text
mask:    a | b = a OR b        a & b = a AND b
numeric: a | b = max(a,b)      a & b = min(a,b)
char:    a | b = greater rune  a & b = lesser rune
```

The char instances follow from A9 by derivation; they are not a separate ruling. The order is the code-point order, so `"sat" | "cow"` is `"sow"`, `"sat" & "cow"` is `"cat"`, `max/ "genie"` is `'n'` and `min/ "genie"` is `'e'`. There is no `||`. Direct mixed Greater/Lesser joins upward through `mask < number < char`; this is the family-specific q convention, not a universal carrier hierarchy and not the arithmetic conversion direction.

The same instance is selected for direct dyads, folds, scans, grouped fibers, and long forms:

| spelling | mask carrier | numeric carrier | char carrier | empty fold |
|---|---|---|---|---|
| `|` | OR | maximum | greater rune | n/a |
| `&` | AND | minimum | lesser rune | n/a |
| `|/` | any | maximum | greatest rune | mask `false`; numeric and char no row |
| `&/` | all | minimum | least rune | mask `true`; numeric and char no row |
| `|\` | running any | running maximum | running greatest rune | empty column |
| `&\` | running all | running minimum | running least rune | empty column |
| `max/`, `max\` | refuse | exact bridge to `|/`, `|\` | exact bridge to `|/`, `|\` | no row / empty column |
| `min/`, `min\` | refuse | exact bridge to `&/`, `&\` | exact bridge to `&/`, `&\` | no row / empty column |

The numeric descriptor declares no extrema identity and never manufactures positive or negative infinity as one. `num` independently admits signed infinities but excludes NaN and absence; value admission does not confer identity. The char order has no greatest or least rune either, so the char extrema declare no identity and no code-point zero stands in for one: the answer on an empty glyph scope is no result row, exactly as it is for the numeric extrema. Bridge spellings share the same resolved operation and therefore the same guards, exceptional-value policy, scalar extension, grouping, lineage, and output behavior.

The backend cannot supply the char step: BQN's `⌈` and `⌊` refuse characters outright. The instances therefore travel through code points — `@` is the null character, `c-@` a character's code point, and `@+n` the character at one — and the whole round trip lives in one declared step, `AnoCharGreater ← {@+(𝕨-@)⌈𝕩-@}` and its `⌊` twin. The direct dyad, the fold's left recurrence, and the scan all name that one step, so the three cannot diverge. No other carrier's step needs a declaration, and none of the others has one.

## Other required instances

| operation | descriptor | empty fold | empty scan |
|---|---|---|---|
| `+` | homogeneous numeric reducer | `0` | empty |
| `*` | homogeneous numeric reducer | `1` | empty |
| `avg` | state `(sum,count)`, projected mean | no row | empty |
| `#` | presence/mask prefix machine; state is cardinality | `0` | empty |
| compatible registered reducer | declared descriptor | declared identity or no row | empty |

Any other operator or registered name is admitted only when its descriptor proves carrier compatibility for the requested form. `>` remains comparison; `>/` remains invalid unless a separately registered reducer explicitly owns a non-conflicting name. The char carrier admits Greater, Lesser and their two bridges and nothing else: `+/`, `*/`, `-/`, `avg/` and a registered numeric reducer all refuse a glyph operand with the same carrier-gate message, while `#/` counts glyphs because count consumes presence whatever the payload is.

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
- [X] DONE — `scan2` is deleted end to end. Remove English and Nihongo keywords, parser/AST/emitter flags, documentation, and hardcoded two-axis lowering. A two-axis prefix operation is composition of ordinary scans along explicitly declared axes, not a fixed-rank primitive.

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
- [X] DONE — Kore OUTPUTS: a suppressed identityless result emits no tag, record, group, seam, or fabricated scalar.
- [X] DONE — Expectations: compare against the empty-result representation, not the backend placeholder.
- [X] DONE — The empty-result path introduces no option carrier, `none` marker, exception, NaN, infinity, or display-only special case; it remains distinct from the independently admitted extended-real `num` values.

Forms with a declared empty result remain real scalar results on empty input: `+/ → 0`, `*/ → 1`, `#/ → 0`, mask `|/ → false`, and mask `&/ → true`. For `#/`, `0` is the count machine's registered empty result, not evidence that `#` is a homogeneous binary reducer.

## Implementation order

1. [X] DONE — Add carrier-resolved operation descriptors and one fold/scan head resolver.
2. [X] DONE — Make direct `|` and `&` dispatch on the checked operand carrier while preserving mask precedence and behavior.
3. [X] DONE — Lower homogeneous folds/scans by the exact recurrence above; retain declared order in the plan.
4. [X] DONE — Lower `avg` and count through explicit state and projection; count consumes the checked selection-presence stream, not arbitrary numeric payloads.
5. [X] DONE — Add `min\`, `avg\`, and `#\`; converge glyph, long, grouped, and along forms.
6. [X] DONE — Apply identity metadata per carrier rather than per glyph. Remove any carrier-blind assumption that `|` or `&` always has an identity.
7. [X] DONE — Repair query guard consumption before binding, labeling, expectations, and display.
8. [X] DONE — Delete `scan2` and replace its documentation with axis-composition semantics; do not retain it as compatibility sugar.
9. [X] DONE — Update Ano/Nihongo grammar tables, registry documentation, BQN explanatory witnesses, and Kore output/inspection behavior. The spec's Greater/Lesser paragraph, precedence entries and permutation table, the keyword atlas's fold/scan table and the `&/`/`|/` sections, and the demo READMEs carry all three carriers; Kore tests pin empty-result suppression and exact trace-channel separation.
10. [X] DONE — Give `fold/ rel@row` the validity channel every other identityless fold has. It applies its helper once per row, so an empty fiber used to abort the whole program on the backend's missing identity instead of dropping one row; A12 rules the answer is no result row, and the fold manufactures neither q's negative-infinity result nor Haskell's exception on `maximum []`. This remains the empty-fold law even though `num` admits signed infinity as an ordinary value.
11. [X] DONE — Make `#/ rel'.Comp` the cardinality of the fiber's admitted elements. The Fold branch of the normalizer did not presence-wrap where the Scan and ScanAlong branches did, so it emitted a sum of the payload; the wrap is now pushed onto the hopped component, which keeps the fiber shape the grouped lowering needs.
12. [X] DONE — Reserve the `anoRelStage<n>` staging family in the registry's name gate. It is a generated family rather than one fixed name, so it is reserved by its stem beside the twelve fixed emitter identifiers; a world column that mangled onto a member would sit between a staged relationship write and its publication.

## Acceptance matrix

Positive tests must include:

- [X] DONE — direct scalar and column numeric Greater/Lesser, including the chain `1 | 7 | 9 | 8 | 6 | 1 | 9 | 8 | 99 | 1 | 23 | 4 | 5 | 174 | 1 | 2 | 3 → 174`;
- [X] DONE — unchanged mask truth tables and precedence;
- [X] DONE — `|/`, `|\`, `&/`, and `&\` on masks, numbers, and glyphs, including `"sat" | "cow" → "sow"`, `"sat" & "cow" → "cat"`, `max/ "genie" → 'n'`, and `min/ "genie" → 'e'`;
- [X] DONE — exact equivalence of `max`/`min` bridges with the `|`/`&` instances on both the numeric and the char carrier;
- [X] DONE — `fold(+)`, `scan(+)`, named registered reducers, and exact ordered subtraction/division where an order is declared;
- [X] DONE — grouped/fiber folds and scans with domain and guard parity;
- [X] DONE — running mean prefixes; running count on mixed masks (`1,0,1,1 → 1,1,2,3`), filtered fibers, and an all-true control (`1…n`);
- [X] DONE — empty scans for every instance;
- [X] DONE — folds with declared empty results, including homogeneous identities and count's explicit `0` law;
- [X] DONE — empty identityless folds as no result row in bare, labeled, expected-output, assignment, grouped, per-row (`f/ rel@row`), and Kore paths;
- [X] DONE — mixed Greater/Lesser joins over mask-number, number-char, and mask-char; arithmetic mask/char promotion in dyads, folds, scans, and along forms; strict destination checking and NaN-sealed numeric publication.
- [X] DONE — Ano and Nihongo parity, on mixed and char carriers as on the other two.

Refusal tests must include:

- [X] DONE — symbol arithmetic, symbol ordering, and symbol comparison except same-carrier equality/inequality;
- [X] DONE — promoted results written to incompatible number, mask, char, or symbol destinations, each naming result and destination carrier;
- [X] DONE — every non-extremum head on the char carrier, and the internal canonical spellings `charmax`/`charmin` as unknown reducers;
- [X] DONE — `max`/`min` on masks;
- [X] DONE — an operation with no fold or scan descriptor;
- [X] DONE — an unordered or regrouped non-associative fold;
- [X] DONE — a registered identity whose carrier does not match the reducer result;
- [X] DONE — a stale or incompatible registered operation handle;
- [X] DONE — `scan2` as an unknown removed keyword;
- [X] DONE — any attempt to observe the guarded backend placeholder.

Property tests should compare exact left recurrence with the emitted backend for arbitrary nonempty vectors, verify scan length preservation, verify the last scan element equals the fold result for homogeneous instances, verify the last emitted prefix equals the finish result for nonempty mean/count machines, and test bridge equality including float edge cases under the declared numeric policy.

## Numeric stress

Small tidy numbers cannot tell the ruled left accumulation from the backend's own, which is how a real regression in the mean's finish survived until a reviewer probed it by hand. Three demo units exercise the arithmetic where accumulation order and carrier width actually show, each pinning the fold, the scan, and the law that the fold is the last element of its scan.

- `139` — `1, 1e100, ¯1e100`, the canonical case: the left recurrence answers `0` where a right fold answers `1`, and the mean agrees with the sum because its finish accumulates in the machine's order. `avg\` is `1, 5e99, 0`.
- `140` — three scales at once. `1e300, 2e300, ¯3e300, 1` cancels before its trailing `1` arrives, so left answers `1` and right answers `0`. `1e¯320, 1e¯300, ¯1e¯300, 0` loses its subnormal head into a normal neighbour, so left answers exactly `0` and right answers `1e¯320`. `2.5e¯323, 5e¯324, ¯2.5e¯323, 0` lies wholly inside the subnormals, where addition is exact and the two orders therefore agree on `5e¯324`; that unit pins the value, not the order.
- `141` — integers beside fractions, `1, 0.1, 2, 0.2, 3, 0.3`. Nothing is extreme, but every prefix rounds: the left sum is `6.6000000000000005` against a right fold's `6.6` and the left product is `0.036000000000000004` against `0.036`.

One harness limit is worth writing down rather than working around: `--! out` and `--! expect` compare numbers within `1e¯9` absolute, so below that magnitude they cannot tell a subnormal from `0`. The `140` claims at those scales are therefore pinned as exact `==` against a declared column, which compares inside the backend, and the order claims in `141` are pinned by the fold-equals-last-prefix law for the same reason. Every value in the three units was computed in cbqn and read back, never derived from reasoning about floating point.

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
CharGreater ← {@+(𝕨-@)⌈𝕩-@}
! "sow" ≡ "sat" CharGreater¨ "cow"
! 'n' ≡ CharGreater˜´ ⌽ "genie"
! "ggnnn" ≡ CharGreater` "genie"
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
"sow"~"sat"|"cow"
"cat"~"sat"&"cow"
"n"~|/"genie"
"e"~&/"genie"
anoMax:{ $[0=count x;0#0j;enlist |/x] };
0=count anoMax 0#0j
```

The q block is cited for the Greater/Lesser convention, not for a universal coercion or null policy. Ano now agrees that `98 | "a"` lifts to char and yields `"b"`, while arithmetic lifts the char toward number. q's `max 0N 5 0N 1 3` ignores nulls and `max 0N 0N` yields `-0W`; Ano has no null or NaN value, and A12 answers the distinct empty-scope case with no result row.

## Demo decommission and rewrite ownership

Demos `028`–`032` have returned as complete units: dynamic/bare selection under folds, a non-associative registered left fold and scan, homogeneous and prefix-machine scans, explicit `along` order, and mean/count empty laws. Demo `033` also depends on the spatial contract in `99` and remains quarantined until that task lands.

Decommission each number as one unit across all language twins, registry fixtures, expected output, manifests, README counts, and test claims. New demonstrations must not present a closed syntax whitelist as the operation semantics, require associativity for exact left execution, teach only bridge names while omitting Greater/Lesser, or use `0` as evidence for an empty identityless fold.

Still-correct controls `011`, `012`, `038`, `039`, and `040` remain active. Do not decommission them merely because they use mask folds/scans or a registered reducer spelling.

New units this task added, each a full unit with its registry, expectations, Nihongo twin, and explanatory BQN: `139`, `140` and `141` under `3-fold-scan` are the numeric stress above, `142` under `3-fold-scan` is Greater and Lesser over glyphs with their bridges and the empty glyph scope, and `143` under `8-gamma` is the per-row fold over a fiber column with two empty fibers.

## Completion gate

- [X] DONE — One semantic descriptor table governs direct dyads, glyph folds/scans, long forms, grouped forms, and along forms.
- [X] DONE — Ordered semantics exactly match the recurrence in this file. Every scan holds it natively; every fold renders it from its checked descriptor, seeded only where the carrier declares an identity.
- [X] DONE — Running mean and count are represented honestly as stateful prefix machines.
- [X] DONE — `|` and `&` have carrier-directed q-style behavior over all three carriers and the ruled mixed-carrier join; arithmetic, comparison, folds, and scans share their exact operator-family conversions.
- [X] DONE — Empty identityless queries cannot expose a placeholder: the validity channel guards every rendered fold that declares no identity, in the scoped-global, grouped and per-row forms alike, and no form manufactures ±∞ or a code-point zero merely to answer emptiness.
- [X] DONE — `scan2` no longer exists in code, grammar, docs, or active demos.
- [X] DONE — Steel, Ano, Nihongo, explanatory BQN, and Kore output/trace boundaries agree on same-carrier and promoted positive/refusal cases, strict destinations, and extended-real publication.
