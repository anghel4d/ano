
Step ← {(»𝕩)+»»𝕩}
h0 ← 1‿2‿3‿4‿5‿6‿7‿8
"ex46: one stencil step, shift not carry"!(0‿1‿3‿5‿7‿9‿11‿13≡Step h0)
fresh ← 8⥊0
"ex46: a fresh line stays zero under iteration"!(fresh≡Step⍟8 fresh)

Fib ← {⊑{⟨1⊑𝕩,+´𝕩⟩}⍟𝕩 1‿1}
fibCol ← Fib¨↕8
"ex46: fib(index) column"!(1‿1‿2‿3‿5‿8‿13‿21≡fibCol)
"ex46: the stencil step is not the recurrence"!(¬fibCol≡Step h0)
