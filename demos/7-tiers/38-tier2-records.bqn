# 38 - Tier 2 records: aligned value write, scalar read, rank scattered back to identities
# ano: Nord & TwoHanded > 60 , Gold += 1000
# ano: +/ Gold @ Nord
# ano: Unit , Rank = rank(Gold)          -- Version B, canonical
# Semantics: three statements, three barriers; each write-back is identity-aligned (α→α), each fold exits the tier to one scalar, and each statement's gather observes the previous statement's commit.
# Invariant: post-states equal the hand-computed columns; the write touches only masked rows; on tie-free gold the ordinal rank ⍋⍋ agrees with the Sym(I)-lawful dense rank (the tie case is t2-ties.bqn).

# world: six entities, all Units
nord      ← 1‿1‿0‿1‿0‿1
twoHanded ← 70‿50‿80‿90‿61‿65
gold      ← 100‿250‿40‿10‿520‿305
rank      ← 6⥊0

# statement 1: masked add, alignment preserved — entity 3's gold stays entity 3's
sel ← nord∧twoHanded>60
! 1‿0‿0‿1‿0‿1 ≡ sel
gold +↩ 1000×sel
! 1100‿250‿40‿1010‿520‿1305 ≡ gold
! 250‿40‿520 ≡ (¬sel)/gold          # unselected rows untouched

# statement 2: scoped fold to one scalar, observing statement 1's commit
! 3665 ≡ +´nord/gold                 # 1100+250+1010+1305

# statement 3: rank gold, scatter back aligned to the same identities
rank ↩ ⍋⍋gold
! 4‿1‿0‿3‿2‿5 ≡ rank
! rank ≡ (∧⍷gold)⊐gold               # tie-free: ordinal and dense rank coincide
