# anoc — Patch Notes

## Snapshot 26w28a — "That One Over There, Part II"

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
- Nothing was committed or pushed. Grab the snapshot by checking out `feature-jp-lexer`; back up your world first, because this one does not commit itself.

## Verification Report — 26w28a audited against the working diff

Audited 2026-07-07 against the uncommitted working tree on `feature-jp-lexer` (baseline commit 9b0edd6). Method: every claim above was checked against the actual `git diff`, a fresh `-Wall -Wextra` rebuild, the full harness, a pre-patch binary built from HEAD in a throwaway worktree, and hand-run negative fixtures. Verdict: the snapshot is accurate — every checkable claim holds, the diff contains nothing these notes do not mention, and the audit surfaced one corner-case behavior change the notes miss (finding #1 below).

### Verified

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
- T: Decide whether a `ja id <col>` alias should still steer `idCol` (restore the hop) or whether `role id <col>` is now the only sanctioned spelling (bless the divergence in one doc line).
- A: Verified the divergence empirically: a world with `ja id 識別`, no literal id/keys column, no role line, and a key column in relation position emits `{/bind=𝕩}¨jp0` pre-patch and `{/bind=𝕩}¨(↕anoN)` post-patch. Grepped the corpus: no registry uses `ja id` or `ja keys`, and the 225-demo byte-identity stands, so nothing live is affected. Not fixed — a design call, surfaced per doctrine, carried into NEXT-2.md.
- R: Either `reg_role`'s fallback gains the ja hop (one loop, mirroring registry.c:375-380) or compiler.md/GRAMMAR.md states that roles route by declaration or literal name, never through ja aliases.
- (a) Issue IS: src/registry.c:387-399 (`reg_role`, fallback loop without the ja hop `reg_find` has at src/registry.c:375-380); the inexact sentence is this file's ANO-101 line.
- (b) Affects: src/emit.c:157-158 (`idCol`), reached from the set-hop sites src/emit.c:359 (`fiberVar` over a key column) and src/emit.c:942 (`AnoImage` membership). Repro registry: `n 4` / `col 識別 num 3 4 5 6` / `col bind num 3 3 5 5` / `col mark bool 0 1 0 1` / `col damage num 0 0 0 0` / `ja id 識別`; program: `mark , damage += (#/ (bind' & mark))`.

#2 — role proto and role id have no witnessing demo
- S: registry.c accepts five role names and emit.c routes all five; s54 declares and load-bearingly witnesses three (pos, keys, parent). proto and id are routed but never exercised through a `role` declaration anywhere in the corpus.
- T: Witness the remaining two: a spawn-from-proto into a kanji-named proto column, and a set-hop membership over a kanji-named id column (which also covers #1's corner from the sanctioned side).
- A: Confirmed by grep: `role proto`/`role id` appear in no `.reg`; the proto route and the idCol id-probe execute only their literal-name fallbacks in every demo. Not added here — new demos are new scope; recorded for NEXT-2.md.
- R: An s56 pair whose proto and id roles are each proven load-bearing the way s54's are: delete the role line, a pin fails.
- (a) Issue IS: coverage gap — src/registry.c:346 admits five names; demos/registries/s54-native-roles.reg:13-15 declares only pos/keys/parent; no other `.reg` carries a role line.
- (b) Affects: src/emit.c:1356 (proto routing) and src/emit.c:156-161 (`idCol`) — both untested under declared roles; the witness belongs in the demos/9-nihongo s-series beside s54/s55.
