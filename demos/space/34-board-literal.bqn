# Board literal (ex34, spec §26).
# Ano: "RNBQKBNRPPPPPPPP................................pppppppprnbqkbnr"
#        to 8 8 , spawn (pieceOf char)
# Semantics: the string literal becomes a glyph lattice carrying char per cell; the rule is exactly 64 glyphs, no separators — the shape must consume the literal; a registered lookup maps each non-empty glyph to a spawn.
# Invariant: the 64-glyph literal reshapes to the chess start position rank by rank; a separatored literal cannot be consumed by the shape (71 ≠ 8×8) and is rejected, never stripped; the spawned pieces and their cells match the hand-computed start position.

glyphs ← "RNBQKBNRPPPPPPPP................................pppppppprnbqkbnr"
! 64 = ≠glyphs                     # exactly 64 glyphs: the shape consumes the literal

board ← 8‿8⥊glyphs
! 8‿8 ≡ ≢board
! "RNBQKBNR" ≡ 0⊏board
! "PPPPPPPP" ≡ 1⊏board
! ∧´"........"⊸≡˘ 2‿3‿4‿5⊏board
! "pppppppp" ≡ 6⊏board
! "rnbqkbnr" ≡ 7⊏board

# a separatored literal is a compile error in ano, never a strip: the shape cannot consume it
sep ← "RNBQKBNR/PPPPPPPP/......../......../......../......../pppppppp/rnbqkbnr"
! 71 = ≠sep
! (×´8‿8) ≠ ≠sep

# spawn (pieceOf char): '.' maps to no spawn, every other glyph spawns its piece at its cell
occupied ← '.'≠⥊board
piece    ← occupied/⥊board
cell     ← /occupied
! 32 = ≠piece
! piece ≡ "RNBQKBNRPPPPPPPPpppppppprnbqkbnr"
! cell  ≡ (↕16)∾48+↕16             # back ranks and pawn ranks only
! 8 = +´piece='P'
! 8 = +´piece='p'
! 2 = +´piece='R'
! 1 = +´piece='q'
