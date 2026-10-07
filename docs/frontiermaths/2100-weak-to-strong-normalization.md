# 2100 — Weak normalization implies strong normalization in pure type systems

openai/math family **245**. Card **2100**.

## What it says

A pure type system (PTS) is specified by a set of sorts and axioms/rules that say which products are legal. Weak β-normalization (WN) means every legal expression in every valid context has *some* finite β-reduction sequence to a normal form. Strong β-normalization (SN) means *every* β-reduction sequence from every such expression terminates.

The result is that WN implies SN for every PTS. Reduction is allowed inside type annotations. The specification may have arbitrary sorts and nonfunctional axioms or rules. Open contexts are included. This is the β-Barendregt–Geuvers–Klop conjecture.

In plain terms: if every well-typed term can be normalized by some strategy, then no well-typed term admits an infinite reduction, so you do not need a separate SN proof once WN is established.

## Manuscripts

- Family entry: [CONTENTS.md §245](https://github.com/openai/math/blob/main/CONTENTS.md)
- [Weak and strong normalization in pure type systems](https://github.com/openai/math/blob/main/preprints/Weak-and-strong-normalization-in-pure-type-systems-September-25-2026/paper.pdf) — folder [`preprints/Weak-and-strong-normalization-in-pure-type-systems-September-25-2026/`](https://github.com/openai/math/tree/main/preprints/Weak-and-strong-normalization-in-pure-type-systems-September-25-2026)

## Lean status

Partial. One comparator: [`lean/docs/245.md`](https://github.com/openai/math/blob/main/lean/docs/245.md), statement in `TypeSystemNormalization.lean`.

The formalized claim matches the prose: WN for all legal expressions in all valid contexts implies SN, with reduction inside annotations and no functionality hypothesis.

## Tie to this repository

Ano is not a PTS. Steel parses, resolves carriers, and emits BQN ([emit.rs](../../steel/src/emit.rs)). Kore runs the world. The Lean kernel in `proofs/Ano/` models fields, frames, interpolation, and accepted plans; it does not type-check Ano source ([lean.md](../../proofs/lean.md)).

The live connection is the *future* Sky rewrite registry. [foundations.md](../../proofs/foundations.md) requires a reusable rewrite to name its law, version, typed signature, endpoints, schema fingerprint, normalized proposition, checker, and proof payload. [ano-sky.md](../ano-sky.md) is the design motivation; [reducer.rs](../../steel/src/reducer.rs) already refuses regrouping or reordering unless associativity/commutativity is declared (`LawStatus::Undeclared` admits only exact left accumulation).

If that rewrite language is later taken to be a PTS (or a fragment whose β-steps match the theorem's), 2100 says: prove WN for the registered laws, and SN follows. You do not need a second termination argument to stop an optimizer from looping. That is the only practical implication.

The theorem does **not** apply to:

- BQN evaluation of emitted programs
- `|>` / `;` effect composition ([ano-language.md](../ano-language.md) §10)
- standing-rule scheduling ([ano-time.md](../ano-time.md))
- finite tick induction ([tick-induction.md](../../proofs/tick-induction.md))
- seeded-fold or fallible-traverse decisions ([todo/06](../../todo/06-seeded-fold-and-traverse.md))

Those are not β-reduction of PTS terms. Task 06's open seed/failure readings stay open; 2100 does not settle them.

See [types as predicates](types-as-predicates.md) for why a rewrite language, if added, should still treat types as properties rather than as proofs.

## Caveats

The result is about untyped β-steps on legal PTS terms, not about computational complexity or Steel's left folds. A weakly normalizing *object language* that is not a PTS gains nothing.

WN is the hard hypothesis. The theorem does not produce a normalizer. If the Sky language is not shown to be WN, SN remains unproved.

Partial Lean covers the comparator statement, not a machine-checked proof that any particular Ano rewrite set is WN.
