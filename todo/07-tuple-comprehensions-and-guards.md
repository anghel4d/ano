# 07: Tuple comprehensions and guards

The accepted surface is one expression-local `[]` family: tuples `[a, b]`, inclusive integer ranges `[a..b]`, and forward comprehensions `[source -> name & qualifiers |=> result]`. Cartesian sources, dependent bindings, and scalar Boolean guards are implemented. `entities(predicate)` and `selection(references)` explicitly connect temporary values to deduplicated world targeting. Parentheses retain ordinary grouping and callable argument lists. Steel/Kore execute these constructors eagerly; explicit n-tuple assignments now use brackets. The old effectful bracket comprehension is disabled and preserved as commented code.

## Remaining work

- Extend the implemented Erlang-inspired pure guards with presence-aware and pattern guards. Preserve the accepted `&` qualifier separator; comma is tuple/hinge structure and `|` is OR, not a qualifier separator.
- Specify binding/destructuring patterns and failed-pattern behavior. Multiple generators already traverse a Cartesian product, with earlier bindings changing slowest; later sources may depend on earlier names. Yielded tuples retain nesting.
- Specify presence-aware guards without importing side effects into local construction. Existing presence checks inform the design but do not settle its grammar.
- Add stepped and unbounded ranges only after their evaluation and resource contracts are established.
- Preserve `lazy(construction)` as the intended optional wrapper: every enclosed construction is deferred, without eager exceptions. Decide forcing, world snapshot timing, lifetime, and errors before implementing it.
- Extend local callable carrier checking to nominal and symbol signatures and checked entity-returning host calls without erasing registry identity or checking only buffer shape.
- Specify how generated tuples can feed explicit assignment or destructuring while retaining row identity. Equal buffer lengths never establish world-row lineage.
- Review the relationship to prime's special relational algebra without confusing converse with tuple construction or differentiation.

<!-- /// !TODO: lazy(...) is intentionally retained in the design. Its entire enclosed construction must be lazy; current execution is eager and must not expose a pretend lazy wrapper. -->

## Evidence

Public CLI tests cover range results, order, multiplicity, nesting, lexical binding, empty inputs, column reads, Cartesian products, dependent generators, all comparison guards, entity provenance, deduplicated effects, keyed and positional identities, tuple publication, Kore rehydration, and refusal of retired syntax. Extend these contracts as the remaining design questions receive rulings. The historical rulings remain read-only.
