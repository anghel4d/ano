# ano (new):        Pen , Headcount = #/ (livestock' & Cattle)
# Semantics: `fold/ col @ scope` remains the scoped-global fold and is always one scalar; `fold/ rel'…` is always per-source — the result-type split is lexical, never a registry lookup.
# Invariant: the broadcast column and the per-pen gamma column disagree; the gamma column matches the hand count and folds back to the global scalar.

pen    ← 0‿0‿1‿1‿1‿2‿0        # target pen per animal (livestock' at pen j is the preimage)
cattle ← 1‿0‿1‿1‿1‿0‿1        # Cattle mask per animal
pens   ← ↕4

broadcast ← (≠pens) ⥊ +´cattle          # what §11 semantics gave: one scalar over all livestock, scattered into every pen
perPen    ← +´¨ (pen∾≠pens) ⊔ cattle    # gamma: fold each fiber of livestock'

! 5‿5‿5‿5 ≡ broadcast
! 2‿3‿0‿0 ≡ perPen
! broadcast ≢ perPen
! (+´cattle) ≡ +´perPen        # the scoped-global scalar is the fold of the gamma column
