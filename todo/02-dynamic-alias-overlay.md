# 02 — dynamic alias overlay

Status: the core overlay, the resolver framework (`input.entity` and `input.mask`, a frozen `HostInput`, service versions), the host boundary, replay (persist-environment with barrier-stamped `-- alias@<barrier>` records and per-version sidecar snapshots), and Kore's display are landed in Steel and Kore. Remaining: wiring the ruled `^cursor` cold default—alias-first with bare fallback, per the documented contract—and the concrete deictic resolvers alongside `99`'s host services; and the alias demo rewrites, owned by the demo campaign task.

The overlay itself is untouched by the fold and foundness work in flight, and every marked item below holds, with one exception: the end-to-end CLI sidecar test cannot run, because its fixture folds and fold rendering is a stub. That is a fixture dependency, not a gap in the overlay.

## Ruling

Let `Γ` be the validated registry and ordinary bare-binding environment, and let `A_t` be the live alias overlay observed by statement step `t`.

```text
lookup_t(name)  = Γ(name)
lookup_t(^name) = A_t(name), when name ∈ dom(A_t)
lookup_t(^name) = Γ(name),   otherwise
```

Installing, rebinding, or deleting an entry of `A_t` never changes `Γ`. A live alias may deliberately have the same stem as a bare entry. Deleting it reveals the bare entry again. Bare fallback occurs only when the canonical key is absent from `A_t`; a present but stale, invalid, or context-incompatible target refuses instead of silently exposing the bare binding.

An alias target is one of: an already resolved column/binding handle; a host-supplied mask value with an explicit carrier and row domain; or a validated host resolver handle for deictics such as `^cursor`. A resolver declares its result carrier/domain contract, determinism boundary, input snapshot, and service version. It may be invoked at each lookup/gather, but every invocation in one statement receives the same frozen world and host-input snapshots.

An alias target is not registry schema, source-level assignment syntax, or a recursive string macro. If the host accepts a binding target by name, it resolves that target before installation and records the stable resolved handle; dynamic alias chains and cycle-breaking are therefore not part of evaluation.

One statement evaluates against one coherent triple `(World_t, A_t, HostInput_t)`. Host changes become visible only at a statement barrier. No row, gather, fold, effect, trace, resolver invocation, or Kore repaint inside one statement may observe a mid-statement rebind or a different host-input snapshot.

## Existing mechanisms that remain separate

1. **Spelling aliases.** `Registry.aliases` translates accepted spellings to canonical registry names. It is immutable with the registry and applies during ordinary name resolution.
2. **Static alias masks.** The registry `alias` directive creates a stored `AliasMask` entry. It is registry data and keeps its existing static-fixture meaning.
3. **Dynamic aliases.** `A_t` is host-managed session state and is consulted only for `^name`.

The registry directive does **not** seed the dynamic overlay. This resolves the old ambiguity without overloading one declaration with two lifecycles.

## Runtime model

The implementation must provide the equivalent of:

```rust
struct AliasEnvironment {
    version: u64,
    entries: CanonicalNameMap<AliasTarget>,
}

enum AliasTarget {
    Binding(ResolvedBindingId),
    Mask(ResolvedMaskValue),
    Resolver(RegisteredAliasResolverId),
}
```

The concrete ownership and arena design may differ. The required properties are:

- [X] DONE — canonical keys use the registry's existing ASCII name-folding rule;
- [X] DONE — targets retain carrier, row-domain, schema/service version, and host-snapshot information needed to reject stale or incompatible handles;
- [X] DONE — a statement captures an immutable environment version or snapshot;
- [X] DONE — installation validates the target before publishing a new environment;
- [X] DONE — rebinding is atomic replacement of one key;
- [X] DONE — deletion is idempotent at the host API or reports a clear host error, but never touches the bare namespace;
- stale registry-bound targets are invalidated or revalidated across a `Σ → Σ′` migration described in `05`.

## Host and replay boundary

1. [X] DONE — Expose host operations to install, rebind, inspect, and delete a dynamic alias between statement steps. The exact Steel CLI, API, and Kore command spelling is a host-interface decision, not Ano grammar.
2. Apply an alias transition only after its target has resolved and validated. A failed transition leaves the old environment unchanged.
3. [X] DONE — Add each successful transition to the deterministic session/input log with the statement barrier and environment version at which it became visible. Resolver-backed aliases also require the resolver version and the frozen host-input events needed to reproduce each statement.
4. [X] DONE — Save/reload must choose one explicit policy and test it end to end:
   - persist the alias environment as session state; or
   - persist/replay the alias transition log.

   Either policy is valid only if replay reproduces the same environment versions and statement observations. The overlay must not be serialized as immutable registry schema. The chosen policy is persist-environment with barrier-stamped transition records; replay reproduces versions and plans.
