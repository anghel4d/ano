考案: Anghel (Anghel4d)

# ano 日本語


ano's canonical surface is ASCII. This document defines a second concrete surface where Japanese grammatical particles inform the overall structure of the language at a semantic and syntactic level. This is because I think Japanese is pleasant, simple (grammatically), and has an exceptional internal consistency for an extant spoken language.

The 日本語ーmode is also a litmus test for ano's semantics. If adding the Japanese reader is trivial, the lexer, parser, and semantics are cleanly separated. Note to self: this project might be my opportunity to introduce my long-cherished coinage, the word "lexiom". I don't know what it means yet.

## Implemented reader

Steel implements a spaced Japanese particle surface and numeral reader in [lex.rs](../steel/src/lex.rs). It produces the same token kinds consumed by the ASCII parser. It does not parse arbitrary Japanese sentences.

Registry nouns may be UTF-8. Exact declaration names resolve before optional `ja` aliases. The lexer does not consult the registry to segment unspaced noun phrases; reserved grammar spellings cannot become registry nouns.

| ASCII | Japanese |
|---|---|
| `&`, `\|`, `.` | と, か, の |
| `@` | で |
| `,` | 、, が, は |
| `>`, `<`, `==` | より, 未満, 同 |
| `>=`, `<=`, `!=` | 以上, 以下, 不同 |
| `!` | ない |
| `+=`, `-=`, `*=`, `/=` | たす, ひく, かける, わる |
| `=` assignment | にする |
| `+`, `-`, `~` structural | 付, 除, 消 |
| `;`, `=>` | て, なる |

に marks a target. Postfix rewriting consumes one primary: an atom, matched group, hop chain, or shape run. Parenthesize a wider operand, as in `金 に ( 千 + 三 ) たす`.

## Folds and scans

総和, 総積, 総数, 最大, 最小, 平均, 皆, and 或 select sum, product, count, maximum, minimum, average, all, and any folds.

累和, 累積, 累数, 累大, 累小, 累平均, 累皆, and 累或 select the corresponding scans. Their carriers, empty results, and accumulation order are the same as the ASCII forms.

定義, 解除, 生成, 於, 至, 経由, 沿, 整列, 別, 取, 降順, 上位, 格付, 縮約, 走査, 交差, and 展開 map to the corresponding control and query tokens. [The keyword reference](ano-keywords.md) and lexer own the complete grammar.

## Examples and boundary

The paired programs in [demos/9-nihongo](../demos/9-nihongo) exercise reader equivalence. [TODO.md](../todo/TODO.md) identifies spatial examples that remain quarantined.

The author's [WIP tour](../tour.md) develops the intended language. Natural-sentence translation, general counters as units, and a verb-dependent case grammar are not implemented by the current reader.

## A paired sentence

Demo 084 declares Japanese aliases for Merchant, Whiterun, and Gold. Its Japanese program is:

```haskell
商人 ホワイトラン で 、 金 に 五千 たす
```

Its ASCII reading is:

```haskell
Merchant @ Whiterun , Gold += 5000
```

で attaches the scope, 、 separates selection from effect, に identifies the destination, and たす supplies the update. The numeral reader turns 五千 into 5000. The registry gives the nouns their meanings; the particles alone do not discover a merchant or a place.

Both readers feed the shared parser and emitter. Reader equivalence therefore checks that the two spellings produce the same program, while fixture expectations check what that program does to its declared world. Those are separate checks.

Japanese-inspired design remains useful beyond the currently admitted reader. A fluent natural sentence may still need syntax or vocabulary the lexer does not implement. Keep that distinction visible when adding tutorial examples.
