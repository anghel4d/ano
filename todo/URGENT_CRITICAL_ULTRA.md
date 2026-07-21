# URGENT CRITICAL ULTRA — decommission superseded demos

Status: urgent demolition task. This pass does not write replacements, repair old demonstrations, or silently reinterpret their success. It removes every listed number from the active proof and acceptance surface until that number is rewritten from first principles against the current contract.

Decommissioning means that the numbered demo, every language twin under that number, its registry fixture, its golden output, and every README or test claim derived from it cease to count as evidence. The old artifact may remain only as explicitly quarantined historical material; passing Steel or CBQN is not a reprieve when the demo proves the wrong denotation.

## Tally

Total: 64 demo numbers.

Compact range: `005`, `019`, `028–037`, `044`, `046–047`, `050–053`, `055–078`, `089–091`, `093`, `099`, `101–110`, `114–119`.

Exhaustive tally: `005`, `019`, `028`, `029`, `030`, `031`, `032`, `033`, `034`, `035`, `036`, `037`, `044`, `046`, `047`, `050`, `051`, `052`, `053`, `055`, `056`, `057`, `058`, `059`, `060`, `061`, `062`, `063`, `064`, `065`, `066`, `067`, `068`, `069`, `070`, `071`, `072`, `073`, `074`, `075`, `076`, `077`, `078`, `089`, `090`, `091`, `093`, `099`, `101`, `102`, `103`, `104`, `105`, `106`, `107`, `108`, `109`, `110`, `114`, `115`, `116`, `117`, `118`, `119`.

The number is the unit of demolition. A listed number includes its English Ano, Nihongo Ano, BQN, registry, expected-output, and documentation siblings even when one sibling is absent or the obsolete claim occurs in only one of them; `072` and the spatial portion of `076` therefore remain in scope despite their unusual artifact mix.

Do not expand this list merely because a still-correct demo uses `&`, `|`, a relationship, a fold, or a scan. In particular, `011`, `012`, `038`, and `039` remain correct mask fold/scan witnesses; `040` remains a correct registered-reducer spelling witness; `008`, `015`, `026`, `027`, and `087` remain correct foundness/dead-link result witnesses. They can acquire stronger diagnostics later without first being declared false.

## Execution order

1. Remove the 64 numbers from every active demo ladder, acceptance manifest, green count, README proof claim, and default proof run.
2. Quarantine or delete their current artifacts as one operation per number; do not leave a BQN twin or registry fixture advertised as authoritative after its Ano sibling is decommissioned.
3. Leave each number vacant and visibly awaiting a total rewrite. Do not patch expected output, rename the old claim, or write a replacement in this task.
4. Preserve the mathematical contracts below as the map for the later rewrite campaign. The sentinels are one witness per broad category, not exhaustive replacement demos and not semantic oracles.

## Rewrite group: live alias overlay

Demos: `005`, `019`, `028`, `071`, `093`, `103`.

Deviation: these demos treat `^name` as a fixed fixture mask, compile it by erasing the sigil, or otherwise fail to prove the live overlay, bare fallback, shadowing, statement snapshot, and independent alias lifetime. A live alias named `Whiterun` does not mutate or destroy the bare `Whiterun` column; deleting the alias reveals the bare binding again.

Let `Γ` be the registry's bare bindings and `A_t` the live alias overlay at statement step `t`. Then `lookup_t(n) = Γ(n)` and `lookup_t(^n) = A_t(n)` when `n ∈ dom(A_t)`, otherwise `lookup_t(^n) = Γ(n)`. Rebinding or deleting an entry of `A_t` never changes `Γ`, and one statement observes one coherent `A_t` snapshot.

BQN sentinel:

```bqn
bare ← 1‿0‿1‿0
live ← 0‿1‿0‿1
Resolve ← {𝕨 ? 𝕩 ; bare}
! live ≡ 1 Resolve live
! bare ≡ 0 Resolve live
! bare ≡ 1‿0‿1‿0
```

q/kdb+ sentinel:

```q
bare:1010b;
live:0101b;
resolve:{[has;target]$[has;target;bare]};
live~resolve[1b;live]
bare~resolve[0b;live]
bare~1010b
```

## Rewrite group: Greater/Lesser, registered accumulators, and empty results

Demos: `028–033`.

