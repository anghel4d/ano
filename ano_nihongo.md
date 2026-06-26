# ano 日本語 — a Japanese surface mode

ano's canonical surface is ASCII. This document defines a second concrete surface where Japanese grammatical particles stand in for the operators: a tokenizer skin over the same AST.

## Premise

ano's core form is head-final: `source & predicate , effect`. Selection on the left, effect on the right, the comma between. Japanese is also head-final and agglutinative, and its particles are postfix markers that attach to a noun and name its role. A postfix role-marker is a tacit postfix operator. So the surface ano already wants and the grammar Japanese already has are the same shape, and the mapping is discovery.

## Architecture: a reader skin

One AST, many readers. The grammar is defined over abstract token kinds (`AND`, `OR`, `HOP`, `GT`, `COMMA`, `ASSIGN_ADD`, `IDENT`, `NUM`). Each surface is a lexer that maps concrete glyphs to those kinds. と and `&` emit the same `AND`; everything downstream is identical. The Japanese mode is a lexer table plus a numeral reader. The parser, the type/footprint resolution, the bytecode, the JIT, none of them learn which surface produced the tokens.

Make it bidirectional: one AST, N reader/printer pairs. Then ASCII and Japanese round-trip through the same tree, which is useful for teaching and docs, and the mode falls out of the printer for free.

The mode is also a litmus test. If adding the Japanese reader is trivial, the lexer, parser, and semantics are cleanly separated. If it is not trivial, surface has leaked into semantics and the leak is a bug worth finding. The meme doubles as an architecture check, which is the real reason to build it early.

## Particle → operator map

| ano | particle | gloss | note |
|---|---|---|---|
| `&` and | と | "and", the listing particle | |
| `\|` or | か | "or" | か is also the question particle |
| `.` the dotted hop, `rel.Comp` | の | genitive "'s" | the relationship hop is literally the possessive |
| `@` region or scope | で | locative "at / in" | で marks where the action happens |
| `,` selection-effect separator | 、 | the comma (読点) | exact one-to-one |
| `>` | より, or 超 | "than", "over" | より alone needs a direction word, 超 disambiguates |
| `<` | 未満 | "under" | the sign-board word, 18歳未満 |
| `==` | 同 | "same" | |
| `+=` | に … たす | "to X, add" | に marks the target, たす is add |
| `-=` | に … ひく | subtract | |
| `*=` | に … かける | multiply | |
| `/=` | に … わる | divide | |
| `=` set | を … にする | "make X be" | |
| `!` not | ず, ない | the negation auxiliary | postfix, and it bites, see below |

The standout is の. ano's relationship hop, `mentor.TwoHanded`, reads `師匠 の 両手`, mentor's two-handed. The genitive particle is the foreign-key join with no adaptation at all. で for `@` is as clean: a scope is a place, and で is the particle for the place an action occurs.

## Worked example

The canonical task, +1000 gold to every Nord with Two-Handed over 60.

ASCII ano:
```text
Nord & TwoHanded > 60 , Gold += 1000
```

Japanese, kanji:
```text
北 と 両手 六十 より 、 金 に 千 たす
```

北 Nord, と and, 両手 Two-Handed, 六十 より over sixty, 、 the comma, 金 に 千 たす is Gold += 1000. The token order is the ano token order with the operators moved to their postfix positions, which is where Japanese already puts them.

The dotted hop, `Nord & mentor.TwoHanded > 80 , Gold += 1000`:
```text
北 と 師匠 の 両手 八十 より 、 金 に 千 たす
```

A scoped selection, `Merchant @ cell(Whiterun) , Gold += 5000`:
```text
商人 ホワイトラン で 、 金 に 五千 たす
```

Loanword entities take katakana (ノルド, ホワイトラン), native concepts take kanji. The choice is cosmetic, both lex to `IDENT`.

## Numerals

Kanji numerals are positional-by-name: 六十 is 60, 千二百 is 1200, 五千 is 5000. A small numeral grammar over 一..九 and 十百千万 reads them to integers. Arabic digits can be allowed alongside; they lex to the same `NUM`. The numeral reader is the one piece of the Japanese mode that is more than a table.

## The unspaced problem

APL and BQN are unspaced because every token is a single glyph, so lexing is maximal-munch over single characters. Japanese identifiers are multi-kana, so an unspaced stream like `のるどとりょうてろくじゅうより、きんにせんたす` needs segmentation, and the particles (に と より は を) are short and collide with substrings of names.

General Japanese segmentation is hard because the vocabulary is open. ano's vocabulary is closed. Component names are declared in the registry before evaluation, so at lex time the vocabulary is fixed and known: registry names, the particle and verb set, the numerals. Segmentation is maximal-munch against that closed dictionary, the registry being the dictionary, an ordinary lexer problem. Remaining ambiguity, a component name that ends in a particle homograph, resolves by longest match plus grammar position, since the parser only accepts a particle where an operator is legal. Spaced input needs none of this and is a pure table; ship it first and treat the unspaced reader as the harder tier.

## Negation, the one that fights back

Japanese negation is a postfix auxiliary, which suits a postfix language, but `死 ない` for not-dead sits next to 死ぬ, which is the verb to die, and the kana run together in unspaced mode. Options are the classical ず, the modern ない, or the kanji prefix 非 which is unambiguous but breaks the postfix rhythm. Unresolved.

## Open questions

- Direction of comparison. より needs a paired adjective in natural Japanese; 超 and 未満 disambiguate but read as jargon. Pick one scheme and commit.
- Negation glyph, per above.
- Reduction and scan. The ASCII surface uses `/` and `\`; the particle forms for fold and scan are not yet chosen.
- IME ergonomics. Writing kana code needs an input method, and the unspaced form is hard to type as well as to lex. The spaced form is the authoring surface; the unspaced form is accepted but not authored.
- Whether the printer normalizes mixed kana, or preserves the author's katakana-vs-kanji choices on round-trip.

## Status

A side-project, orthogonal to the implementation language. Even on a q or k prototype the Japanese reader is a preprocessor that emits ano AST, so it can be built at any time and costs nothing later. Keep it a mode. The ASCII surface stays the product; this is the teaching and marketing skin, and it is good at that because あの is already Japanese and the canonical task is, after all, paying Norsemen in their own grammar.
