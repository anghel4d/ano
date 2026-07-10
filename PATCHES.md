# anoc — Patch Notes

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
