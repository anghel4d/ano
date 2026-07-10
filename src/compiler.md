# compiler.md — anoc, the architecture of record

Status: implemented.

anoc compiles ano — both surfaces — to a BQN program evaluated against a registry-loaded world, and asserts the resulting post-state against pins written in the demo itself. One binary, six C files, one `cc` invocation, whole-program, every time. There is no build cache, no incremental mode, and there never will be. BQN is the reference back-end and the semantic oracle; the fixed future target is the bytecode VM and JIT, and nothing in this file changes when that lands except the box after `emit`.

## The machine

```
 demo.ano ────────── main.c: read, strip --! directives ────────── demo-nihongo.ano
    │                (--! registry <path>, --! ja, --! expect/out)        │
    │                                 │                                   │
    │                                 ▼                                   │
    │                      demos/registries/X.reg                         │
    │                      reg_load (registry.c)                          │
    │                      world: n · col · rel · srel · inv              │
    │                      bind · fn · field · lattice                    │
    │                      ja aliases (surface word → entry name)         │
    │                                 │                                   │
    ▼                                 │                                   ▼
 lex_ascii (lex.c)                    │                        lex_ja (lex.c)
   maximal munch                      │                          word-split on spaces
   UTF-8 identifiers                  │                          grammar first: particle,
   fused folds +/ max\                │                          keyword, fold/scan table,
   ^alias · :sym · 3mo                │                          kanji numerals; then any
    │                                 │                          legal identifier, UTF-8 or
    │                                 │                          ASCII, becomes NAME; then
    │                                 │                          re-root postfix operators,
    │                                 │                          delete the に marker
    │                                 │                                   │
    ▼                                 │                                   ▼
 ═══════════ ONE TokKind stream — columnar, interned, head-initial ═══════════
                 │                                  ▲ convergence point; --tokens taps here
                 ▼
           parse.c ──► ONE AST — one grammar, surface-blind
                 │
                 ▼
           resolve + emit (emit.c) ◄── registry: entry names, then aliases, one case fold;
                 │                      non-ASCII names mangled to stable BQN identifiers
                 ▼
           BQN program text — --emit taps here
                 │  + rt.bqn prepended (the runtime, main.c)
                 │  + --! expect/out compiled to ! assertions (emit.c)
                 ▼
           cbqn — exit 0 iff every pinned post-state holds
                 │
                 ▼
           check-ano.sh — every demo against its pins; every conjugate pair
           byte-identical under --emit; the .bqn twins are the oracle
```

## The parts

main.c — the driver. Reads the demo, strips `--!` directives (registry path, the ja flag, the expected post-state), orchestrates lex → parse → emit — the emitter appends the pins as BQN assertions — and runs the result under cbqn via a temp file. `--tokens` and `--emit` dump the two intermediate representations; there are only two. `--dump <path>` writes the loaded world back out as .reg text through a staged file and rename(2) — the commit loop's write half, crash-safe by construction; dump → load → dump fixpoints byte-identically. `--save <path>` (requires `--run`) closes the loop on post-state: the child's stdout runs through a pipe, 0x1E-sentinel lines the flag-gated serializer printed after the pins patch the loaded Registry to post-state (n may grow on spawn; arrays allocate fresh), everything else forwards verbatim, and the world commits to `<path>` through the same staged rename — on a nonzero exit nothing is written. This is the pipe-back channel kore's REPL tick runs on.

registry.c — the world loader and dumper. A `.reg` file is the extensional database: entity count, columns, relationships and their inverses, binds, verbatim BQN fn bodies, lattice fields, derived tags. Entry names are any legal identifier, UTF-8 included; a Japanese world is registered exactly like an English one. `as`/`ja` lines are pure name aliases, (surface word → entry name) pairs filling one table and nothing more — they carry no data, they exist only so one world can be scripted idiomatically from both surfaces, and a registry that wants only one vocabulary needs none; the ja spelling documents the Japanese surface at the declaration site. The three-word `as <word> <col> <value>` declares a derived tag, the equality mask over the live column, recomputed at each use and read-only as an effect target. Loading is strict within name kinds: two entries or two alias sources whose names fold together are one name and rejected, as is any name the lexer owns under the fold. A `role <name> <col>` line points a system-column role (keys, id, parent, proto, pos) at a natively-named column, so the spawn machinery routes by declaration, not by the literal spellings pos/keys; with no `role` line the fallback is reg_find on the literal name — one resolver, entries then aliases — and every English world routes exactly as before. reg_dump is the loader's inverse: the in-memory world back to .reg text, staged then rename(2)-committed — the world is a column store, so a save is a registry dump.

