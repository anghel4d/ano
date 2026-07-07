# NEXT — make Japanese nouns first-class, end to end

You are picking up verified, in-progress work on branch `feature-jp-lexer`.

The architecture you must land on is specified in `src/compiler.md`, written in end-state present tense with a status line at the top. Read it before touching code. When you finish, verify every sentence of it against the implementation and flip its status line. If the implementation forces a divergence from it, alter the fine details to make it work. The Hierarchy is : The Mathematics > The Semantics > The Grammar > The Syntax > keywords, pipelining, implementation details.

## Situation

The repo is ano, an ECS query language compiled by `anoc` (src/, C, ano → BQN) and differential-tested against BQN post-states (`src/check-ano.sh`, needs `bqn` on PATH — cbqn is installed via nix profile; `make` is NOT on PATH, build with `cc -std=c23 -O2 -Wall -Wextra -c lex.c parse.c registry.c emit.c fs.c main.c && cc -std=c23 -O2 -o anoc *.o -lm` in src/).

The language has two surfaces: ASCII (as in, the keywords must be ASCII, the source files themselves take utf-8 as valid) ano, and the first-class Japanese-derived あの. (`--! ja`), design in `ano_nihongo.md`, mechanics in `src/GRAMMAR.md`. One abstract token stream, one parser, one emitter; `lex_ja` (src/lex.c) maps space-separated Japanese words to the same TokKinds, then re-roots each postfix operator before its operand span (`grab_primary`/`operand_start`). This architecture is sound and verified: all 211 demos pass; 100 of 104 nihongo/ASCII twin pairs emit byte-identical BQN, and the other 4 differ only in assertion lines the nihongo authors dropped (the transformations are identical and the missing assertions pass when grafted). The branch introduces zero ASCII drift: master-built anoc and branch-built anoc emit byte-identical BQN on all 107 ASCII demos. Keep the re-rooter as is — its one restriction (a postfix operand wider than one primary must be author-parenthesized, hard error otherwise) is a documented design decision, the measured boundary of the lexer-table claim in ano_nihongo.md.

What is broken is the noun layer of the current implementation. This paragraph is a defect report, not a design statement: the design holds ano and あの as first-class peers over one semantic substrate, and the code as it stands betrays that for nouns — the present `lex_ja` resolves Japanese nouns only through per-registry alias lines (`ja 北 Nord` in .reg files, loaded by registry.c into jaFrom/jaTo, consulted by `reg_ja`). Your job is to remove this asymmetry entirely. Today a registry written natively in Japanese (`col 金 num …`) fails three separate ways, all reproduced experimentally:

1. `lex_ja` resolves nouns only via the alias table, never via registry entry names — `unknown word '北'`.
2. `lex_ascii` rejects any non-ASCII identifier (`nstart`/`nchar` are `[A-Za-z0-9_]`) — `unknown character U+5317`.
3. Even smuggled past both with self-aliases, `emit.c` uses registry names verbatim as BQN identifiers and CBQN dies: `Error: Unknown characters: 北両手金…`.

