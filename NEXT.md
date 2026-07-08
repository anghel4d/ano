# NEXT — The Purity of the Language. 

You are picking up verified, in-progress work on branch `feature-jp-lexer`.

The architecture you must land on is specified in `src/compiler.md`, written in end-state present tense with a status line at the top. Read it before touching code. When you finish, verify every sentence of it against the implementation and flip its status line. If the implementation forces a divergence from it, alter the fine details to make it work. The Hierarchy is : The Mathematics > The Semantics > The Grammar > The Syntax > keywords, pipelining, implementation details.

## Situation

The repo is ano, an ECS query language compiled by `anoc` (src/, C, ano → BQN) and differential-tested against BQN post-states (`src/check-ano.sh`, needs `bqn` on PATH — cbqn is installed via nix profile; `make` is NOT on PATH, build with `cc -std=c23 -O2 -Wall -Wextra -c lex.c parse.c registry.c emit.c fs.c main.c && cc -std=c23 -O2 -o anoc *.o -lm` in src/).

The language has two surfaces: ASCII (as in, the keywords must be ASCII, the source files themselves take utf-8 as valid) ano, and the first-class Japanese-derived あの. (`--! ja`), design in `ano_nihongo.md`, mechanics in `src/GRAMMAR.md`. One abstract token stream, one parser, one emitter; `lex_ja` (src/lex.c) maps space-separated Japanese words to the same TokKinds, then re-roots each postfix operator before its operand span (`grab_primary`/`operand_start`). This architecture is sound and verified: all 211 demos pass; 100 of 104 nihongo/ASCII twin pairs emit byte-identical BQN, and the other 4 differ only in assertion lines the nihongo authors dropped (the transformations are identical and the missing assertions pass when grafted). The branch introduces zero ASCII drift: master-built anoc and branch-built anoc emit byte-identical BQN on all 107 ASCII demos. Keep the re-rooter as is — its one restriction (a postfix operand wider than one primary must be author-parenthesized, hard error otherwise) is a documented design decision, the measured boundary of the lexer-table claim in ano_nihongo.md.

### 8. Docs
Do not violate the content of the following .md files: ano-language.md, ano-nihongo.md, ano-sky.md. 

The demos are currently all considered STABLE, and should not be altered. 

Add new demos and wire them up.
For every demo, write:
- Code Sources
- What the data is meant to represent
- What "Layer" (1, 2, 3) the data approximately falls into, or if it defies categorization in that layering idea.
- What the scenario could be in gameplay terms (Skyrim analogies work. Also: Factorio, Noita, Starcraft II, Starcraft II Cortex Roleplay).
- A .ano and a .bqn file. For new demos, a -nihongo.ano is optional.

## Verification, in order

1. Rebuild anoc (cc line above). Zero warnings expected.
2. `cd src && ./check-ano.sh` — every demo ok, including the new s50–s52 and the emit-identity pass.
3. Sanity check: existing ASCII demos pass every assertion on the same data unchanged exactly as they do today. Before your first edit, snapshot `for f in $(find demos -name '*.ano' ! -name '*-nihongo.ano'); do ./src/anoc --emit $f > /tmp/base/$(echo $f | tr / _).bqn; done` and diff after. Any diff on a pre-existing ASCII demo is a bug in your step 3 passthrough.
4. Negative tests by hand: a column named と must not shadow the particle (particle wins); `金 に 千 + 三 たす` still errors (unparenthesized multi-primary operand); an unknown kanji word still errors by word, an unknown ASCII char still errors by codepoint.
5. Optional, author doctrine (flat whole-program builds, LTO welcome): add `-flto` to the Makefile CFLAGS while you are in there. Never add caching or incremental machinery of any kind.
6. Show the author the full diff and STOP. No commit, no push. Suggest archiving this NEXT.md in the same commit.
