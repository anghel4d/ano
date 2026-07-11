# Ex46 — Japanese spatial recurrence, re-glossed: one stencil step; Version B canonical.
# ano: 升目の列で、各升の高さは前の二升の和となる。
# ano: Cell @ row , Height = prev.Height + prev.prev.Height   -- Version A
# ano: Cell @ row , Height = fib(index)                       -- Version B (canonical)
# Semantics: the comma is gather-effect-scatter and every read observes pre-state, so prev is a parallel shift, not a carry — Version A is one two-back stencil step and cannot generate Fibonacci (iterating it k times gives k stencil steps; a freshly minted line has zero pre-state and stays zero) — which is why the decision record makes Version B, the recurrence inside the registered host function fib, the canonical form.
# Invariant: one stencil step equals the hand-shifted pre-state sum; eight iterated steps on a fresh line stay all zero; fib(index) yields the Fibonacci column and differs from the stencil step.

Step ← {(»𝕩)+»»𝕩}              # Version A: one barrier step over pre-state
h0 ← 1‿2‿3‿4‿5‿6‿7‿8
"ex46: one stencil step, shift not carry"!(0‿1‿3‿5‿7‿9‿11‿13≡Step h0)
fresh ← 8⥊0
"ex46: a fresh line stays zero under iteration"!(fresh≡Step⍟8 fresh)

Fib ← {⊑{⟨1⊑𝕩,+´𝕩⟩}⍟𝕩 1‿1}     # Version B: the recurrence runs host-side
fibCol ← Fib¨↕8
"ex46: fib(index) column"!(1‿1‿2‿3‿5‿8‿13‿21≡fibCol)
"ex46: the stencil step is not the recurrence"!(¬fibCol≡Step h0)
