# kore/

kore. これ, "this one here" — ano addresses the world from over there; the editor is where you hold it in your hand. A TUI over the process and text boundary: kore spawns anoc as a child exactly as anoc spawns cbqn, and its data contract is the .reg format. It never includes ano.h, so it survives the Steel port untouched. One file, `kore.c`, C23, zero deps — raw ANSI CSI rendering over termios raw mode, double-buffered into one write(2) per frame, SGR mouse reporting, SIGWINCH resize, no ncurses. CJK, kana, and fullwidth codepoints occupy 2 cells.

Build: `make -C kore`. anoc discovery: `$ANOC` wins, else `../src/anoc` beside kore's own binary, else `src/anoc` under CWD, else `anoc` from PATH. Runs need `bqn` on PATH (the nix dev shell provides CBQN).

## Entry points

- `kore <file.reg>` — the bare world, REPL-only: no rail, no code surface, the prompt focused.
- `kore <file.ano>` — the demo form: code, world, output, prompt.
- `kore` — the rail: every .ano under `demos/` (walked from CWD at startup), scrollable, selectable, runnable.
- `kore --check <file.reg>…` — headless verification: load each world, render table and space to memory, one ok/FAIL line each, nonzero on any failure.
- `kore --edit <file.reg> <seg> <row> <col> <value>` — headless cell edit, the same splice the inline edit performs (guard and undo bypassed — point it at a copy).

## The tick

Each prompt submission is one program against the current world: kore writes `.kore/repl.ano` — `--! registry <world>`, the session's defs, then the one statement — and runs `anoc --run --save <world>`. One gather, one barrier, one scatter, then the world file advances (staged, rename(2), atomic) and the panels redraw from the advanced file. On failure the compiler's error lands in the output surface and the world stands. Defs persist across submissions exactly as the session log replays them: every successful `def` joins the session (same name resubmitted replaces it) and is prepended to each later same-surface submission, so `def kin = #/ (moore' & Planted)` submitted once serves every later statement, and resubmitting `Plot , Planted = kin == 3 | Planted & kin == 2` steps the world tick by tick — with the space up, the glider walks. The session log (`session.ano` beside the world) records every successful statement over a `session-base.reg` snapshot of the world as it stood before the first one, so the log is a valid .ano program that genuinely replays; reopening a world rehydrates its defs from the log — the log is the session's memory. A Japanese statement takes the `ja ` prefix at the prompt; a session keeps one surface (`--! ja` is file-wide), and a statement on the other surface logs as a comment, marked not replayable. One anoc limit surfaces here: the `--! registry` directive is one word, so a world whose absolute path contains a space cannot be addressed by the REPL.

## The surfaces

- demos — the corpus as a rail; Enter or click-on-selected opens, r runs. The rail runs demos read-only: r saves the post-state to `.kore/post.reg` and the world surface shows it labeled (post-state); the demo's own registry is never written.
- code — the loaded .ano, edited in place, vi-flavored: browse with hjkl/arrows, `i`/`a`/`o` insert (ESC leaves), `x` delete char, `d` delete line, `s` save. Saving is always explicit — `r` refuses on unsaved changes rather than write the file under you, and saving a corpus file announces itself in the output surface. Comments and directives dim; the hinge comma and `=>` glow.
- world — the registry's data whole: every column in world order, presence gaps dimmed to `·`, rel columns as row indexes with `/` the visible none, alias lines as their stored mask bits, srel fibers, lattice fields in their w×h shape below the table, kanji names at correct width. The cell cursor reaches every value; Enter opens the inline edit (Enter commits, ESC cancels). A rel cell accepts `/` for none; a vec cell takes `x y`; an srel cell takes the whole fiber (numbers only); a char cell takes one printable ASCII byte and refuses a write the loader could not read back (a boundary space, a word-boundary `#`). Edits splice one word in the .reg text — comments and layout pass through untouched, never regenerated.
- space — `m` toggles table ⇄ space: the world as a glyph grid looked down at. char fields draw their exact glyphs, bool fields block-paint in a per-field color, num fields shade ░▒▓█ against their max (coordinate fields x/y skip — they would drown the picture), positioned entities (the pos role, else the literal `pos` vec column) stand on their cells as `@`. The cell cursor and mouse work on the map exactly as on the table; Enter edits the field that painted the cell.
- output — child stdout/stderr verbatim (`--! out` prints land here), and the one verdict line: pins held, which statement failed, or the save/undo status.
- the prompt — the drawing's `>` line, reachable from anywhere with `:` or `>`; arrow-up history within the session; ESC leaves.

## Keys and mouse

Global (outside text entry): `q` quit, Tab cycle focus, `:` or `>` the prompt, `m` table ⇄ space, `r` run, `u` undo, `w` snapshot, `E` the $EDITOR hop (opens the world under world focus, else the .ano; reloads on return), j/k or arrows scroll. Inside the prompt, an edit, or insert mode, keys are text; ESC leaves first. Mouse: click focuses and cursors, click a selected rail entry opens it, wheel scrolls the panel under the pointer, click the prompt to type, and dragging across world rows or space cells paints a selection whose predicate skeleton pre-fills the prompt — `index >= a & index <= b , ` over rows, `w h & x >= … & y <= … , ` over cells (registered x/y fields are sampled so either row/column convention lands right). Pointing at the world is the language's founding gesture; kore takes it literally.

## Mutation and the ring

In the demo form the first mutating act (statement or cell edit) copies the current world to `.kore/play/`, says so in the output surface, and retargets the session there — the demo's own registry never mutates under kore. A bare `.reg` world mutates in place, unless it sits under a `demos/` tree, which always copies: the corpus is immutable. Every world advance first copies the pre-state to `.kore/undo/<key>-<seq>.reg`; `u` steps back. Scratch names carry `<key>` = the world's stem plus an 8-hex hash of its absolute path, so two worlds sharing a basename never share a ring. The ring is files, so it survives kore itself — reopening a world resumes the count. `w` writes `.kore/<key>-snap<seq>.reg`. All scratch lives under `.kore/` in the CWD; the working file is always committed truth (every write is staged → rename).
