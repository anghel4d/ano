# anoc — Patch Notes

## Snapshot 26w28f — 2026-07-11 — "The Corpus Learns to Count"

The corpus normalization: todo/03, the tenth and pending task of the demo-pass review, lands. Four rulings executed as one pass over every demo file: the 1/08 rel rename, the s05 split, the teaching re-annotation, and the global renumbering — every demo now carries one absolute ascending number, 001 through 132, and the old prefix zoo is retired to provenance markers.

> Emitted BQN Format: **unchanged in form** — a comment-and-rename pass; comments never reach the emitter. Demo 008's fixture nouns renamed (faction → leader), so its emit follows; the five new quads emit fresh; everything else byte-identical, ja conjugates still emit byte-identical to their ASCII twins.
> Registry Format: **unchanged**. Fixture files renamed to match their demos; one rel renamed inside 008-relationship-hops.reg.

### New Features

- **The set-hop family, taught in parts (010-013).** The old s05 packed four sub-worlds behind archetype masks; each independent idea now runs alone in its own small world with re-derived pins — 010 the fiber image as a mask plus in-degree in the gamma fold, 011 the any/count neighbor quantifiers with the one-ring spread, 012 the all-quantifier and the vacuously certified empty pen — and 013 keeps the original 23-row world as the wrap-up.
- **The soul gate (025).** The barrier-granularity spawn made loop-safe, as ruled: `Nord & Dead & Soul , Soul = 0 ; spawn Ghost` — the fuel is consumed in the same barrier that burns it, minted ghosts take the type-zero soul, so the step is linear and terminating; the .bqn pins the fixed point, Step ∘ Step ≡ Step. The barrier-granularity original keeps its exponential growth deliberately: that is the pinned point, not a bug.
- **Teaching headers across every demo.** Every .ano now opens with a paragraph saying what world is being set up and introducing its operands, K&R the register. The ruled specifics all landed: `@` is not "where", it is "within", with the empty-scope identities beside it (028); the dyadic-max carry illustrated over two columns (029); the associativity refusal stated formally, ending at "/ and - are not possible to fold-reduce because they are not associative" (030); what "identical keys" pins across the two replicate spellings (049); the `_` free axis (051); the barrier `;` as "and at the same time" with BQN/APL/Haskell renderings (019); "; is not 'then', it's 'and', in the same breath" at the swap (024). The author's own comments stand verbatim.

### Changes

- **The global renumbering.** One counter across demos/ in folder order, 001-132, three digits so lexicographic order stays numeric past 099; the sNN/cN/nN/wN/tN/rN prefixes fold into the sequence and survive as `(was …)` markers on 68 title lines; shared .bqn witnesses take their family's first number; the two BQN-only witnesses number at their folder positions (053-keygen, 072-reverse); every registry renamed to match its demo, references rewritten in 386 files. `.kore/play` tags key off realpath, so pre-rename play dirs are orphaned — acceptable, scratch by design; the seven archived hand-edit copies sit in kore/saved, `>reset`-proof.
- **faction → leader in 008** (`rel leader`, ja 頭領). The demo's "faction" was a pointer to an individual Nord — nonsense under that name. The spec's ex8 faction stays as written: it names a faction archetype, a genuinely different kind of entity.
- demos.md and the series READMEs navigate by the new numbers; the manual's suite counts and citations follow; todo/ closed out — 01-10 archived out of the tree per the .archive convention, the execute-todo item crossed off, 00-open-rulings and 11-next-steps remain.

### Verified

- Both suites green at every one of the four substep commits; final state 93 .bqn witnesses ok and the full check-ano battery ok — every twin, every emit pair, every refusal.
- Every `--! registry` line and every "Twin of" reference resolves post-rename, checked mechanically.
- The five new quads passed all four gates on first run: .bqn witness, `anoc --run`, the ja conjugate, and emit byte-identity.

## Snapshot 26w28e — 2026-07-11 — "The Type Is the License"

The todo/ execution pass: nine task files from the demo-pass review (01–10, 03 pending), landed in one day. The centerpiece is 02, the first genuine advancement of the core design since the initial spec: registry types as declared constraint evidence, and the functional hop made sound against despawn. Around it: every query result finally visible in kore, the universal reducer spelling, the fold/scan permutation table, honest editors, glyph maps and a bitmap, an airtight numeric seal, and the debugger's voice.

> Emitted BQN Format: **unchanged with every flag off** — byte-identical across the corpus, demo 15 excepted (it deliberately gained the `threat/` twin statement). Three flag-gated stdout channels now ride beside each other: `--save`'s 0x1E world lines, `--label`'s 0x1D query tags, `--trace`'s 0x1F diagnostics — one control byte apart, mutually uncontaminable by construction.
> Registry Format: **extended** — `unique`, the keyed `rel <keycol> <name>` form, `def` protos, `reap seal|host`. Every existing .reg loads unchanged: an undeclared rel stays keyed to the row index, zero churn.

### New Features

