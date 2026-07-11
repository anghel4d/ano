# DONE

# 02 — registry types and hop integrity (the s10 bundle)

Author's framing (2026-07-11, binding): "of maximal, critical importance… the first genuine advancement / change made to the core design of the language since the completion of the initial spec." The ano surface stays largely unchanged. This formalizes and corrects the registry side, hitherto improvisational. Lineage stance, recorded: this is q/kdb+ for realtime world simulations. Ideas come from databases, HPC, big data, fintech, not consumer ECS implementations. Forget FLECS.

Coordinate with TODO.md item 3 (REGFIX item 6, the widened entry taxonomy). This task absorbs its column-typing half, so do not fork two taxonomies.

## Findings, verified in code (2026-07-11)

- The functional rel hop emits `((0⌈rel)⊏comp)`, a positional gather into CURRENT row space. The guard only masks the -1 sentinel (src/emit.c:704, emitHop). After any despawn shifts rows, both reported problems are real and latent: PROBLEM 1 (a stored id below the new row count silently gathers the wrong entity) and PROBLEM 2 (a stored id at or beyond it is a hard BQN index-out-of-range fault). No corpus demo hops a functional rel after a despawn, which is the only reason both suites are green.
- The set-hop image is correct: it resolves by membership against the stable-id column (src/emit.c:155 idCol, :976), the ex12 law. But idCol's fallback when no id/keys column is registered is `(↕anoN)` computed at USE time, reminted after a despawn, so the fallback is unsound post-despawn too.
- Spawn fill today (src/emit.c:1350-1420): the proto-named column gets 1, other columns get their registered `default`, else the type zero, and rel columns already fill ¯1. The fallback-defaults ruling below is therefore half-implemented.
- `default <col> <v>` binds to a COLUMN and applies to every spawn in the registry regardless of proto. Nothing "knows" Ghost. A ghost is one bit plus the global defaults. The proto is the missing noun.
- Magic name roles the emitter guesses today: `id`, `keys`, `parent`, `proto`, `pos` (reg_role). registry.c already carries a `role` kind to rebind them (`role pos 位置`). The role machinery is the seed the declared tags grow from.

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

