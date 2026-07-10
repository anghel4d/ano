# 09 — numeric edges: the float64 cliff and the inf asymmetry

Surfaced by the 2026-07-10 overflow experiments (live, verified). No ruling was requested then. Filed now. Two halves: document the number model, and close a gap in its seal.

## The number model, as measured

Numbers are IEEE 754 float64 end to end. lex.c reads digit-run literals via strtod (optional decimal point, no exponent form in .ano source, 63-char cap). emit.c's numLit spells exact integers |x| ≤ 9e15 as integers and everything else as round-tripping %.17g with BQN high-minus. registry.c's dnum writes plain digits in the exact range, else shortest round-tripping %g. Consequences, all confirmed live:

- The max exact integer is 2^53 = 9,007,199,254,740,992. Past it, additions below the local ULP silently vanish (`+= 1` onto 3.1e19 is a no-op). Quiet absorption, IEEE semantics, not a bug.
- Past DBL_MAX ≈ 1.8e308, BQN yields ∞ and the save REFUSES the pipe-back ("save: col gold: bad number '∞'", exit 2, the world file stands). Overflow is a refused tick. The seal holds from the inside.

## The gap: the seal leaks from the outside

A hand-written `inf` in a .reg LOADS (strtod parses it to INFINITY) and then dies downstream: numLit spells it `inf`, BQN sees an undefined identifier, the program crashes with no useful diagnostic. So the system refuses to WRITE a non-finite world but accepts READING one. Anything anoc saves it can load, but not everything it loads could have been saved.

## Work items

1. Rule and implement the closure. Review recommendation: refuse non-finite values (inf, -inf, nan, and any strtod spelling of them) at registry LOAD, mirroring the save's refusal, with a line/column diagnostic. The invariant becomes airtight: the registry's value domain is exactly the finite doubles, and load ∘ save = identity on it. The alternative (teach numLit BQN's ∞ literal and let worlds go non-finite) trades the determinism stance for nothing the demos want. Surface it, but the refusal is the coherent seal.
2. Document the model where users hit it: ano-manual.md's "Where the edges are" chapter. The 2^53 cliff, quiet ULP absorption, overflow as a refused tick with the world standing, and the (now-closed) load rule. A sentence in the spec if the value-domain statement belongs there too.
3. A load-refusal fixture in the suite (a .reg carrying `inf`, expected exit 2 with the diagnostic) so the seal is pinned, not promised. The refused-save path already has its behavior. Pin it too if no fixture exists.

## Pointers

src/lex.c (number lexing, ~150), src/emit.c numLit (~174) and emitSave (~1843), src/registry.c dnum (~472) and the load path. The refused-save message text is the seam to mirror.

## Invariants

- Both suites green. No existing demo carries a non-finite value, so no corpus churn.
- kore ticks keep their behavior: an overflowing tick fails, the world stands, undo unaffected.
- ano-time.md's quantize entry (BIND_QUANTIZE, float ingress as the determinism boundary) is adjacent doctrine. Cite it, don't touch it.