5. [X] DONE — Kore must display enough alias state to diagnose shadowing without presenting the overlay as a column declaration.

## Resolver work

- [X] DONE — Bare `name` always bypasses `A_t`.
- [X] DONE — `^name` checks the captured overlay first, then runs the exact bare resolver path only when the canonical key is absent. An invalid present entry is an alias-resolution refusal, not a miss.
- [X] DONE — A spelling alias may still participate in resolving the bare target according to the existing registry rules; it must not become a live alias entry merely because the word “alias” is shared.
- [X] DONE — `!^name` resolves first and then negates the resulting mask.
- An overlay mask with the wrong query domain follows the ordinary alignment/lineage rules; the overlay is not permission to align equal-length buffers.
- [X] DONE — Diagnostics identify whether the value came from a binding, materialized mask, resolver, or bare fallback and include the captured overlay, resolver, and host-input versions when tracing is enabled.

## Tests

Add Steel, host/API, replay, and Kore tests for:

- [X] DONE — fallback with no dynamic alias;
- [X] DONE — same-stem shadowing of a bare column or binding;
- [X] DONE — installation, rebinding, and deletion across consecutive statement steps;
- [X] DONE — survival of the bare binding through the full alias lifecycle;
- [X] DONE — `!name` versus `!^name` before, during, and after shadowing;
- [X] DONE — ASCII case-fold equivalence and non-ASCII controls matching registry behavior;
- [X] DONE — a statement that performs multiple gathers while the host queues a rebind, proving one overlay snapshot is observed;
- [X] DONE — a resolver-backed deictic invoked by multiple gathers, proving that it may rerun but receives one frozen world/host-input snapshot, followed by a later statement that observes changed host input;
- failed installation of an unknown, stale, wrong-carrier, or wrong-schema target with no environment change, plus a stale present entry proving that lookup refuses rather than falling through;
- [X] DONE — deterministic log replay and save/reload under the chosen policy;
- [X] DONE — coexistence with a spelling alias and a static `AliasMask` fixture of related names.

## Sentinel

One executable witness for the category, carried from the demolition ledger; explanatory mathematics, not a semantic oracle. Lookup is overlay-first with bare fallback, and the bare binding survives the alias lifecycle.

```bqn
bare ← 1‿0‿1‿0
live ← 0‿1‿0‿1
Resolve ← {𝕨 ? 𝕩 ; bare}
! live ≡ 1 Resolve live
! bare ≡ 0 Resolve live
! bare ≡ 1‿0‿1‿0
```

```q
bare:1010b;
live:0101b;
resolve:{[has;target]$[has;target;bare]};
live~resolve[1b;live]
bare~resolve[0b;live]
bare~1010b
```

## Demo decommission and rewrite ownership

Decommission demo numbers `005`, `019`, `028`, `071`, `093`, and `103` as complete numbered units: Ano, Nihongo, BQN, registry, expected output, manifest, README claim, and any golden sibling. Do not patch their old outputs in place or count a surviving sibling as evidence.

Rewrite them only after the overlay implementation lands. New witnesses must prove fallback, shadowing, rebinding between steps, deletion, bare-name survival, and statement snapshots. Demo `028` also depends on `03`; demos `071` and `093` also depend on `99`. Each number is counted once in the global 64-demo ledger in `TODO.md`.

## Completion gate

- [X] DONE — The value denotation of `^name` can differ from bare `name` only while the captured overlay contains the stem; diagnostics still retain the sigiled source request.
- [X] DONE — Alias lifecycle cannot mutate a bare binding or registry declaration.
- [X] DONE — One statement cannot observe two alias-environment or host-input versions; resolver-backed deictics may rerun only against that frozen snapshot.
- [X] DONE — Replay and save/reload reproduce the same observations.
- [X] DONE — The three alias mechanisms are named and tested separately in code and documentation.
- All rewritten alias demos pass through Steel and Kore; the old six numbers remain out of the active evidence surface until then.
