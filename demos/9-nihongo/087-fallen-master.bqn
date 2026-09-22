# ano: 師匠が死んだ北は、訓練を失う。
# ano: Nord & mentor.Dead , -Trained
# Semantics: gather Dead through the mentor hop; a dangling link (no mentor) fails the predicate per §5's left-join-null rule; -Trained clears the component's presence mask on the selection.
# Invariant: Trained is removed exactly where Nord holds and the mentor exists and is dead; the dangling-mentor row keeps Trained even though a naive wrapped gather would select it.

mentor  ← 3‿¯1‿3‿4‿0‿4        # ¯1 = no mentor (entity 1 dangles)
dead    ← 1‿0‿0‿1‿0‿0
nord    ← 1‿1‿1‿1‿1‿0
trained ← 1‿1‿1‿0‿1‿1
has ← mentor≥0
sel ← nord∧has∧(has×mentor)⊏dead   # clamp the dangle to 0, then mask it out
trained ↩ trained∧¬sel
"ex44: post-state Trained"!(0‿1‿0‿0‿0‿1≡trained)
"ex44: dangling hop fails the predicate"!(1=1⊑trained)
