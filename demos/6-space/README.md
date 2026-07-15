# space

Status: active proof and acceptance suite. The current Steel emitter still has one unnamed 2D lattice, infers transient frames from surface shape, treats fields and entity columns alike in value expressions, and carries no semantic habitat or destination lineage. The tests make those gaps visible; they are expected to drive the formalization, not be hidden until it exists.

The one-step pins are insufficient. Repeating `056-computed-line` grows the entity world from 12 to 44 rows on the first step and fails the second with a 12/44 scatter-length mismatch. Repeating spatial top-k grows 64 to 72 and fails the second on a 72/64 mask mismatch. These failures are expected until the compiler implements the habitat contract in `docs/ano-language.md` and `proofs/foundations.md`.

The files below remain useful as transformation sketches and regression inputs for the rebuild.
- `055-lattice-patterns.bqn` — ex27/§20: checkerboard, diagonal, triangle, stripes as predicates on the coordinate columns; masks match the outer-product forms and counts.
- `056-computed-line.bqn` — ex28/§21: Version B canonical (`offset = fib(index)`); one shift-add barrier step over a fresh line stays zero, never Fibonacci; the spiral zips index-angle pairs, not a catenated 24-list.
- `057-spiral-assign.bqn` — ex29/§21: phyllotaxis positions from the selection's own index, scattered onto Coin rows only; carries the pair-zip fix.
- `058-space-reductions.bqn` — ex30/§22: scoped folds, plus BOTH cost scans — the leading-axis `+\` and the true `scan2(+)` summed-area table — asserted cell-by-cell and asserted unequal.
- `060-density-fields.bqn` — ex32/§24: count fields replicate cells into populations; the fertility mask filters before the counts replicate.
- `062-derived-fields.bqn` — ex33/§25: ridge corrected to `sin(x/8)+sin(y/8)` and asserted at hand-computed corners against the one-coordinate dfn; basin flood, peak spawn, clamp-boundary slope.
- `063-board-literal.bqn` — ex34/§26: exactly 64 glyphs, no separators — the shape must consume the literal; a separatored 71-char literal is rejected, never stripped; pieces and cells match the chess start position.
