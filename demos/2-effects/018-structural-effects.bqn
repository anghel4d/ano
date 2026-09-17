# ano: Dead , ~
#      Frenzy.targets' , +Frenzied
#      Burdened , -Encumbered
# Semantics: despawn filters every column; the set-hop image is the union of the target fibers and a selection is a mask, so a target reachable from two sources gains Frenzied once; component add/remove flips presence masks. Targets are stable ids, not row positions.
# Invariant: rows shrink by exactly the Dead count with survivors intact and in order; Frenzied lands exactly on the deduplicated image (3 targets from 4 edges); Encumbered clears exactly under Burdened.

id         ← 0‿1‿2‿3‿4‿5‿6‿7
dead       ← 0‿1‿0‿0‿0‿0‿1‿0
frenzy     ← 1‿0‿0‿1‿0‿0‿0‿0
targets    ← ⟨2‿4, ⟨⟩, ⟨⟩, 4‿5, ⟨⟩, ⟨⟩, ⟨⟩, ⟨⟩⟩   # 0→{2,4}, 3→{4,5}: id 4 hit twice
frenzied   ← 0‿0‿0‿0‿0‿0‿0‿1
burdened   ← 0‿0‿1‿0‿0‿1‿0‿0
encumbered ← 0‿0‿1‿0‿1‿1‿0‿0

# statement 1: Dead , ~   (barrier commits before statement 2 gathers)
keep ← ¬dead
id‿frenzy‿targets‿frenzied‿burdened‿encumbered ↩ keep⊸/¨ id‿frenzy‿targets‿frenzied‿burdened‿encumbered

! id ≡ 0‿2‿3‿4‿5‿7                    # exactly the ¬Dead rows, order preserved

# statement 2: Frenzy.targets' , +Frenzied
edges ← ∾frenzy/targets               # every in-edge, with multiplicity
img   ← id ∊ edges                    # the image: a mask, no multiplicity
frenzied ∨↩ img

! edges ≡ 2‿4‿4‿5
! 4 ≡ ≠edges                          # four edges...
! 3 ≡ +´img                           # ...three targets: the scatter is idempotent
! frenzied ≡ 0‿1‿0‿1‿1‿1              # ids 2,4,5 gain it; id 7 keeps it

# statement 3: Burdened , -Encumbered
encumbered ∧↩ ¬burdened

! encumbered ≡ 0‿0‿0‿1‿0‿0            # ids 2,5 cleared; id 4 (not Burdened) keeps it