Deviation: these demos present an old closed table of reductions and scans, leave ordered non-associative accumulation unresolved, overstate associativity as a condition of ordinary left-to-right execution, teach numeric bridge names without the carrier-directed Greater/Lesser family, or omit identityless empty results because the old harness could not spell a no-row answer. The current contract is registry-driven and preserves the established meanings of `|/`, `|\`, `&/`, `&\`, `+/`, `+\`, and their compatible peers.

For booleans, `a | b = a ∨ b` and `a & b = a ∧ b`; for numbers, `a | b = max(a,b)` and `a & b = min(a,b)`. This is exactly the q/kdb+ Greater/Lesser convention over carriers admitted by Ano, not a claim of general q compatibility; the bridge spellings `max/`, `max\`, `min/`, and `min\` remain during migration. See KX's definitions of [Greater](https://code.kx.com/q/ref/greater/), [Lesser](https://code.kx.com/q/ref/lesser/), [max](https://code.kx.com/q/ref/max/), and [min](https://code.kx.com/q/ref/min/).

For an ordered source `x₀,…,xₙ`, a compatible registered accumulator `f` defines `a₀ = x₀` and `aₖ₊₁ = f(aₖ,xₖ₊₁)`. The fold returns the final accumulator and the scan returns every successive `aₖ`. Exact ordered execution permits any compatible accumulator; regrouping requires associativity; parallel or otherwise order-discarding execution requires associativity and commutativity. An empty fold returns the registered identity when one exists and otherwise produces no result row; an empty scan produces an empty column and needs no identity unless a seeded form explicitly emits the seed.

BQN sentinel:

```bqn
mask ← 0‿1‿0‿1
nums ← 1‿7‿3‿9
! 1 ≡ ∨´ mask
! 9 ≡ ⌈´ nums
! 0‿1‿1‿1 ≡ ∨` mask
! 1‿7‿7‿9 ≡ ⌈` nums
NoId ← {0=≠𝕩 ? ⟨⟩ ; ⟨⌈´𝕩⟩}
! 0 = ≠ NoId ⟨⟩
```

q/kdb+ sentinel:

```q
mask:0101b;
nums:1 7 3 9;
1b~|/mask
9=|/nums
0111b~|\mask
1 7 7 9~|\nums
anoMax:{ $[0=count x;0#0j;enlist |/x] };
0=count anoMax 0#0j
```

## Rewrite group: declared space, habitat, placement, and lineage

Demos: `033–037`, `044`, `046–047`, `050–053`, `055–078`, `089–091`, `093`, `099`, `101–102`, `104–110`, `114–119`.

Deviation: every current spatial demonstration is decommissioned. Together they rely on some combination of an implicit singleton lattice, raw buffer coincidence, inferred frames, implicit axis order, equal-length alignment, `vec` masquerading as a point carrier, a supposedly regenerable space index or placement derived from shape alone, raw `x/y` defaults standing in for a raycast point, undeclared boundaries, one-step success where lineage must survive a later tick, or the retired tier ladder. Even when the arithmetic inside a demo is sound, its advertised spatial proof is not.

A habitat is nominal. For habitats `H` and `K`, `|H| = |K|` does not imply `H = K` and does not authorize alignment. A stored field has type `f : H → V`; a storage layout is an isomorphism `ℓ_H : Fin(n) ≅ H`, not the habitat itself. A query has source domain `X` and explicit partial lineage `λ_H : X ⇀ H`. After selection, an effect destination is `d : S ⇀ H`; compatibility is proved by habitat identity, lineage, placement, or an explicit declared map, never by equal buffer length.

A position component has a declared carrier and frame, `pos : P_pos → Point⟨F⟩`. A lattice has a declared logical domain `D_H`, axes, boundary policy, and chart into `H`; when world-space placement exists it is an explicit map `χ : D_H → Point⟨F⟩`. Two coordinates are entirely valid when the declared destination carrier exposes at least two spatial axes. Legacy `pos = to shape` therefore needs neither a fabricated third coordinate nor a blanket refusal: it is legal exactly when registry declarations determine a compatible two-or-more-axis destination and placement, and otherwise remains ambiguous.

BQN sentinel:

```bqn
ground ← ⟨"Ground", 2‿3, ↕6⟩
mars ← ⟨"Mars", 2‿3, ↕6⟩
SameHabitat ← {(0⊑𝕨) ≡ 0⊑𝕩}
! ¬ ground SameHabitat mars
! (≠2⊑ground) = ≠2⊑mars
```

q/kdb+ sentinel:

```q
ground:(`habitat`shape`values)!(`Ground;2 3;til 6);
mars:(`habitat`shape`values)!(`Mars;2 3;til 6);
sameHabitat:{x`habitat~y`habitat};
not sameHabitat[ground;mars]
(count ground`values)=count mars`values
```

The q sentinel carries habitat metadata explicitly because an unadorned q vector does not encode Ano's nominal habitat. It witnesses the refusal of length-based alignment; it is not a claim that q supplies Ano's spatial type system.

## Rewrite group: predicate and effect diagnostic domains

Demos: this is a cross-cutting obligation for the rewritten relationship and spatial demos, especially `105–109` and `114–115`; it adds no further demolition number by itself because the existing foundness result witnesses listed above do not claim the unresolved trace contract.

Deviation: a backend-wide relationship or fiber report can expose dead links outside the denotational domain of an effect, while filtering predicate diagnostics through a sibling conjunct invents short-circuit order. Rewritten demos must distinguish the two semantic positions and must preserve distinct crossings rather than collapsing them in the trace contract.

Let `X` be the source domain, `p : X → Bool`, `S = {x ∈ X | p(x)} = p⁻¹({true})`, and `i : S ↪ X` the inclusion. Predicate expressions are columns on `X`; sibling masks such as `Enemy` and `Owner.Gold > 10` combine pointwise, so `&` is commutative and does not short-circuit. An effect-side source column `c : X → V` is restricted to `c ∘ i : S → V`, and an effect destination is a partial map `d : S ⇀ H`. RELATION and FIBER reports therefore use `X` in predicates and `S` in effects. Two crossings in two semantic positions remain two trace events; presentation may aggregate them later without changing that contract.

BQN sentinel:

```bqn
x ← ↕4
enemy ← 1‿0‿1‿0
eligible ← 0‿1‿1‿0
dead ← 1‿1‿0‿1
p ← enemy ∧ eligible
! p ≡ eligible ∧ enemy
! 0‿1‿3 ≡ dead/x
! ⟨⟩ ≡ (p∧dead)/x
```

q/kdb+ sentinel:

```q
x:til 4;
enemy:1010b;
eligible:0110b;
dead:1101b;
p:enemy&eligible;
p~eligible&enemy
0 1 3~dead#x
0=count (p&dead)#x
```