- SOLUTION 2b, generational tombstoning (ano-ecs.md §2: the ID carries a generation; §12: kills bump gen, slot reuse only at tick seal), is CONFIRMED as the semantics. Review verdict, for the record: it is also the computationally elegant choice. The staleness compare is O(1), no information is destroyed (2a's backtrace loses data and costs a reverse scan per kill), and it composes with §5 left-join-null so the surface algebra never grows a fault path. Pick the generation width with wraparound in mind. The author's diagnostic, `RELATION <col> <origin> -> <sink> IS DEAD !`, is observability, never semantics: a kore/debug surface hooked on the hop's found-guard, alongside a per-tick structural trace (rows before/after, spawned/killed counts). The algebra stays a silent mask-clear.
- SOLUTION 1 is not open, it is SUBSUMED. 2b already is mark-and-defer: the gen bump plus presence clear IS the mark, and free-list reuse at tick seal IS the deferred reap. The remaining knob is reap ownership and timing, exposed as a registry option (reap at seal vs host-owned). The mask-level meaning of `~` never changes. Only storage reclamation is policy. Open sub-question to surface at implementation: the option's granularity, world vs archetype vs column (the author wrote "per-column registerable type or override").
- SOLUTION 3 goes from half-real to full-real, via option C below. Declaring is always optional, defaults are sane, and overloads constrain or expand the admitted operations on type-theoretic principles (TAPL framing).
- Fallback type defaults, ruled yes: spawn fill is a three-layer lookup, proto value → registry `default` → the type's zero (num 0, bool 0, sym "", rel ¯1). Record in the spec. The rel case already behaves as None under left-join-null. The surface spelling is ruled closed (author, 2026-07-11): not an open question — kore already renders a ¯1 rel as `/`, but that is a kore rendering choice, never a language feature; if a surface literal ever lands it is `none` or `null`. No Open Questions entry.
- A proto (registered archetype) is wanted. Name ruled (author, 2026-07-11): vocabulary re-used across the registry/program boundary with different meanings is fine, so the `def`-vs-program-`def` collision warning is dismissed; the name is `def` if it collides with nothing inside the registry itself (check the kind inventory — `default` is the near neighbor), otherwise `kind`. Syntax note: protos need named fields (`proto Marine soldier=1 hp=100`), the registry's first row-oriented named-value construct. All registry data today is positional column vectors.

## Option C (ruled preferred): unique + keyed rels

```
unique id 0 1 2 3 4 5 6 7 8 9    -- a column with an algebraic constraint; no 'key' keyword
rel id mentor -1 0 0 3 4 5 6 7   -- relational column mentor, keyed to id
rel leader 1 1 2 -1 -1 2 1 -1    -- relational column, implicitly keyed to the default: idx
```

Design notes from review, to carry into implementation:

- Uniqueness is injectivity. An injective column is invertible on its image, and invertibility is precisely what a keyed hop needs: the keyed hop is `rel ; unique⁻¹`. This is the formal content of "algebraic, number theory": the type IS the license for the operation.
- `unique` subsumes the id/keys name magic: it declares what reg_role currently guesses. Implied semantics: load-time pairwise-distinct check, spawn mints fresh (1+max), `default` on a unique column is a load error, assignment to it by effects is a compile error (keys are minted, not written).
- Parse is LL(1)-clean: rel/srel data is always numeric-or-`|`, so two names before the data means the first is the key column and the second the declared name. Reads type-annotation-first (`rel id mentor` ≈ mentor : rel over id), beside inv's existing name-then-source shape.
- Composition: `srel <keycol> <name> …` the same way. The inv of a keyed rel is keyed automatically.
- Migration is zero-churn: every existing `rel name …` stays keyed-to-idx.
- The several-uniques mint question is ruled void (author, 2026-07-11): `unique` means exactly one thing, a comptime (and runtime) enforcement that every element of the column is distinct. That is all — minting is not a `unique` semantic, so nothing competes. Consequence for the bullet above: the pairwise-distinct check is the ruled content; the spawn-mint, default-refusal, and write-refusal behaviors remain design proposals to surface at execution, not part of the ruling.
- Steel note: `keyed` becomes a column tag in the C-struct ABI. Generations stay orthogonal: gen protects slots within a run, keys protect identity across slots, saves, hosts.

## Types on algebra, not contents (direction, confirmed)

A column keeps ONE carrier (num/bool/sym/vec) and accumulates constraint evidence: unique (injective), ordered (grade/max admitted), monoid(+,0) (fold with identity), keyed(id), quantized (ano-time's BIND_QUANTIZE). Tags gate which operators the compiler admits. Several tags coexist because they are stateless lawful predicates with set-intersection semantics, not inheritance. Lineage to cite in the spec entry: qualified types / typeclasses (Wadler–Blott 1989, "How to make ad-hoc polymorphism less ad hoc"; one type, many instances: Num, Ord, Monoid), Rust traits, Agda/Lean algebraic hierarchies (one carrier, many lawful structures, laws proved), TAPL's bounded quantification for the subtype flavor. `unique` sits at the refinement-types end (Liquid Haskell: a predicate on values enforced at boundaries). The fold contract already IS this: `+/` demands a monoid, `max/` a semigroup whose empty scope fails the row.

## Work items

1. Fix the functional hop: resolve through the stable-id column (`idCol ⊐ rel` with a found-guard folded into the left-join-null path, or equivalent), and make the no-id fallback sound. Either materialize the row iota ONCE at fixture time as a hidden column filtered through despawns, or refuse structural effects in worlds with no unique column. Surface the choice.
2. New differential twins pinning despawn-then-functional-hop (the fallen-master-after-despawn case): .bqn witness + .ano twin, both suites, both the PROBLEM 1 (wrong-entity) and PROBLEM 2 (out-of-range) shapes.
3. `unique` kind and keyed rel/srel: registry.c loader (checks above), emit.c resolution, dumper round-trip, docs.
4. The proto construct (name per the 2026-07-11 ruling: `def` unless it collides inside the registry, else `kind` — run the collision check at execution) and the three-layer spawn fill. Record the fallback ruling in the spec.
5. The `~` reap-policy registry option (granularity question surfaced first).
6. Spec work: record every ruling above where it executes, add the keyed-hop section, align ano-ecs.md, move the resolved parts out of Open Questions, and add the one remaining open (reap-option granularity). None spelling, proto name, and the mint-column question are ruled (2026-07-11) and recorded above — the spec states those as rulings, not opens.

## Invariants

- Both suites green. Every existing demo's emitted BQN byte-identical (new capability, no changed defaults).
- Registry dump round-trips the new kinds. The case fold stays registry-names-only, values exact-byte (2026-07-10 re-ruling).
- No heavyweight deps. Flat C the Steel port can read whole.
