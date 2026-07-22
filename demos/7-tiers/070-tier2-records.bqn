
nord      ← 1‿1‿0‿1‿0‿1
twoHanded ← 70‿50‿80‿90‿61‿65
gold      ← 100‿250‿40‿10‿520‿305
rank      ← 6⥊0

sel ← nord∧twoHanded>60
! 1‿0‿0‿1‿0‿1 ≡ sel
gold +↩ 1000×sel
! 1100‿250‿40‿1010‿520‿1305 ≡ gold
! 250‿40‿520 ≡ (¬sel)/gold

! 3665 ≡ +´nord/gold

rank ↩ ⍋⍋gold
! 4‿1‿0‿3‿2‿5 ≡ rank
! rank ≡ (∧⍷gold)⊐gold
