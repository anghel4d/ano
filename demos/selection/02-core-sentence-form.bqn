# Core sentence form (ex2).
# ano: source & predicate , effect
# Semantics: gather the rows of source where predicate holds, apply the effect to that fiber, scatter back in place — one barrier.
# Invariant: effect-under-mask-compress touches exactly the selected rows and preserves the rest.

source    ← 10‿20‿30‿40‿50
predicate ← 1‿0‿1‿0‿1
Effect    ← ×⟜2                  # a column effect: double the selected fiber

got ← Effect⌾(predicate⊸/) source

! 20‿20‿60‿40‿100 ≡ got
# doubling under the mask agrees with the masked-arithmetic form v×1+mask
! got ≡ source × 1+predicate
