# 01 — sigil semantics and the resolver boundary

Status: complete. The resolved IR preserves bare, sigiled, and negated requests; overlay and bare fallback attach at one boundary; all seventeen original tests and the expanded lifecycle/deictic suite pass now that fold rendering is live.

## Verified current state

- The lexer already distinguishes `^name` from a bare name and rejects a lone `^`.
- The parser preserves that distinction as an alias node. `!` is parsed independently as prefix logical negation, so `!^name` is structurally `Not(Alias(name))` rather than a third lookup form.
- Names are interpreted contextually in mask position: a Boolean column denotes its values (and presence where applicable), a sparse non-Boolean value column denotes its presence mask, and a total non-Boolean value column denotes the all-present mask. The emitter has an explicit `!` fast path for sparse value columns, but it computes the same `not(presence)` result. This is mask interpretation, not a lookup mode.
- The emitter carries an explicit lookup mode through normalization and resolution. Bare requests bypass the overlay; sigiled requests consult the captured overlay and invoke the exact bare path only on absence; diagnostics retain the source request and provenance.
- The registry already uses the word “alias” for two other, static mechanisms:
  - a name-translation table used to accept alternate spellings; and
  - an `AliasMask` registry entry created by the registry `alias` directive.
  Neither mechanism is the live, statement-to-statement overlay ruled for `^name`.
- Registry name equality is ASCII case-insensitive. The dynamic path must use the same canonicalization; this task must not introduce Unicode folding or a second equality rule.

## Contract

`!` is an operator. It applies pointwise negation to the mask denoted by its operand under the existing mask-context rules. It never changes how a name is looked up.

`^` is a lookup sigil. A sigiled name and a bare name with the same stem are different resolver requests:

```text
name   → resolve_bare(name)
^name  → resolve_alias_or_bare(name)
!name  → not(as_mask(resolve_bare(name)))
!^name → not(as_mask(resolve_alias_or_bare(name)))
```

The fallback in `resolve_alias_or_bare` is implemented in `02`; this task only makes that resolver choice survive parsing, planning, diagnostics, and lowering.

The following remain invalid unless separately ruled:

- a lone `^`;
- `^` applied to an arbitrary expression rather than a name;
- assignment to `^name` from Ano source as an implicit alias-installation syntax;
- treating `!` as part of an identifier;
- stripping `^` before resolution.

## Work

1. Preserve `Name` and `Alias` as distinct cases through the resolved IR. Do not reduce both to a shared string before name resolution.
2. Introduce one explicit resolver boundary with separate entry points or an explicit lookup mode, for example:

   ```rust
   enum LookupMode {
       Bare,
       DynamicAliasThenBare,
   }
   ```

   The concrete Rust shape is not prescribed; the semantic distinction is.
3. Route bare names only through the existing registry/binding lookup path. Route alias nodes to a stub or interface that `02` can connect to the live overlay, with bare fallback remaining explicit rather than accidental.
4. Preserve source spelling in diagnostics. An unresolved bare `name` and an unresolved `^name` may share a final “name not found” cause after fallback, but the diagnostic must retain which resolver request the source made.
5. Keep `Not` outside lookup. The resolver/planner must first resolve the operand and obtain its mask-context interpretation, then emit negation. Preserve the existing presence/absence interpretation for value columns; any operand that cannot denote a mask under those rules refuses.
6. Rename internal documentation or fields where necessary so the three meanings are not conflated:
   - **spelling alias** — the registry name-translation table;
   - **static alias mask** — the stored `AliasMask` registry entry;
   - **dynamic alias** — the live overlay consulted only by `^name`.
   Public compatibility names may remain where renaming would be disruptive, but comments and tests must use the precise term.
7. Add parser, resolver, and emitted-plan tests for bare `name`, `^name`, `!name`, and `!^name`; include Boolean masks, sparse-value presence/absence, total-value all-present controls, ASCII case-fold controls, and lone-`^` refusals.
8. Do not add alias mutability, persistence, host commands, or statement snapshots here. Those belong together in `02`.

## Completion gate

- [X] DONE — The resolved plan can distinguish `name` from `^name` without inspecting source text again.
- [X] DONE — `!^name` is demonstrably negation of the sigiled lookup result’s mask interpretation, while `!name` is negation of the bare lookup result’s mask interpretation.
- [X] DONE — No existing spelling alias or static `AliasMask` fixture silently becomes dynamic.
- [X] DONE — Existing bare-name behavior is unchanged.
- [X] DONE — The live-overlay task can attach at one resolver boundary rather than patching lexer, parser, emitter, and Kore independently.
