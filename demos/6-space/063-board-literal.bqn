
glyphs ← "RNBQKBNRPPPPPPPP................................pppppppprnbqkbnr"
! 64 = ≠glyphs

board ← 8‿8⥊glyphs
! 8‿8 ≡ ≢board
! "RNBQKBNR" ≡ 0⊏board
! "PPPPPPPP" ≡ 1⊏board
! ∧´"........"⊸≡˘ 2‿3‿4‿5⊏board
! "pppppppp" ≡ 6⊏board
! "rnbqkbnr" ≡ 7⊏board

sep ← "RNBQKBNR/PPPPPPPP/......../......../......../......../pppppppp/rnbqkbnr"
! 71 = ≠sep
! (×´8‿8) ≠ ≠sep

occupied ← '.'≠⥊board
piece    ← occupied/⥊board
cell     ← /occupied
! 32 = ≠piece
! piece ≡ "RNBQKBNRPPPPPPPPpppppppprnbqkbnr"
! cell  ≡ (↕16)∾48+↕16
! 8 = +´piece='P'
! 8 = +´piece='p'
! 2 = +´piece='R'
! 1 = +´piece='q'

sourceGlyph ← ↕≠glyphs
cellSource ← 8‿8⥊sourceGlyph
! 64 = ≠sourceGlyph
! 8‿8 ≡ ≢cellSource
! sourceGlyph ≡ ⥊cellSource
! 32 = ≠occupied/sourceGlyph
