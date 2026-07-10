# 02 — registry types and hop integrity (the s10 bundle)

Author's framing (2026-07-11, binding): "of maximal, critical importance… the first genuine advancement / change made to the core design of the language since the completion of the initial spec." The ano surface stays largely unchanged; this formalizes and corrects the registry side, hitherto improvisational. Lineage stance, recorded: this is q/kdb+ for realtime world simulations; ideas come from databases, HPC, big data, fintech — not from consumer ECS implementations. Forget FLECS.

Coordinate with TODO.md item 3 (REGFIX item 6, the widened entry taxonomy) — this task absorbs its column-typing half; do not fork two taxonomies.

## Findings, verified in code (2026-07-11)

- The functional rel hop emits `((0⌈rel)⊏comp)` — a positional gather into CURRENT row space; the guard only masks the -1 sentinel (src/emit.c:704, emitHop). After any despawn shifts rows, both reported problems are real and latent: PROBLEM 1 (a stored id below the new row count silently gathers the wrong entity) and PROBLEM 2 (a stored id at or beyond it is a hard BQN index-out-of-range fault). No corpus demo hops a functional rel after a despawn, which is the only reason both suites are green.
- The set-hop image is correct: it resolves by membership against the stable-id column (src/emit.c:155 idCol, :976), the ex12 law. But idCol's fallback when no id/keys column is registered is `(↕anoN)` computed at USE time — reminted after a despawn, so the fallback is unsound post-despawn too.
- Spawn fill today (src/emit.c:1350-1420): the proto-named column gets 1; other columns get their registered `default`, else the type zero; rel columns already fill ¯1. The fallback-defaults ruling below is therefore half-implemented.
- `default <col> <v>` binds to a COLUMN and applies to every spawn in the registry regardless of proto. Nothing "knows" Ghost; a ghost is one bit plus the global defaults. The proto is the missing noun.
- Magic name roles the emitter guesses today: `id`, `keys`, `parent`, `proto`, `pos` (reg_role); registry.c already carries a `role` kind to rebind them (`role pos 位置`). The role machinery is the seed the declared tags grow from.

## Current kind inventory (tabulate; the improvisational baseline)

| kind | declares | value shape | governs |
|---|---|---|---|
| n | row count | int | row space |
| col | data column | num / bool / sym / vec | masks, value effects, folds |
| field | lattice field | per-cell values | Tier-1 space; never row-filtered |
| rel | functional relationship | one id per row, -1 = none | dot hop, 0≤ guard, left-join-null |
| srel | set-valued relationship | fibers, `\|`-separated | set hop `'`, image, γ |
| inv | inverse of a functional rel | name + source | γ over preimages |
| alias | named selection | — | `^name` / bare resolve |
| bind | constant | num / mask / vec / point / entity | scopes, along, at |
| fn | host function (BQN dfn) | body | reducers, value calls |
| default | spawn fill for one column | scalar | spawn |
| role | rebind a magic role name | role + name | emitter conventions |
| ja / as | name bridge | from → to | surface naming, mangler |

Plus the unregistered magic: id/keys/parent/proto/pos resolved by name. Option C replaces the id/keys magic with a declared kind.

## Rulings (author, 2026-07-11)