- **`unique` and keyed relationships** (02). A `unique` column is declared injectivity — pairwise-distinct at load, minted `1+max` on spawn, unwritable by effects — and injectivity is the license the keyed hop needs: `rel <keycol> <name> …` resolves stored ids by index-of against the key with one found-guard, so a despawned target fails the row instead of gathering the wrong entity (PROBLEM 1) or faulting out of range (PROBLEM 2). Not-found is dangling is dead: left-join-null, never a fault path. The inverse of a keyed rel is keyed automatically; keyed srel fibers translate the same way. Twins s58/s59/s60 pin despawn-then-hop in both failure shapes.
- **`def` protos and the three-layer spawn fill** (02). `def Marine soldier=1 hp=100` is the registered archetype, and `spawn Marine` fills proto value → registered `default` → type zero (num 0, bool 0, sym "", rel -1 — None under left-join-null). s61 pins it. `reap seal|host` records the `~` reclamation policy, world-level; the mask meaning of `~` never moves.
- **kore OUTPUTS** (01). The large surface between world and history: labeled query results per tick, newest first, `q1 · +/ threat @ Enemy → 390`, multi-line values indented under their labels. anoc's `--label` tags each query over the 0x1D channel with its statement line; kore resolves tags against the very program it wrote. The old output box is now `history`, a slim strip above the prompt with the verdict line where it always was. Ticks strip `--! out` pins exactly as they strip `--! expect` — the pins witness the pristine run, and a stale out-pin can no longer wedge the world.
- **The universal reducer spelling** (05). `/` attached to any non-keyword name is the fold: `threat/ Damage @ Enemies` beside `+/ Gold @ Nord`, one grammar row, UTF-8 names included (`脅威/`), scans free (`threat\`). `reduce(f)` is cut clean to `fold(f)`, no deprecation cycle. Demo 15 pins both spellings of one meaning; s64 adds the scan.
- **The fold/scan permutation table** (04). The whole level-9 family in one table with folds, scans, empty-scope identities, and the boolean latches (`|\` ever-any, `&\` still-all, demos s62/s63). The original max ruling rejected `>/` and kept `max/`. The 2026-07-21 ruling later adopted q's Greater and Lesser family directly. Implementation is tracked in `todo/17-greater-lesser.md`.
- **Editors worth the name** (07). The E hop walks `$VISUAL → $EDITOR → nvim → vim → micro → nano → vi` — the floor is never vim.tiny again — and the devshell exports `EDITOR=nvim` only-when-unset. Inside: Tab types two spaces in insert mode, the pane title flips a loud reverse-video INSERT chip, and the terminal cursor becomes a real bar via DECSCUSR (block reverse in browse), restored on every exit path. The modeless-vs-vim rewrite waits on its ruling with a keymap sketch filed.
- **The space views** (08). Map cells render two columns wide (~square in any font), entities draw their registered glyph (the glyph role, `@` the floor) with a stable per-archetype color, and `m` now cycles table → map → bitmap — the third view painting the world as an RGB canvas through `▀` half-blocks, two pixels per cell, zero deps. `--check` renders all three.
- **The numeric seal** (09). Numbers are float64 end to end and the registry's value domain is exactly the finite doubles: a hand-written `inf`/`nan` refuses at load with a line diagnostic, mirroring the save's refusal, so load∘save is the identity. `src/refusals/` pins every face; the manual's edges chapter documents the 2^53 cliff, quiet ULP absorption, and overflow as a refused tick.
- **The debugger gets loud** (10). `anoc --trace` (kore: the `t` toggle) emits `RELATION <column> <origin> -> <sink> IS DEAD !` per dead crossing, `FIBER <column> <origin> IS EMPTY !` per identityless empty-fiber row, and one `s<N>: <before> rows -> <after> (spawn <Proto>: +k, kill: -j)` line per structural statement — over 0x1F, into kore's history, never OUTPUTS. Off by default, not a byte of emit changes when off, and post-state is byte-identical traced or not. The repro channel is documented: the session log plus the play registry IS the bug report.

### Changes

- The spec records the pass's rulings where they execute: generational tombstoning confirmed (ano-ecs §2, gen-width note included), the keyed hop in §5, the three-layer fill in §9, types-on-algebra in the addendum (Wadler–Blott lineage), the `!`/`^` tabulation in the appendix, the reap-granularity open question. Deferred decision points surfaced as Q10–Q17 in todo/00-open-rulings, never resolved silently. The 2026-07-21 rulings later blessed bare-rel foundness, made identityless empty folds produce no query row, kept DEAD as the single missing-target diagnostic, and generalized the long-form fold/scan head to every semantically admitted operator or reducer.
- The registry census in the manual teaches the new kinds in the same breath as the old; GRAMMAR.md's .reg section carries their normative grammar and rejections.
- src/check-ano.sh grew the refusals battery (`ok-refuse` per fixture that must exit 2).

### Fixed bugs

- **ANO-401** — every pinned query computed, asserted, and printed nothing in kore; a kept out-pin over a mutated column wedged the world on tick 2+ → out-pins stripped from ticks, every query visible and labeled.
- **ANO-402** — the functional hop gathered positionally into current row space (`(0⌈rel)⊏comp`): silent wrong-entity below the new row count, hard BQN fault at or past it, latent because no corpus demo hopped after a despawn → keyed resolution through the unique column, found-guard folded into left-join-null.
- **ANO-403** — `idCol`'s no-id fallback reminted `(↕anoN)` at use time, unsound after rows shift → structural effects in a world using the fallback are refused with a diagnostic naming the fix (declare a unique column).
- **ANO-404** — a hand-written `inf` in a .reg loaded and crashed BQN downstream with no diagnostic → refused at load, line named.
- **ANO-405** — `min\`/`avg\` hit the stray-backslash lex error while `max\` fused → the open fusion admits every name; scan cells the emitter refuses stay honest refusals, tabled.
- **ANO-406** — Tab in insert mode was silently dropped; the mode lived in a status-bar whisper; browse and insert shared one reverse cell → typed, chipped, DECSCUSR'd.

### Verified

- Both suites green end to end: 89 .bqn witnesses, 379 check-ano lines (every twin, every emit pair, every refusal), `kore --check` full ok over 109 registries, all three views rendered.
- Emit byte-identity audited against a pre-pass snapshot: 233/235 identical, the two diffs demo 15's deliberate gain.
- Post-state differential for the trace: saved worlds byte-identical trace-on vs trace-off across the corpus; `--label --trace` stacked leaves the 0x1D records and the saved world untouched.
- kore driven under a PTY for the acceptance claims: demo 25's 390 and grade vector in OUTPUTS with the `$ n` line in history, demo 14's census with the empty-scope identities, the trace toggle announcing both ways, no raw control byte ever on screen, demos/ hashed untouched around every battery.

## Snapshot 26w28d — 2026-07-10 — "The Game Loop Under a Key"

The kore session update, plus the first shared C module. `common/` arrives: the anoptic strings module ported whole from anoptic_engine — the 16-byte string value, UTF-8 totality, DUCET collation, interning — with its one adaptation, the mimalloc heap swapped for a bump arena (`ano_arena_t`) under the same region contract. kore stops trusting C strings and starts holding its worlds in arenas. And the world loop becomes what it was always meant to be: repeatedly mutable in place until you write.

> Emitted BQN Format: **unchanged** — anoc itself is untouched in this snapshot.
> Registry Format: **unchanged**. Nothing under demos/ is ever a write target — registries and the demo .ano files both; verified byte-for-byte across the whole battery.

### New Features

- **`common/` — the strings module and its arena** (new top-level directory). `anostr_t`, builders, interning, compile-time SIDs, UTF-8 iteration/classification (UCD 17.0.0 tables), DUCET collation with kana in gojuon order, and `ano_arena_t` — chunked bump regions with per-block size headers so realloc is total and the newest block reallocs and frees in place. `make -C common test` runs the smoke battery. Both kore and (eventually) Cano compile it in directly; no library step, no deps.
- **The world loop: r / n / u.** `r` is load-and-reset — the pristine registry copied over the demo's play scratch, the pre-reset state staged on the ring first so u steps back across a reset. `n` is next — one tick: the demo's program retargeted at the scratch through `--run --save`, its `--! expect` pins stripped (they witness the pristine run; against any later step they would fail the tick and hold the world still), pre-state staged, repeatable indefinitely. `u` is n's exact inverse (the old r staged nothing, which is why undo could never touch it). The game loop is the scan; n is the scan under a key.
- **Per-demo play directories.** `.kore/play/<demo-tag>/` holds each demo's scratch world, session log, base snapshot, and (by tag) undo ring — the `.kore/post.reg` collision that let two demos share a session is gone. Reopening a demo whose scratch exists resumes the play world at its recorded step; r is always the way home. Session logs record `-- n:` / `-- r:` seam lines when the world moves underneath them.
- **Vim vocabulary in the code surface**: `w`/`b` word motions over runes, `gg`/`G`, counts (`5j`, `3dd`, `12G`), `0`/`^`/`$`, `dd`, `/` search with base-letter matching (case- and accent-insensitive, highlights, `n`/`N`), and `u` as code-local undo — a snapshot stack, focus-scoped so the world's ring is untouched. ESC steps out of any panel to the mode's home surface.
- **The prompt in its own box**, above a dedicated status line that carries the context-sensitive key atlas — and statements span lines: `\⏎`, Shift+Enter (kitty CSI-u / xterm modifyOtherKeys), or Alt+Enter break a line, Enter runs the whole body as one program (its def lines join the session individually). The box grows with its lines and scrolls to the cursor; the cursor's line scrolls horizontally; ↑↓ walk lines then history; the wheel walks history.
- **The corpus is immutable, code included — and `>reset` is the way home.** The demo .ano joins the registry under copy-on-write: the first `s` (or `E`) on a demo under demos/ writes `.kore/play/<tag>/<demo>.ano` instead, the buffer rides the copy (the title says `play copy`), reopening resumes it, and `n` ticks the edited program. `>reset` at the prompt (`>` opens it pre-filled with the command form) or Ctrl+Shift+R (kitty CSI-u / modifyOtherKeys) asks — y confirms, anything else cancels — then deletes every play copy with its ring: all demos return to the pristine corpus. Snapshots stay. `--edit` refuses corpus paths outright.
- **Colour.** xterm-256 palette throughout, on a forced dark canvas (bg `234`, ink `252` — pastel accents read identically on light-themed terminals; deriving from the shell theme is TODO item 4): per-panel border accents, .reg value hues by kind (sym lilac, rel salmon, bool teal, char warm), ano syntax tints in the code surface (directives, comments, def heads, numbers, operators, the glowing hinge), origin-tinted output lines, a red verdict when the world stands, violet `-nihongo` twins in the rail, and scroll thumbs on every overflowing panel.

### Changes

- The demos rail sorts by natural collation with extensions stripped: digit runs compare as numbers (`2-` before `10-`, the file-browser order), the stretches between them by DUCET — kana in gojuon order, and `01-x.ano` before `01-x-nihongo.ano`, so the rail no longer opens on the Japanese conjugate because byte `'-' < '.'`.
- The mouse wheel moves one item per notch in every panel (was three) — the view follows a line at a time instead of leaping.
- kore's parsed world state allocates from one arena per load and dies with the load that replaces it — no per-entry frees, no leak surface; the loader's per-line tokenizing scratch included (bounded region garbage instead of two free sites an early continue could miss).
- `kore/Makefile` compiles `../common` in directly; `-std=gnu23`.
- The old "saving a corpus file announces itself" ruling is gone — announced or not, the corpus was still being written. `s` and `E` now guard exactly as the world does, and `E` under world focus guards too (it used to hand the pristine registry to $EDITOR).

### Verified

- `make -C common test` green (arena seams, string round-trips, UTF-8 totality, collation facts including the twin rule).
- `kore --check` over the corpus registries: all ok.
- PTY end-to-end: r → n → n → u leaves the scratch byte-identical to an independently computed one-tick state; repeated n diverges (the world really advances); reopen resumes; two demos hold disjoint scratches; a demo-mode prompt statement logs to that demo's own session; `dd`+`u`+`s` round-trips a file byte-identically; a synthetic rail with 1/2/10/11 directories and 01/2/10 files renders in file-browser order with the twin still second; one wheel notch moves the rail selection exactly one entry and up reverses it; demos/ hashed before and after every battery — untouched; on a synthetic corpus, `s` after an edit leaves the demo byte-identical and lands the edit in `.kore/play/<tag>/`, reopening resumes the play copy, `>reset` cancelled leaves it standing and confirmed removes the whole play tree with the buffer back to pristine, and `--edit` on a corpus registry refuses and mutates nothing.

## Snapshot 26w28c — 2026-07-10 — "The World at Hand"

The editor update. kore (これ, "this one here") is the ano editor: ano addresses the world from over there, and kore is where you hold it in your hand — a zero-dependency TUI that spawns anoc exactly as anoc spawns cbqn and speaks nothing but the process and text boundary, so it survives the Steel port untouched. Under it, anoc finally closes the loop the dump left open: `--save` pipes the post-state back from the bqn child and commits it, so a played world is a file again. The clock is still the install runs; now you can watch it beat.

> Emitted BQN Format: **unchanged** — all 235 pre-existing demos compile byte-for-byte identical to a pre-patch snapshot of every emit. The `--save` serializer is flag-gated and appends after the pins.
> Registry Format: **unchanged** — but `--dump`'s number spelling improved: integral values now write as plain digits (`900`, never `9e+02`). Dumps still fixpoint byte-identically; anything comparing against old dump bytes should re-snapshot.

### New Features

- **Added `--save <path>` to anoc** (requires `--run`). The emitted program gains a serializer that prints the post-state after the pins hold — one line per datum, each prefixed with the 0x1E record separator so no user print can collide: `n`, then per data-carrying entry in declaration order, col/field values, pres bits, rel indexes (-1 the none sentinel), srel fibers. Schema never pipes — fns, binds, aliases, roles, and derived tags are load-side, and inv fibers recompute at load. main.c pipes the child, patches the sentinel lines into the loaded Registry (n may grow on spawn; arrays allocate fresh), and commits through the same staged rename as `--dump`. On a nonzero exit, or any post-state with no .reg spelling, nothing is written. Closes ISSUES.md's "dump is pre-state today".
- **Added kore, the ano editor** (new top-level `kore/`, one file, C23, zero deps — raw CSI and termios, no ncurses; CJK, kana, and fullwidth glyphs at their true two-cell width). Six surfaces: the demos rail (the corpus walked at startup, scrollable, runnable — the author learns the language by walking his own witnesses), the code editor (vi-flavored, hinge and `=>` aglow), the world table (every value reachable and editable in place — presence gaps dim to `·`, a rel's none draws as the drawing's `/`, edits splice one word of .reg text and never regenerate), the space (the world as a glyph grid looked down at: char fields draw their exact glyphs, bools block-paint per-field colors, nums shade ░▒▓█, positioned entities stand on their cells), the output log with one unambiguous verdict line, and the prompt.
- **The REPL line is the commit loop.** Each submission is one program against the current world — `anoc --run --save` advances the file, the panels redraw from it, and on failure the compiler's error lands in the output and the world stands. Defs persist across submissions by prepending the session's defs to each program, exactly as the session log replays them; the log (`session.ano`, over a `session-base.reg` pre-state snapshot) is a valid .ano program that genuinely replays, and reopening a world rehydrates its defs from it. Submit c2's `def kin` once, resubmit the one Life statement, and the glider walks the space view — verified to land generation four byte-identical to the demo's own pin, across a kore restart.
- **Mutation requires a copy, and undo is free.** The demo form copies the world to `.kore/play/` on the first mutating act and says so; every advance stages the pre-state into `.kore/undo/` first — the ring is files, so it survives kore itself. `w` snapshots, `u` steps back, `E` hops to `$EDITOR` and reloads on return.
- **Headless verification hooks**: `kore --check <reg>…` loads and renders every view to memory (all 103 registries walk clean), `kore --edit` performs one cell splice from a script — so the editor's own claims are testable without a human at the keys.

### Changes

- `--dump` number spelling: integers as digits (above); `-0.0` keeps its sign; the fast path guards its range before the cast.
- `numLit` now spells every `-` as BQN high-minus, exponents included, and drops C's `e+` — so a saved world carrying `1e-09` re-emits as `1e¯09` instead of poisoning every later run against it.
- Documentation: `GRAMMAR.md`'s pipeline line and `compiler.md`'s main.c paragraph carry `--save`; `ISSUES.md` closes the dump entry and records the text-format seams this landing surfaced (below); `kore/kore.md` is the editor's own doc — keys, mouse, surfaces as built, the tick semantics; TODO.md crosses the editor item.

### Technical Changes

- `ano.h` — `Directives.save`, the flag-gate the emitter checks.
- `emit.c` — `emitSave` (the serializer block, appended after the expectations only under the flag); `numLit` hardened as above.
- `main.c` — `--save` flag (requires `--run` and a registry); `run_bqn` gains a pipe and a sentinel/verbatim splitter; `save_line`/`save_patch` parse the pipe-back with the loader's own conventions and refuse any post-state the loader could not read back (empty syms, unrepresentable glyph runs, non-finite values, part-pair columns — the error names the column, nothing commits); stored masks reconcile to the new n so the dump always reloads; an exit-0 child that printed no sentinel block is an error, never a silent pre-state save.
- `registry.c` — `dnum` as above.
- `kore/kore.c` — everything else; `kore/Makefile`; `kore/kore.md`.

### Fixed bugs in this snapshot (surfaced during review)

An adversarial multi-agent review ran against the working diff — 6 lenses, 94 agents, 43 findings confirmed by paired refuter/reproducer verification, zero refuted. 36 fixed before publication, the rest recorded (below). The ones that would have bitten hardest:

- **ANO-301** — `--save` exited 0 while committing an unloadable world when despawn left a char run empty, boundary-spaced, or with a word-boundary `#` → refused with the column named, nothing written; kore's char edits validate against the same rule
- **ANO-302** — a division could mint `1e-09` into a saved world that loaded fine and then failed every future run at the fixture line → `numLit` re-reads everything `dnum` writes
- **ANO-303** — every kore splice/save cycle appended one blank line: pass-through violated, files growing forever → the post-final-newline tail is not a line
- **ANO-304** — kore's char payload read overran the heap on an empty run and disagreed with the loader on spacing → one `char_span`, mirroring `strip_line` + raw-tail byte for byte
- **ANO-305** — a REPL def died with its submission, so c2's rule could not step the world from the prompt → session defs, the session log as their memory
- **ANO-306** — `r` silently wrote a dirty code buffer to a corpus demo → saving is explicit, announced, and `r` refuses instead
- **ANO-307** — undo rings and play copies collided across worlds sharing a basename; `u` could restore another world's bytes → scratch names carry a path hash
- **ANO-308** — an exit-0 child that never reached the serializer passed its pre-state off as a successful save → refused
- **ANO-309** — wide-glyph halves overwritten by later draws shifted whole rows; a 0-row terminal wrote out of the grid; `abort()` left the terminal raw in the alternate screen → orphan repair in `put`/`fill`, a floor under the grid, `SIGABRT`/`SIGBUS`/`SIGFPE` in the restore set
- **ANO-310** — input decode: F5-F8 read as Home, oversized CSI sequences leaked their tails as typed keys, Alt+q quit the editor, unknown mouse codes latched phantom drags, ^C during the `$EDITOR` hop killed kore underneath it → all five decoded or ignored correctly
- and 26 more of the same review, from cast-before-guard UB on hostile .reg scalars to session logs whose registry header pointed at a world that had already advanced (now `session-base.reg`).

### Known Issues

- The text-format seams, recorded in ISSUES.md: a played world can hold values the .reg format cannot spell (empty syms, certain glyph runs, `∞`, part-pair columns) — `--save` refuses these rather than commit; stored masks reconcile by pad/truncate across structural change and a truncated mask can mark the wrong rows; a vec column despawned to n 0 re-spells as `num`.
- Spawning rows into a world with an entity char column has never had a default glyph (the emitter appends numeric defaults to a BQN string); pre-existing, corpus-free, and under `--save` it fails the run cleanly rather than corrupt anything.
- A registry name containing `"` breaks the serializer's BQN string literals — the save fails, nothing commits. Loader-legal, pathological, unhandled.
- The `--! registry` directive is one word, so kore cannot address a world whose absolute path contains a space; kore says so at the prompt.
- A mixed-surface session logs other-surface statements as comments (one `--! ja` per file) — recorded in the log as not replayable.

### Behind the Scenes

- Test suite: 235 demos green under `--run`, 116 conjugate pairs byte-identical under `--emit`, zero failures — and every one of the 235 emits byte-identical to the pre-patch snapshot, re-verified after every emitter touch.
- The verification battery from EDITOR.md ran in full: s57a saves `gold 900 600 700 650 221`, `master 2 -1 -1 4 -1` (the plan's `/` is the drawing's none — kore renders it, the file spells -1), the saved world fixpoints under `--dump`, a spawn demo's saved n grows to 17, the nihongo world round-trips its kanji columns exactly, a failing program writes nothing and leaves no staged file, all 103 registries walk and render, and a cell edit round-trips on a copy with comments preserved verbatim.
- kore was driven under a pty for every claim above — all three entry forms, the REPL advance, undo, the demo run's post-state view, the space view, and the glider's four-generation walk.
- Nothing was committed or pushed at publication. Suggest archiving EDITOR.md to `.archive/` when this lands; PATCHES.md stays, it is the running snapshot ledger.

## Snapshot 26w28b — 2026-07-08 — "The Registry Is the API"

The case-contract update. One rule, no exceptions: registry names are case-insensitive (ASCII fold, non-ASCII bytes exact); everything else — values, defs, binders, every program variable — is case-sensitive. The whole compiler now enforces it through one comparator. Registries learned the `as` directive, roles resolve through one path, and your world can finally save itself: `--dump` writes it back out, atomically. Both findings from 26w28a's verification report are closed.

> Registry Format: now accepts `as` lines — name aliases and derived tags. Loads got stricter: two names that fold together are one name and rejected, as is any name the lexer owns under the fold. Measured against all 100 registries before shipping: nothing live rejects.
> Emitted BQN Format: **unchanged** — all 229 pre-existing demos compile byte-for-byte identical (verified against a pre-patch snapshot of every emit).

### New Features

- **One comparator.** `names_eq` — ASCII letters fold, every other byte exact — now serves every registry name resolution: entries, aliases, role targets, the loader's own directive lookups. `GOLD` finds `gold`; `:Nord` still refuses `:nord`, because sym values are values; `def ridge` beside a column `Ridge` is two names, because defs are program variables, not registry nouns. Kanji names are untouched by construction.
- **Added the `as` directive, both arities.** `as <word> <name>` is a pure name alias with `ja`'s exact semantics — one hop, no transitivity, outranked by real entries; both spellings fill one table, and `ja` survives because it documents the Japanese surface at the declaration site. `as <word> <col> <value>` declares a derived tag: the word names the equality mask over the live column (present ∧ col = value), recomputed at each use. Write `as nord race Nord` and `Nord` selects Nords without a stored bit anywhere. Derived tags are read-only as effect targets — the write spells `race = :Nord` — and the error message points you at the carrier.
- **One resolver for roles.** `reg_role`'s literal-name fallback now calls `reg_find` — entries, then aliases, one path in the whole compiler. A declared `role` line remains the explicit override whenever plumbing must be pinned. Closes 26w28a verification finding #1 (the fallback used to drop the alias hop).
- **Save files.** `--dump <path>` serializes the in-memory world back to .reg text — everything `reg_load` reads, round-trippable — through a staged file and rename(2): the atomic commit, crash-safe saves for free. load → dump → load → dump fixpoints byte-identically on all 103 registries, and a dumped world passes the same pins as the in-memory one. The world is a column store, so a save is a registry dump.
- **Added 6 new demos and 3 new world fixtures:**
  - `s56-native-proto-id` (demos/9-nihongo) — the last two roles proven load-bearing: set-hop membership reads the `role id` kanji 識別 (prime ids, so row indexes would miss) and a board spawn writes computed pieces through `role proto` into the kanji 種別. Delete either role line, a pin fails. With s54, all five system roles are witnessed. Closes 26w28a verification finding #2.
  - `s57-derived-tag-{a,b}` (demos/12-registry-forms) — one program, two registries: a stores the tag as a bool column, b derives it from a race column via `as`. Identical pins across both — DATAMODEL.md's representation-independence claim as a running test. Delete the `as` line and the program fails at compile.

### Changes

- Loads are strict within name kinds: entry-vs-entry and alias-source-vs-alias-source fold collisions are load errors, and the reserved check folds (`col Til` rejects exactly as `col til`). Alias-vs-entry is deliberately not rejected — entries outrank aliases, that is deterministic shadowing, and the seven `ja x x` identity-alias worlds depend on it.
- Defs stay case-sensitive. Ruled by the author mid-landing, superseding the def-fold ruling recorded in REGFIX.md: the fold is registry names only — defs, binders, and every other program-level variable match exact-byte, exactly as before this patch. An earlier draft folded defs and renamed two demo rule labels to dodge the new collisions; the folding and the renames are both reverted, the corpus stands untouched. The silent 129th-def drop is now a proper error while we were in there.
- `--! same-tokens` only routes NAME/ALIAS payloads through the resolver now; sym, string, and counter payloads are values and compare exact bytes — the full fold would otherwise have judged two distinct sym values token-equal through name resolution.
- Documentation: `GRAMMAR.md` states the case contract and the word-class boundary in the Names policy, and gains a Registry files (.reg) section — every line kind, the `as`-vs-`alias` distinction (name alias vs stored mask value), the shadowing rule, the load rejections; `compiler.md`'s resolution paragraph carries the contract in one line and the invariants gained "Registry names fold, values never"; `src.md` and `ano.h`'s format comment updated to match; `ISSUES.md` records the two tradeoffs this patch surfaced instead of resolving (below).

### Technical Changes

- `ano.h` — `RK_TAG` entry kind and `RegEntry.tagCol`; the alias table generalized to `asFrom`/`asTo`/`asJa`/`nas` (the `asJa` byte keeps the declared spelling so dumps round-trip the surface); `names_eq` and `reg_dump` declared; `fs_write_commit` declared; the `.reg` format comment extended for `as` and the case contract.
- `registry.c` — `names_eq`; `wuniq`/`wuniq_alias` (the load rejections); `find_exact` → `find_ent` (names_eq, serving pres/default/inv/role/as targets); the `as` branches; `reg_find`/`reg_role` rewritten over the comparator, the role fallback delegating to `reg_find`; `reg_dump` with shortest-round-trip number formatting (format∘parse∘format = format, so dumps fixpoint).
- `lex.c` — `lex_reserved_fold`, the reserved check under the fold: exact first, then once more on the ASCII-folded word. Only the loader consults it; `lex_reserved` stays exact for the parse-level def-head check, and the lexer's own tables stay exact — folding here never makes `Def` lex as the keyword, it only bars `Def` from naming a registry entry.
- `emit.c` — `RK_TAG` emission in mask and value positions (carrier presence folded in per the left-join-null rule) and rejection in effect and expect positions; the 129th-def overflow error. `findDef` and the binder compares stay exact strcmp — program variables live outside the comparator.
- `main.c` — `--dump` flag (dumps right after the registry loads; alone it stops there, with `--run` the pipeline continues); the `same_stream` NAME/ALIAS gate.
- `fs.c` — `fs_write_commit`: write `<path>.staged`, fsync, rename(2) over the target; staging beside the target keeps both on one filesystem, so the rename is atomic. The one writer, next to the one reader.

### Fixed bugs in 26w28b

- **26w28a finding #1** — `reg_role`'s literal-name fallback dropped `reg_find`'s alias hop → the fallback IS `reg_find` now; one resolution path.
- **26w28a finding #2** — `role proto` and `role id` were routed but witnessed nowhere → s56 proves both load-bearing.
- The first-letter-only fold — too narrow to be the rule, too magical to be no rule — is gone in both directions: full fold for names, exact bytes for values, nothing in between.

### Fixed bugs in this snapshot (surfaced during review)

An adversarial multi-agent review pass ran against the working diff — 5 lenses, 23 raw findings, each judged by 3 independent refuters; 19 confirmed, 4 refuted. All 19 resolved before publication:

- **ANO-201** — Hop onto a derived tag (`mentor.nord`) emitted an unbound BQN identifier: anoc exited 0 and the program died at runtime, and hop position could tell the s57 twins apart → both hop branches now expand the tag's recomputed mask, so representation independence holds through hops too
- **ANO-202** — `spawn <tag>` silently spawned unmarked rows; the read-only-tag rule had a hole at the spawn-proto path → rejected with the same carrier-naming error as assignment and presence writes
- **ANO-203** — A derived tag over a vec column loaded cleanly, then crashed inside BQN at every use → rejected at load beside the char-carrier rule
- **ANO-204** — A registry redeclaring `n` or `lattice` mid-file loaded (each line validated against the count current at its line) but dumped an unloadable one-header file, and a stale `pres` could read out of bounds → one header per world: redeclaration is now a load error
- **ANO-205** — `--emit --dump` silently printed nothing, because `--emit` was an untracked no-op flag → an explicit `--emit` now survives `--dump`
- **ANO-206** — Four doc sentences overclaimed: GRAMMAR.md said entries shadow all six contextual specials (`index` and `char` never shadowed and still don't) and omitted the exact-matched builtins `rank`/`abs`/`sin` from the closed grammar; compiler.md implied defs resolve after the registry; the .reg section said "n-glyph tail" where the loader counts bytes → all four now state exactly what the code does
- **ANO-207** — demos.md's 12-registry-forms bullet went stale against the s57 placement → updated

### Known Issues

- `--dump` writes the loaded world: post-state lives and dies inside the bqn child (exit-code-only, nothing pipes back), so "dump the post-state" waits for a world that lives in C memory. The fixpoint and pins axes hold as specified; recorded in ISSUES.md.
- `demos/demos.md`'s 9-nihongo bullet still enumerates the native witnesses as s50-s53 — stale since s54 landed, untouched here because REGFIX names no other doc targets.

### Behind the Scenes

- Test suite: 235 demos green under `--run` (up from 229), 116 conjugate pairs byte-identical under `--emit` (up from 113), zero failures. All 229 pre-existing demos emit byte-identical BQN to the pre-patch snapshot — the fold widened resolution without moving a single byte of output.
- The corpus measurement in REGFIX.md was independently re-verified before a line of C changed: zero fold collisions, zero duplicates, zero reserved-fold hits across all 100 registries, and exactly the seven claimed identity-alias worlds. The one gap — it covered registries, not defs — is moot under the final ruling: defs never fold.
- Nothing was committed or pushed at publication. Suggest archiving REGFIX.md when this lands; PATCHES.md stays, it is the running snapshot ledger.

## Snapshot 26w28a — 2026-07-07 — "That One Over There, Part II"

The あの native-noun update is now feature-complete. This snapshot clears the entire #2–#8 issue backlog on `feature-jp-lexer`: a world can be declared, spawned into, sigil-matched, and pinned entirely in kanji, and the design docs finally admit what the compiler has quietly been doing. Also, mobs spawn with a parent again in fully-native worlds. Sorry about that one.

> Registry Format: now accepts `role` lines. Fully backwards compatible.
> Emitted BQN Format: **unchanged** — every existing world compiles byte-for-byte identical (verified against a pre-patch snapshot of all 114 ASCII demos and 111 conjugate twins).

### New Features

- **Added the `role` block to registries.** You can now point a system role — `keys`, `id`, `parent`, `proto`, or `pos` — at any column, including a natively kanji-named one. Write `role pos 位置` and the spawn engine routes `at`-positions into `位置` exactly like it does for a column literally spelled `pos`. No `role` line? Everything routes by literal name as before.
- **Sigils now speak UTF-8.** `^alias` and `:Sym` accept non-ASCII names on both the ASCII and Japanese surfaces. You can finally write `:山賊` and `^世界`. Bandits rejoice.
- **Glyph columns can be pinned directly.** `--! expect 印 = .#..#.` now works — a char column is compared as its exact glyph run instead of being handed to the numeric comparator, which previously just fell over.
- **Added 4 new demos and 2 new world fixtures** under `demos/9-nihongo/` and `demos/registries/`:
  - `s54-native-roles` — spawns positioned, keyed, AND parented rows into an all-kanji world (each role proven load-bearing).
  - `s55-native-sigils` — kanji aliases and symbol values, plus a directly-pinned glyph column.

### Changes

- System-column resolution is now consistent across all five sites in the emitter. Previously `idCol` matched case-insensitively while the four spawn sites matched exactly; now they all use the same case-insensitive-first-letter rule as `reg_find`.
- The key-mint expression references the actual keys column variable instead of hardcoding the literal `keys`, so a native keys column mints ids correctly.
- Documentation was brought in line with the shipped compiler:
  - `ano-language.md` — the generate primitive now records its settled surface spelling `til` (the `↕` glyph stays on as a label, beside `⥊`/`to`).
  - `ano_nihongo.md` — recorded the settled fold/scan words, the `非`-as-identifier negation collision, the per-surface numeral policy, and a post-branch note that the future unspaced reader must resolve nouns AFTER lexing.
  - `GRAMMAR.md`, `compiler.md`, `src.md`, and the nihongo `README.md` — updated for `role`, UTF-8 sigils, and char pins.
  - `ISSUES.md` — the #2–#8 STAR analysis is retired to a resolution ledger; the standing open-questions list is carried forward.

### Technical Changes

- `ano.h` — added `ANO_NROLES`, the `Registry.roleName` / `roleCol` / `nroles` fields, and the `reg_role` declaration; documented the `role` line in the `.reg` format comment.
- `registry.c` — parses and validates `role <name> <col>` lines (unknown role names and non-column targets are rejected at load); added `reg_role` (declared role → column, else literal-name fallback).
- `emit.c` — routed `idCol`, parent-fill, key-mint, proto, and position through `reg_role`; added the `CT_CHAR` branch to the expectation emitter.
- `lex.c` — added the `nstart_span` helper and rewired all four sigil entry points (`:` and `^`, on both the ASCII and JA surfaces) to accept UTF-8 identifier names.
- Build unchanged: `-flto` was already in the Makefile CFLAGS. No caching, no incremental machinery, as promised.

### Fixed bugs in 26w28a

- **ANO-2** — Spec spelled the generator with the retired `↕` glyph as if it were a surface token
- **ANO-3** — Open-questions list still claimed the JA fold/scan forms were "not yet chosen" (they were chosen, and the whole corpus uses them)
- **ANO-4** — Adopting `非` as a negation prefix would silently collide with kanji nouns; the tradeoff was unrecorded
- **ANO-5** — Spawn machinery only recognized system columns literally spelled `keys`/`parent`/`proto`/`pos`, so a kanji-named position column never received spawn positions and worlds had to fake it with an explicit assign
- **ANO-6** — `^alias` and `:Sym` rejected every non-ASCII name, so kanji symbol values were unwritable on both surfaces
- **ANO-6B** — `--! expect` had no char-column branch; a glyph column fell into the numeric comparator and crashed the assertion
- **ANO-7** — Unspaced-reader plan described a lex-time registry dictionary, contradicting the shipped registry-blind lexer invariant
- **ANO-8** — Per-surface numeral policy (kanji numerals read as numbers only under `--! ja`) shipped with no note

### Fixed bugs in this snapshot (surfaced during review)

An adversarial multi-agent review pass (4 dimensions, 11 agents, plus one independent verifier) found these regressions and weak spots introduced while fixing the above. All resolved:

- **ANO-101** — `idCol` silently stopped resolving capitalized `Id`/`Keys` columns after the role refactor → fallback now mirrors `reg_find` exactly
- **ANO-102** — `s54` witnessed only 2 of 5 roles; `parent` had no test → now witnesses pos/keys/parent, each proven load-bearing
- **ANO-103** — `s54`'s parent pin coincided with the default fill and so didn't actually exercise the role → carrier moved off row 0 (`親 = -1 -1 1`), now fails without the role
- **ANO-104** — Char pins with consecutive or edge spaces are silently unrepresentable → documented in-code (the demo glyphs are dot/hash)
- **ANO-105** — Two standing open-questions ("Rule ticks are frame-homogeneous", "The clock is the install runs") had gone missing from `ISSUES.md`, one of them cross-referenced by `GRAMMAR.md` → restored to the committed baseline

### Known Issues

- Duplicate `role` lines are accepted first-wins with no warning. This is intentional — every other registry directive (`col`, `rel`, `alias`, `ja`) behaves the same way — but declaring `role pos` twice means only the first counts.
- Glyph columns containing literal spaces still cannot be pinned (the `--!` tokenizer collapses whitespace). Use dots for empty cells, as the demos do.

### Behind the Scenes

- No existing world was harmed: all STABLE demos are untouched and emit byte-identical BQN.
- Test suite: 229 demos green under `--run`, 113 conjugate pairs byte-identical under `--emit` (up from 225 / 111).
- Nothing was committed or pushed at publication; the author landed the snapshot 2026-07-08 as 178172b ("Plans within plans"). Grab it by checking out `feature-jp-lexer`; back up your world first, because this one does not commit itself.

### Verification Report — 26w28a audited against the working diff

Audited 2026-07-07 against the uncommitted working tree on `feature-jp-lexer` (baseline commit 9b0edd6). Method: every claim above was checked against the actual `git diff`, a fresh `-Wall -Wextra` rebuild, the full harness, a pre-patch binary built from HEAD in a throwaway worktree, and hand-run negative fixtures. Verdict: the snapshot is accurate — every checkable claim holds, the diff contains nothing these notes do not mention, and the audit surfaced one corner-case behavior change the notes miss (finding #1 below).

#### Verified

- Build: the six-file `cc -std=c23 -O2 -Wall -Wextra` build completes with zero warnings; `-flto` is confirmed pre-existing in the Makefile CFLAGS and the diff touches no build file.
- Suite: `check-ano.sh` reports exactly 229 ok and 113 ok-emit, zero failures — the claimed numbers, the s54/s55 pairs included.
- Emit stability: a pre-patch anoc built from HEAD and the working-tree anoc emit byte-identical BQN on all 225 pre-existing demos (114 ASCII + 111 nihongo), zero diffs. The byte-for-byte claim holds.
- The `role` block: parsing, the five role names (`known[]`, registry.c:346), rejection of unknown roles (`unknown role 'foo' (keys id parent proto pos)`) and of non-column targets (`role: no column '出現'`), and first-wins duplicates (a second `role pos` at a different column is ignored, the demo stays green) — all confirmed by hand-run fixtures. `ANO_NROLES`, the three `Registry` fields, the `reg_role` declaration, and the `.reg` format comment are in `ano.h` as listed.
- Routing: all five emitter sites go through `reg_role` — `idCol`'s id/keys probes (emit.c:157-158), parent-fill (emit.c:1305), key-mint (emit.c:1348), proto (emit.c:1356), pos (emit.c:1357). The key mint emits the actual column variable (`cur` from `bqnv`), not the literal `keys` — confirmed in code and behaviorally: s54's 鍵 mints 21, and a synthetic `col Keys` world mints through the case-insensitive fallback (the ANO-101 fix, exercised).
- Load-bearing roles (ANO-102/103): deleting any one of s54's three `role` lines flips the demo to a BQN assertion failure — pos, keys, parent each individually proven. The carrier sits on row 1 (`生成器 0 1`), so `親 = -1 -1 1` is reachable only through the parent role, exactly as ANO-103 claims.
- Sigils: `nstart_span` added and all four entry points rewired (`:`/`^` in `lex_ascii`, `:`/`^` in `lex_ja`); s55 witnesses `:勇者`/`:賊軍`/`^照準` on both surfaces; a bare `:` and a `^` before a blacklisted codepoint still fail with the pre-patch messages.
- Char pin: the `CT_CHAR` branch emits an exact glyph-run comparison; corrupting s55's `印` pin fails the run; the whitespace limitation is documented in the emit.c comment (ANO-104).
- Docs: all listed edits — `ano-language.md` (`til`), `ano_nihongo.md` (folds/scans, the 非 collision, the numeral policy, the unspaced post-branch note), `GRAMMAR.md`, `compiler.md`, `src.md`, the nihongo README, `ISSUES.md` — are present and consistent with the shipped code. `compiler.md`'s status line reads implemented.
- ANO-105: both restored open questions ("Rule ticks are frame-homogeneous", "The clock is the install runs") are present and byte-identical to the committed baseline; the carried-forward section differs from baseline only in its intro's witness range (s50-s53 → s50-s55), which is correct.
- No drift on the standing negatives: と rejected at load, `金 に 千 + 三 たす` still errors, an unknown kanji word errors by word, an unknown byte by codepoint (`unknown byte 0x24`) — every message byte-identical between the pre-patch and patched binaries.
- The multi-agent review pass is a process claim and leaves no artifact in the diff; it was not re-run. Its five ANO-10x findings were each re-verified independently here and all hold, which is the part that matters.

### Findings

#1 — reg_role's fallback drops reg_find's ja-alias hop
- S: Pre-patch, `idCol` resolved the stable-id column through `reg_find`, whose fallback ends in the registry's ja-alias hop; the role refactor rerouted it through `reg_role`, whose literal-name fallback stops at the entry table. The ANO-101 line above says the fallback "mirrors reg_find exactly" — it mirrors the first-letter rule, not the alias hop.
- T: Decide whether a `ja id <col>` alias should still steer `idCol` (restore the hop) or whether `role id <col>` is now the only sanctioned spelling (bless the divergence in one doc line). [Done — 26w28b restored the hop: `reg_role`'s fallback is `reg_find`.]
- A: Verified the divergence empirically: a world with `ja id 識別`, no literal id/keys column, no role line, and a key column in relation position emits `{/bind=𝕩}¨jp0` pre-patch and `{/bind=𝕩}¨(↕anoN)` post-patch. Grepped the corpus: no registry uses `ja id` or `ja keys`, and the 225-demo byte-identity stands, so nothing live is affected. Not fixed — a design call, surfaced per doctrine, carried into NEXT-2.md.
- R: Either `reg_role`'s fallback gains the ja hop (one loop, mirroring registry.c:375-380) or compiler.md/GRAMMAR.md states that roles route by declaration or literal name, never through ja aliases.
- (a) Issue IS: src/registry.c:387-399 (`reg_role`, fallback loop without the ja hop `reg_find` has at src/registry.c:375-380); the inexact sentence is this file's ANO-101 line.
- (b) Affects: src/emit.c:157-158 (`idCol`), reached from the set-hop sites src/emit.c:359 (`fiberVar` over a key column) and src/emit.c:942 (`AnoImage` membership). Repro registry: `n 4` / `col 識別 num 3 4 5 6` / `col bind num 3 3 5 5` / `col mark bool 0 1 0 1` / `col damage num 0 0 0 0` / `ja id 識別`; program: `mark , damage += (#/ (bind' & mark))`.

#2 — role proto and role id have no witnessing demo
- S: registry.c accepts five role names and emit.c routes all five; s54 declares and load-bearingly witnesses three (pos, keys, parent). proto and id are routed but never exercised through a `role` declaration anywhere in the corpus.
- T: Witness the remaining two: a spawn-from-proto into a kanji-named proto column, and a set-hop membership over a kanji-named id column (which also covers #1's corner from the sanctioned side). [Done — 26w28b: s56-native-proto-id.]
- A: Confirmed by grep: `role proto`/`role id` appear in no `.reg`; the proto route and the idCol id-probe execute only their literal-name fallbacks in every demo. Not added here — new demos are new scope; recorded for NEXT-2.md.
- R: An s56 pair whose proto and id roles are each proven load-bearing the way s54's are: delete the role line, a pin fails.
- (a) Issue IS: coverage gap — src/registry.c:346 admits five names; demos/registries/s54-native-roles.reg:13-15 declares only pos/keys/parent; no other `.reg` carries a role line.
- (b) Affects: src/emit.c:1356 (proto routing) and src/emit.c:156-161 (`idCol`) — both untested under declared roles; the witness belongs in the demos/9-nihongo s-series beside s54/s55.
