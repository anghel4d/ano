
symFriendly‿symHostile‿symNone ← ↕3

cursor   ← 0‿0‿1‿0‿0
observer ← 0‿1‿0‿0‿0
player   ← 1‿0‿0‿0‿0
selected ← 0‿0‿1‿1‿0

health  ← 50‿60‿70‿80‿90
faction ← symNone‿symHostile‿symNone‿symNone‿symNone
gold    ← 1‿2‿3‿4‿5
damage  ← 10‿20‿30‿40‿50

health ↩ (0¨)⌾(cursor⊸/) health
faction ↩ (symFriendly¨)⌾(observer⊸/) faction
gold +↩ 9999×player
damage ×↩ 1+selected

! 50‿60‿0‿80‿90 ≡ health
! symNone‿symFriendly‿symNone‿symNone‿symNone ≡ faction
! 10000‿2‿3‿4‿5 ≡ gold
! 10‿20‿60‿80‿50 ≡ damage
