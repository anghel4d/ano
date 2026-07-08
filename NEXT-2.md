# NEXT-2 — The Native World, Closed Out.

You are picking up verified, in-progress work on branch `feature-jp-lexer`.

The architecture you must land on is specified in `src/compiler.md`, written in end-state present tense with a status line at the top. Read it before touching code. Its status line reads implemented and every sentence of it has been verified against the code; your job is to keep that true. When you finish, verify every sentence of it against the implementation again, and if the implementation forces a divergence from it, alter the fine details to make it work. The Hierarchy is : The Mathematics > The Semantics > The Grammar > The Syntax > keywords, pipelining, implementation details.

## Situation

The repo is ano, an ECS query language compiled by `anoc` (src/, C, ano → BQN) and differential-tested against BQN post-states (`src/check-ano.sh`, needs `bqn` on PATH — cbqn is installed via nix profile; `make` is NOT on PATH, build with `cc -std=c23 -O2 -Wall -Wextra -c lex.c parse.c registry.c emit.c fs.c main.c && cc -std=c23 -O2 -o anoc *.o -lm` in src/).

The language has two surfaces: ASCII (as in, the keywords must be ASCII, the source files themselves take utf-8 as valid) ano, and the first-class Japanese-derived あの (`--! ja`), design in `ano_nihongo.md`, mechanics in `src/GRAMMAR.md`. One abstract token stream, one parser, one emitter. The 26w28a snapshot (PATCHES.md, audited — its appended verification report re-proves every claim) made native nouns feature-complete: registries take `role <name> <col>` lines routing the five system roles (keys, id, parent, proto, pos) to natively named columns through `reg_role` (src/registry.c), the `^alias`/`:Sym` sigils accept UTF-8 names on both surfaces, and `--! expect` pins char columns as exact glyph runs. The state is verified: all 229 demos pass `--run`, 113 conjugate pairs emit byte-identical BQN, and the working diff introduces zero drift — HEAD-built anoc and working-tree anoc emit byte-identical BQN on all 225 pre-existing demos. Everything is uncommitted on `feature-jp-lexer`; the author commits, you never do. Keep the re-rooter as is — its one restriction (a postfix operand wider than one primary must be author-parenthesized, hard error otherwise) is a documented design decision, the measured boundary of the lexer-table claim in ano_nihongo.md.

Three items remain, in order:

1. Witness the last two roles. `role proto` and `role id` are routed (src/emit.c:1356, src/emit.c:156-161) but no demo declares them (PATCHES.md verification report, finding #2). Add an s56 pair under demos/9-nihongo/: a spawn-from-proto into a kanji-named proto column, and a set-hop membership (`bind'`-style, per w3-c) over a kanji-named id column, each role proven load-bearing the way s54's are — delete the role line and a pin must fail.
2. Surface the reg_role ja-hop tradeoff to the author; do not resolve it silently. Pre-patch `idCol` resolved through `reg_find`, whose fallback ends in the ja-alias hop; `reg_role`'s fallback (src/registry.c:387) stops at the entry table, so a world reaching its stable-id column only via `ja id <col>` now fibers over the row iota — a verified divergence with zero corpus impact (PATCHES.md verification report, finding #1, repro included). The two resolutions: restore the hop in reg_role's fallback (one loop, mirroring registry.c:375-380), or bless the divergence with one doc line in compiler.md and GRAMMAR.md stating roles route by declaration or literal name, never through ja aliases. Prepare both diffs, present the tradeoff, let the author pick.
3. The unspaced reader is the declared next tier, and it is out of scope here. Its one binding constraint is already pinned (ano_nihongo.md, the post-branch note): it may segment by maximal munch, but it resolves the munched spans against the registry after lexing, never during — the dictionary stays a resolution-time oracle and the two skins keep one parser. Do not start it without the author's explicit go; it is named here so the constraint is not lost.

### 8. Docs

Do not violate the content of the following .md files: ano-language.md, ano_nihongo.md, ano-sky.md.

The demos are currently all considered STABLE, s54/s55 included, and should not be altered.

Add new demos and wire them up.
For every demo, write:
- Code Sources
- What the data is meant to represent
- What "Layer" (1, 2, 3) the data approximately falls into, or if it defies categorization in that layering idea.
- What the scenario could be in gameplay terms (Skyrim analogies work. Also: Factorio, Noita, Starcraft II, Starcraft II Cortex Roleplay).
- A .ano and a .bqn file. For new demos, a -nihongo.ano is optional. (The s-series precedent: s50–s55 ship as .ano + -nihongo.ano conjugate pairs with the writeup compressed into the header comment; an s56 should follow its own series.)

## Verification, in order

1. Rebuild anoc (cc line above). Zero warnings expected.
2. `cd src && ./check-ano.sh` — every demo ok, including the new s56 pair and the emit-identity pass (229 ok / 113 ok-emit before your work; your additions only raise the counts).
3. Sanity check: existing demos pass every assertion on the same data unchanged exactly as they do today. Before your first edit, snapshot `for f in $(find demos -name '*.ano'); do ./src/anoc --emit $f > /tmp/base/$(echo $f | tr / _).bqn; done` and diff after. Any diff on a pre-existing demo is a bug in your change.
4. Negative tests by hand: `role foo <col>` still rejected by name, `role pos <non-column>` still rejected at load, duplicate `role` lines still first-wins; deleting any role line from s54 or s56 must fail a pin; a column named と must not shadow the particle (rejected at load); `金 に 千 + 三 たす` still errors (unparenthesized multi-primary operand); an unknown kanji word still errors by word, an unknown ASCII char still errors by codepoint.
5. Optional, author doctrine (flat whole-program builds, LTO welcome): `-flto` is already in the Makefile CFLAGS; leave it. Never add caching or incremental machinery of any kind.
6. Show the author the full diff and STOP. No commit, no push. Suggest archiving NEXT.md, NEXT-2.md, and PATCHES.md in the same commit.
