# This file is not for LLM use. Carry on

q)max 0N 5 0N 1 3    / nulls ignored     → 5
q)max 0N 0N          / all null          → -0W

Ano has no null — A12 says no none marker, error, NaN, infinity or placeholder — and todo/03 bans infinity as an extrema identity outright. So "ignore nulls" has nothing to ignore, and "all null" is just the empty scope, whose answer under A12 is no result row, not -0W. Implementing those two would overturn a settled ruling, so I've explicitly instructed the agent not to. If you want q's null semantics in Ano, that's a real design change and it belongs in the ledger, not smuggled in through a fixture.

Similarly 98|"a": q promotes char to int there, but todo/03 already rules that mixed carriers refuse before lowering, and A9 adopts q's convention over Ano's carriers rather than q's type-promotion rules. So that fixture lands as a refusal pinning the exact diagnostic — which still shows you the behavior, just as a refusal rather than a value.

# 1. Infinity, yes or no?
+Inf and -Inf. What to represent them with?

# 2. None, yes or no?
A 'None' type that isn't actually a columnar entry at all. Hmm.

# Type promotions, yes or no, and if yes what hierarchy to follow? Should be standard everywhere.
Implicit type promotion, a la q. Or perhaps explicit type promotion like Haskell's DataKinds.
  
My position on the matter: we should actually have BOTH. Here's how and why. q's runtime promotion optimized for very fast vector execution in the runtime ano files and scripting, and Haskell's DataKinds at the registry level.
Overarching design principle: in ano, the runtime is a flexible columnar vector world like q, but the registry and column definitions are strict like Rust and Haskell.