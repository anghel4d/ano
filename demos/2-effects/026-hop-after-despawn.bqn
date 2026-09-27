# ano: Cursed , ~
#      Nord & mentor.Dead , -Trained
# Semantics: with no id column the rel is keyed to the fixture row index. A despawn shifts rows, so the hop must resolve through the surviving fixture-row ids (index-of, one found-guard), never a positional gather into the shifted column — mentor 3 names old row 3, not whatever now sits in slot 3.

cursed  ← 0‿1‿0‿0‿0‿0
mentor  ← ¯1‿¯1‿3‿¯1‿3‿0            # fixture-row ids
dead    ← 1‿0‿0‿1‿0‿0
nord    ← 1‿1‿1‿1‿1‿1
trained ← 1‿1‿1‿1‿1‿1
idx     ← ↕6                        # the fixture row iota, materialized ONCE

# statement 1: Cursed , ~   (row 1 leaves; every column filters, idx included)
keep ← ¬cursed
idx‿mentor‿dead‿nord‿trained ↩ keep⊸/¨ idx‿mentor‿dead‿nord‿trained

! idx ≡ 0‿2‿3‿4‿5                   # the surviving fixture-row ids

# statement 2: Nord & mentor.Dead , -Trained   (resolve through idx, not position)
f     ← idx⊐mentor                  # slot of each stored id among survivors; ≠idx = not found
alive ← f<≠idx                      # found-guard: dangling (¯1) and despawned fail alike
mDead ← ((≠idx)|f)⊏dead             # clamped gather, dead weight where alive is 0
trained ∧↩ ¬ nord∧alive∧mDead

! trained ≡ 1‿0‿1‿0‿0               # old rows 2, 4, 5 lose it; dangling rows keep it

# 2 Trained, because shifted slot 3 holds old row 4 (alive), not old row 3 (dead)
! 0 ≡ 3⊑dead