lex.c — two skins, one alphabet. Source is validated as strict UTF-8 once at the `ano_lex` boundary; the skins then decode unchecked. `lex_ascii` is maximal munch: ASCII operators, fused folds and scans, sigils whose names follow the same identifier policy (so `:山賊` and `^世界` are legal), counters, and identifiers that admit any codepoint ≥ U+0080 outside a small operator blacklist, so an unspaced `北&両手>60` lexes as NAME AMP NAME GT NUM. `lex_ja` is the spaced Japanese skin: words resolve grammar-first — particles, keywords, folds, scans, kanji and Arabic numerals — and only then as identifiers, so the closed vocabulary can never be shadowed by a noun; whatever survives becomes NAME with its surface spelling. The skin's one transformation beyond the table is order: Japanese is head-final, so a normalization pass re-roots each postfix operator before its operand span and deletes the fused に target marker. An operand wider than one primary must be author-parenthesized; that rule is the measured boundary of the lexer-table claim in ano_nihongo.md, held deliberately, and it fails loud, never silent.

the token stream — the convergence point. Columnar (kind, name, num, line grown together), text interned, head-initial. Everything Japanese-specific dies here. Neither the parser nor anything after it can determine which surface produced the stream.

parse.c — one grammar, one AST. Names in the AST are surface spellings; meaning has not attached yet. There is no second AST and there must never be one: two ASTs is two semantics, and the whole point of the conjugate corpus is proving there is one.

resolve + emit (emit.c) — where names meet the world. The case contract in one line: registry names are case-insensitive (ASCII fold, non-ASCII bytes exact), values are case-sensitive — always, no exceptions. Resolution tries registry entry names, then the alias table, in that order, identically for both surfaces, every compare through the one comparator; defs are program variables — exact-byte, consulted before the registry in value and mask position as always, so a def and an entry whose names differ only by case are two names — and an alias is outranked by grammar during lexing and by entry names during resolution, which is all an alias should ever be. Emission stages the program over rt.bqn; columns become BQN arrays, fn bodies paste verbatim, and any entry name that is not a legal BQN identifier is mangled to a stable deterministic ASCII name, the human spelling kept in a comment. The emitted program is the semantic object: two conjugate demos emit byte-identical BQN or the build fails.

the harness — check-ano.sh. Two passes, both mandatory: every `.ano` runs under `anoc --run` and its pinned post-state must hold; every `X-nihongo.ano` must emit byte-for-byte what its twin `X.ano` emits. The `.bqn` twins beside the demos are the independent oracle the C implementation is differential-tested against. Coverage is the proof of the language's central claim — two surfaces, one meaning — so a conjugate that pins fewer columns than its twin is a bug in the corpus, not a style choice.

## Invariants

- One AST. Surface variation lives in lexers and dies at the token stream.
- Grammar outranks nouns: no registry name, native or aliased, can shadow a particle, keyword, or numeral — under the case fold included.
- Registry names fold, values never: one comparator (names_eq) for every registry name resolution; sym values, char glyphs, fn bodies, and program-level names (defs, binders) compare exact bytes.
- `as`/`ja` is a pure alias — resolution-time, surface-agnostic, invisible to the AST and the emitted program.
- Conjugate demos emit byte-identical BQN; the harness enforces it on every run.
- Strict UTF-8 exactly once, at the boundary; identifiers are open to any script, the operator set is closed.
- The re-root rule: one primary per postfix operand, parens beyond, hard error otherwise.
- Flat compilation, whole-program, LTO welcome, arenas and columns for all data, no incremental builds ever, nix for all tooling.

## Lineage

Typed translation over phase ceremony, per TAPL; notation as a tool of thought, per Iverson; columnar minimalism, per Whitney; linear discipline in every buffer, per Lafont; totality from finite combinators, per Church. The Dragon Book was consulted so it could be declined.
