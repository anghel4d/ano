# 07: Tuple comprehensions and guards

The accepted surface is one expression-local `[]` family: tuples `[a, b]`, inclusive integer ranges `[a..b]`, and forward comprehensions `[source -> name |=> result]`. Parentheses retain ordinary grouping and callable argument lists. Steel/Kore execute these constructors eagerly; explicit n-tuple assignments now use brackets. The old effectful bracket comprehension is disabled and preserved as commented code.

## Remaining work

- Explore Erlang-style comprehensions and pure guards as a powerful, minimal addition to this construction family. Settle one qualifier grammar before implementation; comma is tuple/hinge structure and `|` is OR, not a qualifier separator.
- Specify multiple generators, dependent sources, Cartesian versus zipped traversal, binding/destructuring patterns, and failed-pattern behavior. Current nesting preserves result nesting and is not an implicit flattening operation.
- Specify presence-aware guards without importing side effects into local construction. Existing presence checks inform the design but do not settle its grammar.
- Add stepped and unbounded ranges only after their evaluation and resource contracts are established.
- Preserve `lazy(construction)` as the intended optional wrapper: every enclosed construction is deferred, without eager exceptions. Decide forcing, world snapshot timing, lifetime, and errors before implementing it.
- Extend local callable carrier checking to nominal and symbol signatures without erasing registry identity or checking only buffer shape.
- Specify how generated tuples can feed explicit assignment or destructuring while retaining row identity. Equal buffer lengths never establish world-row lineage.
- Review the relationship to prime's special relational algebra without confusing converse with tuple construction or differentiation.

<!-- /// !TODO: lazy(...) is intentionally retained in the design. Its entire enclosed construction must be lazy; current execution is eager and must not expose a pretend lazy wrapper. -->

## Evidence

Public CLI tests cover range results, order, multiplicity, nesting, lexical binding, empty inputs, column reads, tuple publication, and refusal of retired syntax. Extend these contracts as the remaining design questions receive rulings. The historical rulings remain read-only.
