# 07: Tuple comprehensions and guards

Explore Erlang-style comprehensions and guards for Ano's n-tuples. The intended surface is one expression-local `()` family for tuples, generators, guards, and comprehensions: Ano is already a language of mutable array columns, so this form denotes values that exist within an expression and are not implicitly written to the world store. The goal is a powerful addition with a small, coherent grammar and a clear denotation. This is a design task; no new syntax is accepted by this TODO.

## Starting point

Steel/Kore implement explicit, matching n-tuple assignments, including nested shapes and mixed leaf carriers. Their members read one incoming state and share row validity. Existing effect comprehensions remain available; `;` and `|>` retain simultaneous and sequential composition respectively.

## Questions to resolve

- Decide what a comprehension ranges over and produces: tuples within one row, streams of tuple values, or row domains carrying tuples. Keep these distinct from entity selections and spatial product domains.
- Explore tuple binding/destructuring and pure guards together. Specify name scope, repeated bindings, nesting, empty inputs, and whether tuple shapes are statically fixed.
- Define whether a shape mismatch, absent member, or failed guard filters an element or refuses the expression. Separate these from type errors and runtime failures.
- Establish ordering, multiplicity, and lineage before lowering. State how a resulting tuple can feed a column assignment without treating equal buffer lengths as proof of domain agreement.
- Decide which expressions guards admit, how they consume foundness, and when they read world state. Keep side effects out of guards; any effectful comprehension must retain explicit composition and publication boundaries.
- Develop the unified `()` grammar alongside existing grouping, calls, scope, and comma syntax; account explicitly for migration from legacy `[... | ... <- ...]` comprehensions. Specify ambiguity and precedence, including nested comprehensions, before choosing a spelling.
- Settle evaluation timing: eager, lazy, or eager by default with a fully enclosing `lazy(...)` boundary. Haskell-style range notation remains a candidate, not accepted syntax.
- Review the relationship to prime's special relational algebra without confusing converse with tuple construction or differentiation.
- Determine whether named tuple values, tuple-returning callables, or a registry tuple carrier are needed. Add only the machinery required by the chosen denotation.

## Evidence before implementation

Work through transferring three different currencies, filtering tuples by a balance guard, destructuring nested mixed-carrier tuples, preserving duplicates, and consuming a guarded result in a simultaneous assignment followed by a sequential stage. Give each example explicit inputs, result shape/domain, output values, and refusal or absence behavior.

Obtain a ruling on the chosen grammar and denotation before implementing them. Then align the language reference, Steel/Kore, persistence, and public behavior tests.