- SOLUTION 2b — generational tombstoning (ano-ecs.md §2: the ID carries a generation; §12: kills bump gen, slot reuse only at tick seal) — is CONFIRMED as the semantics. Review verdict, for the record: it is also the computationally elegant choice — O(1) staleness compare, no information destruction (2a's backtrace loses data and costs a reverse scan per kill), and it composes with §5 left-join-null so the surface algebra never grows a fault path. Pick the generation width with wraparound in mind. The author's diagnostic — `RELATION <col> <origin> -> <sink> IS DEAD !` — is observability, never semantics: a kore/debug surface hooked on the hop's found-guard (with a per-tick structural trace, rows before/after and spawned/killed counts, in the same breath); the algebra stays a silent mask-clear.
- SOLUTION 1 is not an open question — it is SUBSUMED. 2b already is mark-and-defer: the gen bump plus presence clear IS the mark; free-list reuse at tick seal IS the deferred reap. The remaining knob is reap ownership and timing, exposed as a registry option (reap at seal vs host-owned). The mask-level meaning of `~` never changes; only storage reclamation is policy. Open sub-question to surface at implementation: the option's granularity (world vs archetype vs column — the author wrote "per-column registerable type or override").
- SOLUTION 3 goes from half-real to full-real, via option C below. Declaring is always optional; sane defaults; overloads constrain or expand the admitted operations on type-theoretic principles (TAPL framing).
- Fallback type defaults, ruled yes: spawn fill is a three-layer lookup — proto value → registry `default` → the type's zero (num 0, bool 0, sym "", rel ¯1). Record in the spec. The rel case already behaves as None under left-join-null. The SURFACE spelling of None stays open (`/` collides with fold-marker and replicate; candidates: `none`, no surface literal at all) — Open Questions entry, author resolves.
- A proto (registered archetype) is wanted. Name open: author floated `def`; collision warning to surface — `def` is the program-side keyword for derived columns and standing rules, and the pun would cross the registry/program boundary; alternatives `proto`, `arch`, `kind`. Syntax note: protos need named fields (`proto Marine soldier=1 hp=100`), the registry's first row-oriented named-value construct — today all registry data is positional column vectors.

## Option C (ruled preferred): unique + keyed rels

```
unique id 0 1 2 3 4 5 6 7 8 9    -- a column with an algebraic constraint; no 'key' keyword
rel id mentor -1 0 0 3 4 5 6 7   -- relational column mentor, keyed to id
rel leader 1 1 2 -1 -1 2 1 -1    -- relational column, implicitly keyed to the default: idx
```

Design notes from review, to carry into implementation:

- Uniqueness is injectivity. An injective column is invertible on its image, and invertibility is precisely the precondition a keyed hop needs: the keyed hop is `rel ; unique⁻¹`. This is the formal content of "algebraic, number theory" — the type IS the license for the operation.
- `unique` subsumes the id/keys name magic: it declares what reg_role currently guesses. Implied semantics: load-time pairwise-distinct check; spawn mints fresh (1+max); `default` on a unique column is a load error; assignment to it by effects is a compile error (keys are minted, not written).
- Parse is LL(1)-clean: rel/srel data is always numeric-or-`|`, so two names before the data means "first is the key column, second the declared name." Reads as type-annotation-first (`rel id mentor` ≈ mentor : rel over id), beside inv's existing name-then-source shape.
- Composition: `srel <keycol> <name> …` the same way; the inv of a keyed rel is keyed automatically.
- Migration is zero-churn: every existing `rel name …` stays keyed-to-idx.
- Open sub-question: when several unique columns exist, which one mints on spawn (first declared, or an explicit marker) — surface, don't decide.
- Steel note: `keyed` becomes a column tag in the C-struct ABI; generations stay orthogonal (gen protects slots within a run; keys protect identity across slots, saves, hosts).

## Types on algebra, not contents (direction, confirmed)

A column keeps ONE carrier (num/bool/sym/vec) and accumulates constraint evidence: unique (injective), ordered (grade/max admitted), monoid(+,0) (fold with identity), keyed(id), quantized (ano-time's BIND_QUANTIZE). Tags gate which operators the compiler admits; several tags coexist because they are stateless lawful predicates with set-intersection semantics — not inheritance. Lineage to cite in the spec entry: qualified types / typeclasses (Wadler–Blott 1989, "How to make ad-hoc polymorphism less ad hoc"; one type, many instances: Num, Ord, Monoid), Rust traits, Agda/Lean algebraic hierarchies (one carrier, many lawful structures, laws proved), TAPL's bounded quantification for the subtype flavor; `unique` sits at the refinement-types end (Liquid Haskell: a predicate on values enforced at boundaries). The fold contract already IS this: `+/` demands a monoid, `max/` a semigroup whose empty scope fails the row.

## Work items

1. Fix the functional hop: resolve through the stable-id column (`idCol ⊐ rel` with a found-guard folded into the left-join-null path, or equivalent), and make the no-id fallback sound — materialize the row iota ONCE at fixture time as a hidden column filtered through despawns, or refuse structural effects in worlds with no unique column. Surface the choice.
2. New differential twins pinning despawn-then-functional-hop (the fallen-master-after-despawn case): .bqn witness + .ano twin, both suites, both PROBLEM 1 (wrong-entity) and PROBLEM 2 (out-of-range) shapes.
3. `unique` kind and keyed rel/srel: registry.c loader (checks above), emit.c resolution, dumper round-trip, docs.
4. The proto construct (name TBD by author at execution) and the three-layer spawn fill; record the fallback ruling in the spec.
5. The `~` reap-policy registry option (granularity question surfaced first).
6. Spec work: record every ruling above where it executes; keyed-hop section; ano-ecs.md alignment; move the resolved parts out of Open Questions and add the new opens (None spelling, proto name, mint-column choice, option granularity).

## Invariants

- Both suites green; every existing demo's emitted BQN byte-identical (new capability, no changed defaults).
- Registry dump round-trips the new kinds; the case fold stays registry-names-only, values exact-byte (2026-07-10 re-ruling).
- No heavyweight deps; flat C the Steel port can read whole.