Measured costs of the alias-only design: 829 `ja` lines across 91 registries, only 189 distinct; all 91 registries repeat `ja 前 prev`, `ja 行 row`, `ja 番号 index`, `ja 字 char` even though prev/row/index/char are system nouns living in parse.c/emit.c, not registry entries; alias targets have case-drifted (`ja 北 Nord` ×5 vs `ja 北 nord` ×13), silently reconciled by `reg_find`'s first-letter case-insensitivity. Mixing is one-directional: English nouns and keywords work inside `--! ja` (bare-ASCII fallthrough in lex_ja), but kanji fail on the ASCII surface, and ASCII operators `& > < >= , += -=` are not words in JA mode (only `+ - * / % = | ( ) [ ] ; <- |> ' _ ↕` crossed over — this is fine, leave it).

The goal: native UTF-8 nouns everywhere, `ja` demoted to an optional bilingual bridge, and the strong equivalence check made permanent. The author's intent: Japanese nouns and trait names are just natively registerable and treated the same as anything else. Useful reference for strict UTF-8 handling exists at `~/workspace/anopticengine`; note `ucp()` in lex.c is already the strict anoptic decoder and `ano_lex` already validates the whole source once at the boundary, so you need identifier policy, not decoding.

## Steps

### 1. UTF-8 identifiers in lex_ascii

Extend the identifier class: a name may start with and contain any codepoint ≥ U+0080 except a small blacklist, alongside the existing ASCII rules. Blacklist (these stay operators/whitespace/numeral machinery, never identifier chars): U+2195 ↕, U+3000 ideographic space, U+3001 、, U+30FB ・. Maximal munch must stop at ASCII operator bytes so unspaced `北&両手>60` lexes as NAME AMP NAME GT NUM. Kanji numerals do NOT become numbers on the ASCII surface — 六十 lexes as an identifier there; numerals stay per-surface. Decode with `ucp` (source is prevalidated; a decode here still returns length, use it to advance). Keep the existing comment conventions when you touch the char-class functions.

### 2. Nouns resolve by name; `ja` becomes a pure alias applied at resolution, not in the lexer

Design decision (author-confirmed): `ja` is nothing but a name alias — a (surface word → entry name) pair — and must behave like one. Two consequences. First, the closed grammar outranks ALL nouns: in `lex_ja`, word resolution order becomes sigils, jatab, numeral, then nouns, then error — an alias or a native column named と or 千 is shadowed by the particle/numeral by design (the vocabulary-is-closed argument in ano_nihongo.md's segmentation section). Second, move the alias application out of `lex_ja` and into name resolution: both lexers emit T_NAME carrying the surface spelling (in `lex_ja`, any word that survives jatab/numeral and is a legal identifier — ASCII or UTF-8 — becomes T_NAME or a keyword via `kwkind`; unknown words no longer error at lex time, they error at resolution like ASCII names do), and the resolver (`reg_find` or a wrapper used by parse/emit) tries exact entry names first, then the `ja` table (exact bytes, no first-letter case-folding for the alias hop). This makes aliases surface-agnostic — `ja 金 gold` lets an ASCII script say 金 too — and removes the registry dependence from `lex_ja`; if that leaves the registry parameter unused in `lex_ja`/`ano_lex`, delete the plumbing. Knock-on: the one `--! same-tokens` demo (ex01) compares token streams by name payload; canonicalize names through the resolver before comparing (or compare kinds plus resolved entries), and note `--tokens` now prints surface spellings. Also hoist the four system nouns into jatab as global entries emitting T_NAME with a payload name, exactly like the fold entries use `nm`: `{"前", T_NAME, 0, "prev"}, {"行", T_NAME, 0, "row"}, {"番号", T_NAME, 0, "index"}, {"字", T_NAME, 0, "char"}`.

### 3. BQN name mangling in the emitter

Add one helper (emit.c) that maps a registry entry name to a BQN-legal identifier: names already matching `[A-Za-z][A-Za-z0-9_]*` pass through as today (via the existing `lc` behavior — do not change the emitted BQN for any existing ASCII demo, that is a hard invariant), anything else gets a stable deterministic ASCII name derived from the entry's index in the registry (e.g. `jp0`, `jp7`; any scheme works if it is deterministic, collision-free with real column names and emitter temporaries, and stable within a compile). Route EVERY registry-name-to-BQN-identifier emission through it: grep `e->name` and `->name` across emit.c AND main.c — the `lc(em, e->name)` sites, `pres_%s`, `fnv` for fn names, srel/bind/rel emission, and critically the assertion emitter for `--! expect` (search for the `∧´1e¯9≥` format string; `--! expect 金 = …` must assert against the mangled var). Emit the human name as a BQN comment at the column's definition line for debuggability. Mind `ANO_NAMESZ` is 256 — kanji names fit, no truncation logic needed.

### 4. Registry hygiene across demos/registries/

Registry policy (author-confirmed): existing registries keep running exactly as they do today, and `ja` remains a .reg keyword — the alias mechanism, retained as-is for now. Renaming the keyword later is trivial but out of scope; if you are ever tempted, note `alias` is already a different .reg entry kind (RK_ALIAS, target-alias entities) — do not collide with it. The only edits to existing regs: delete the four system-noun alias lines (`ja 前 prev`, `ja 行 row`, `ja 番号 index`, `ja 字 char`) from every .reg — step 2 made them global (≈364 lines gone) — and normalize the alias-target case drift: rewrite each remaining `ja X Y` so Y is the exact byte spelling of the registry entry it names (e.g. `ja 北 Nord` → `ja 北 nord` where the col is `nord`). Script both; do not hand-edit 91 files.

### 5. Restore the four thinned assertion sets

`11-noita/n1-cast-nihongo.ano`, `11-noita/n5-alchemy-nihongo.ano`, `4-order/31-spatial-topk-nihongo.ano`, `5-generate/23-expand-alias-nihongo.ano` each pin fewer columns than their ASCII twins. Mirror the twin's `--! expect`/`--! out` directive block exactly, same lines, same order (order matters for byte-identical emission in step 6). The grafted assertions are already proven to pass.

### 6. Make the strong check permanent

Add a second pass to `src/check-ano.sh`: for every `X-nihongo.ano` with twin `X.ano`, `anoc --emit` on both must be byte-identical; print `ok-emit`/`FAIL-emit` per pair and fail the script on any mismatch. After steps 4–5 this must hold for all 104 pairs (it held for 100 before, and the 4 were only missing assertions). This is the enforcement of two surfaces → one BQN program, corpus-wide, every run.

### 7. New witness demos — the point of the whole exercise

Follow the conventions in demos/demos.md and the per-directory READMEs, supplementary `s`-prefix naming (like `s05-set-hop`), registry in demos/registries/, `--! expect` pins, README line added. Three witnesses in `demos/9-nihongo/`:

- s50-native-registry: a registry whose columns are natively Japanese (`col 金 num …`, `col 北 bool …`, `col 両手 num …`, NO `ja` lines), a `-nihongo.ano` using the nouns directly (`北 と 両手 六十 より 、 金 に 千 たす`), and an ASCII `.ano` twin using the same kanji nouns from the ASCII surface (`北 & 両手 > 60 , 金 += 1000`). Same pins, and the pair must pass the step-6 emit-identity check.
- s51-mixed-surface: English nouns under Japanese grammar (`Nord と TwoHanded 六十 より 、 Gold に 千 たす` against an existing English registry — this already works today, pin it so it never regresses) plus a mixed line with kanji and Latin nouns in one statement.
- s52-bilingual-bridge: one registry with English column names plus `ja` aliases, scripted from both surfaces — the alias mechanism's one legitimate job, kept as a first-class witness.
- s53-native-world-kinds: a family of new registries, natively Japanese throughout, that collectively exercise EVERY registry entry kind under a Japanese name — col in each type (num, bool, sym, vec, char), pres and default, rel with its inv, srel, bind in each kind, fn (Japanese fn name, verbatim BQN body), and a lattice with fields — driven by demo pairs that touch each one. This is not decoration: it is the coverage witness for step 3, since the mangler must hold at every emission site (`pres_` masks, fn names, srel fibers, binds, the `--! expect` assertion emitter), and a three-column toy registry witnesses almost none of them. Split across as many small registries and demo pairs as reads well in the corpus; each pair passes the step-6 emit-identity check.

### 8. Docs, in the right registers

`src/compiler.md` is the blueprint of record — do not redraw it; verify it and flip its status line (see above). `src/GRAMMAR.md` "Japanese skin": update for the new resolution order, native entry names, the global system nouns, and the emitter mangling; state plainly that `ja` is an optional bilingual alias, not the noun mechanism. `src/src.md` lex.c/emit.c bullets: same, one line each. `ano_nihongo.md`: this is the author's voice — tighten nothing, rewrite nothing; append only a short flat subsection (no decorative bolding) recording two settled facts: the spaced skin's one-primary parenthesization rule as the measured boundary of the lexer-table claim, and that registry nouns are natively UTF-8 with `ja` as an optional cross-surface alias. If anything feels like it closes an open question (e.g. negation glyph, numeral policy on the ASCII surface), do NOT close it — add the tradeoff under the doc's open questions instead.

## Verification, in order

1. Rebuild anoc (cc line above). Zero warnings expected.
2. `cd src && ./check-ano.sh` — every demo ok, including the new s50–s52 and the emit-identity pass.
3. Sanity check: existing ASCII demos pass every assertion on the same data unchanged exactly as they do today. Before your first edit, snapshot `for f in $(find demos -name '*.ano' ! -name '*-nihongo.ano'); do ./src/anoc --emit $f > /tmp/base/$(echo $f | tr / _).bqn; done` and diff after. Any diff on a pre-existing ASCII demo is a bug in your step 3 passthrough.
4. Negative tests by hand: a column named と must not shadow the particle (particle wins); `金 に 千 + 三 たす` still errors (unparenthesized multi-primary operand); an unknown kanji word still errors by word, an unknown ASCII char still errors by codepoint.
5. Optional, author doctrine (flat whole-program builds, LTO welcome): add `-flto` to the Makefile CFLAGS while you are in there. Never add caching or incremental machinery of any kind.
6. Show the author the full diff and STOP. No commit, no push. Suggest archiving this NEXT.md in the same commit.
