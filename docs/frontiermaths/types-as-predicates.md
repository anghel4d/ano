# Types as predicates

This is a design note, not a published openai/math family. It records how Ano's existing type-like machinery sits relative to two owner-priority results: [2100](2100-weak-to-strong-normalization.md) and [2167](2167-grothendieck-homotopy-hypothesis.md).

The intended reading is: `e : P` means the property `P` holds of `e`. It is not Curry–Howard. A type is not a theorem, a term is not a proof, and inhabitation is not a construction of evidence inside the object language. Intersection, union, and negation of properties are the natural operations. Program properties are stated as predicates over values, rows, or columns, without dependent types.

## What Ano already has

The language contract opens with that reading at the selection layer. The predicate *is* the entity reference: `Nord & TwoHanded > 60` names the rows for which those properties hold, and the host resolves the set ([ano-language.md](../ano-language.md), Intro and §1–4). `&`, `|`, and `!` are pointwise conjunction, disjunction, and negation of masks. They are the Boolean algebra of properties on a fixed domain, not a proof combinator.

The Lean spatial account uses the same shape. For `p : X → Bool`, the selected domain is `{x ∈ X | p(x)}` with inclusion `i : S → X`; reading a column after selection is `c ∘ i` ([spatialmaths.md](../spatialmaths.md), [Field.lean](../../proofs/Ano/Field.lean)). Predicate evaluation ranges over the source domain `X`; effects range over `S` ([ano-language.md](../ano-language.md) §5, ruling A16).

The stored type system is a small refinement of carriers, not a dependent type theory. Registry columns carry `bool`/`nat`/`int`/`num`/`sym`/`char`; typed descriptors add `mask`, `entity`, and earlier `enum`/`ctor` names; `range` further restricts a numeric carrier ([ano-registry.md](../ano-registry.md), [lib.rs](../../steel/src/lib.rs) `ColType` / `RegType`). Presence is a separate property from the stored value: `!TwoHanded` is absence, not the number zero. Publication retracts writes that fail the carrier or range ([emit.rs](../../steel/src/emit.rs) barrier comments; [relationship.rs](../../steel/src/relationship.rs) for endpoint predicates).

These checks classify values. They do not inhabit a proposition. A matching signature does not prove that a trusted BQN body tells the truth about its effects ([ano-registry.md](../ano-registry.md), Trust and replacement).

## What Ano does not have

There is no Curry–Howard surface. Standing rules, `|>`, and `;` are effect composition, not term constructors ([ano-language.md](../ano-language.md) §10–11). The Lean kernel is an external semantic model of layouts, frames, interpolation, and accepted plans ([lean.md](../../proofs/lean.md)). It is not Ano's type checker, and a successful `lake build` does not type an `.ano` file.

There is no general predicate-type calculus. `nat` and `range gold 0 100` are fixed refinements. `Nord & TwoHanded > 60` is a runtime mask, not a type that Steel uses to reject a later assignment. Intersection and union of *carriers* are not declared; only masks combine that way. Dependent pairs, path types, and universes are absent.

The Sky essay's line "an array language is just a type" is design motivation, not an implemented type language ([ano-sky.md](../ano-sky.md)). A future proof-carrying rewrite registry is specified as checked laws with signatures, endpoints, and proof payloads ([foundations.md](../../proofs/foundations.md), Rewrite authority). That is closer to "types as predicates plus an external checker" than to "types as spaces" or "types as proofs."

## Relation to 2100

[2100](2100-weak-to-strong-normalization.md) is about β-reduction in pure type systems. If a future Sky rewrite language is a PTS and every legal rewrite has *some* normal form, then every reduction sequence terminates.

That theorem does not apply to BQN evaluation, effect pipelines, standing rules, or ticks. Those are not β-steps in a PTS. It also does not decide whether Ano should grow a PTS. The types-as-predicates reading says: if a rewrite language appears, treat its types as properties of programs or columns, and keep inhabitation out of the object language. 2100 then becomes a termination lemma for that rewrite system, not a reason to put proofs in `.ano` files.

## Relation to 2167

[2167](2167-grothendieck-homotopy-hypothesis.md) says that algebraic ∞-groupoids recover the homotopy theory of spaces. That is the types-as-spaces tradition: a type is a space, a term is a point, a path is an identification.

Ano's Lean kernel is the opposite architecture. `AffineFrame`, `WeightedSupport`, and `Field` are explicit 1-categorical structures with declared equations ([Affine.lean](../../proofs/Ano/Affine.lean), [Interpolation.lean](../../proofs/Ano/Interpolation.lean)). Points are not vectors; vectors are not entity keys; equal buffer length is not an identification ([spatialmaths.md](../spatialmaths.md)). The homotopy hypothesis does not authorize treating those identifications as paths, and it does not authorize importing coherators, elementary expansions, or HoTT into Steel.

The useful reading is negative: do not solve Ano's type questions by becoming a homotopy type theory. Keep `e : P` as a property of `e`.
