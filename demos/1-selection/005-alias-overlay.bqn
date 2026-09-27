# ano: the dynamic alias overlay — ^focus reads the live entry, bare focus reads the column, and deleting the entry re-reveals the bare name (A4). An elided first statement is the ^cursor cold default.
# Invariant: lookup is overlay-first with bare fallback; the bare binding survives the whole alias lifecycle; ! negates the resolved mask, never the lookup.

bareFocus ← 1‿0‿0‿0‿1
liveFocus ← 0‿1‿0‿1‿0
Resolve ← {𝕨 ? liveFocus ; bareFocus}     # 𝕨: does the overlay hold the stem?

! liveFocus ≡ 1 Resolve liveFocus
! bareFocus ≡ 0 Resolve liveFocus
! bareFocus ≡ 1‿0‿0‿0‿1                    # survival through install, rebind, delete
! 2 ≡ +´ 1 Resolve liveFocus
! 2 ≡ +´ bareFocus
! 0 ≡ +´ liveFocus∧bareFocus               # shadowed and bare diverge in one world
! 0‿0‿1‿0‿0 ≡ (¬liveFocus)∧¬bareFocus      # !^focus & !focus selects exactly row 2
