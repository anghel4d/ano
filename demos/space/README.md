# space

Part IV demos: the lattice as a column store — coordinate predicates, computed lines, spatial folds and scans, density replication, derived fields, and the board literal, each run to hand-computed post-states.

- `27-lattice-patterns.bqn` — ex27/§20: checkerboard, diagonal, triangle, stripes as predicates on the coordinate columns; masks match the outer-product forms and counts.
- `28-computed-line.bqn` — ex28/§21: Version B canonical (`offset = fib(index)`); one shift-add barrier step over a fresh line stays zero, never Fibonacci; the spiral zips index-angle pairs, not a catenated 24-list.
- `29-spiral-assign.bqn` — ex29/§21: phyllotaxis positions from the selection's own index, scattered onto Coin rows only; carries the pair-zip fix.
- `30-space-reductions.bqn` — ex30/§22: scoped folds, plus BOTH cost scans — the leading-axis `+\` and the true `scan2(+)` summed-area table — asserted cell-by-cell and asserted unequal.
- `32-density-fields.bqn` — ex32/§24: count fields replicate cells into populations; the fertility mask filters before the counts replicate.
- `33-derived-fields.bqn` — ex33/§25: ridge corrected to `sin(x/8)+sin(y/8)` and asserted at hand-computed corners against the one-coordinate dfn; basin flood, peak spawn, clamp-boundary slope.
- `34-board-literal.bqn` — ex34/§26: exactly 64 glyphs, no separators — the shape must consume the literal; a separatored 71-char literal is rejected, never stripped; pieces and cells match the chess start position.
