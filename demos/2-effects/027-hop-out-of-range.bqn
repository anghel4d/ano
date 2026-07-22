# ano: Cursed , ~
#      Nord & mentor.Dead , -Trained
# Semantics: two rows despawn, leaving a stored id (5) at or beyond the new row count (4). A positional gather into the shifted column is a hard index fault; resolution by index-of against the surviving fixture-row ids reads the right survivor and never faults — staleness stays a mask question (left-join-null), not an error path.

cursed  ← 1‿1‿0‿0‿0‿0
mentor  ← ¯1‿¯1‿5‿¯1‿2‿¯1           # fixture-row ids; 5 survives despawn, 4 rows remain
dead    ← 0‿0‿0‿0‿0‿1
nord    ← 1‿1‿1‿1‿1‿1
trained ← 1‿1‿1‿1‿1‿1
idx     ← ↕6

# statement 1: Cursed , ~   (rows 0 and 1 leave)
keep ← ¬cursed
idx‿mentor‿dead‿nord‿trained ↩ keep⊸/¨ idx‿mentor‿dead‿nord‿trained

! idx ≡ 2‿3‿4‿5
! ∨´ mentor ≥ ≠idx                  # the fault shape, read from the column itself: a stored id at/beyond the new count

# statement 2: Nord & mentor.Dead , -Trained
f     ← idx⊐mentor
alive ← f<≠idx
mDead ← ((≠idx)|f)⊏dead             # the modulo clamp keeps every index in range
trained ∧↩ ¬ nord∧alive∧mDead

! trained ≡ 0‿1‿1‿1                 # only old row 2 (mentor 5, dead) loses it
