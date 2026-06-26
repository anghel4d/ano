考案: Anghel (Anghel4d)

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
| `!` not | ず, ない | the negator | postfix, and it bites, see below |

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

## Grammar beyond the operators

The structural reading of Japanese here follows Cure Dolly, who taught Japanese as a single consistent engine whose surface variety is all derived. Her channel, [Organic Japanese with Cure Dolly](https://www.youtube.com/@organicjapanesewithcuredol2667), and her book *Unlocking Japanese*, argue that one structure underlies every Japanese sentence: a subject marked by が, often the invisible ゼロが (the zero pronoun), joined to a predicate, with は a separate non-logical topic marker layered on top. Verbs, adjectives, and noun-sentences are that one engine in different costumes. That refusal of special cases is the design ethic ano borrows. The numbered points below are where the grammar suggests features the array-programming surface never raised.

### 1. Relative clauses with zero ceremony

読んだ本 = "read book" = "the book that I read." A full clause sits in front of a noun and restricts it. No relativizer, no that, no WHERE, no lambda — juxtaposition is the filter. This is precisely "the predicate is the entity reference," and Japanese proves the form works with zero syntax. Attributive position means restriction, and entering it needs no keyword. ano can lean harder on this: any predicate adjacent to a component name restricts it, full stop.

### 2. する vs なる — value effects vs structural effects

する is do-something-to-a-thing (agentive, mutate a value). なる is become (change of state/identity). ano already splits value effects (`Gold += 1000`) from structural effects (archetype change, despawn). Japanese has the exact verb pair: する-effects write columns, なる-effects change which archetype an entity is. This is the most exact mapping in the set, two native verbs for the two effect kinds ano already has. They could be the literal effect markers.

### 3. Counters/classifiers — frame-typed numerals

三本 (3 long-things), 三匹 (3 animals), 三枚 (3 flat-things). The number carries the type/shape of what it counts; 三匹 cannot count a flat thing. ano's open question is coordinate frames — "a world-space `at` cannot silently consume a cell-space coordinate." Counters are the natural-language version of exactly that: a scalar tagged with its frame/unit, and a grammar that rejects the wrong counter for the noun. ano numerals could carry a frame counter checked at the で/`at` boundary. This is the one that turns an open question into a design.

### 4. Agglutination — ordered combinator stacking, no parens

食べ-させ-られ-な-かった: eat-CAUSE-PASS-NEG-PAST. Suffixes stack in a fixed canonical order, each a unary transform on the predicate, and the order never varies, so no brackets are needed to disambiguate. This is APL adverbs/conjunctions and Forth-style postfix in one. ano's column transforms and effect modifiers (polarity, aspect, scope) can stack as ordered postfix morphemes with a total order on modifier classes — a precedence lattice that makes parens unnecessary by construction.

### 5. は vs が — ambient scope vs per-statement selection

Cure Dolly's central correction: が marks the real grammatical subject; は is a non-logical topic that sets a context and persists across sentences. That is a scoping construct. は = "as for Nords, …" establishes a default source for a run of effects; が = the actual selection in each clause. ano gets a native with-block from this: 北は … applies a chain of effects to an established subject without renaming it each time. Topic-chaining = ambient source that carries forward.

### 6. ゼロが (the zero pronoun) — tacit arguments

Japanese omits any subject that context supplies; the が is frequently invisible and inferred. That is point-free programming. Combined with #5: the implicit subject of an effect is the current topic/selection, named only to disambiguate. ano's `@cursor`/`@observer` are already implicit-context predicates — generalize it to a default "zero" subject so effects apply to the ambient selection tacitly.

### 7. こと/もの nominalizers — reify an effect as a value

こと turns an event/action into a noun — "the fact/event of X." This is first-class events. ano's effect buffer is already a described thing the host interprets; こと is the marker that lets a predicate-or-effect be captured as a value, passed, composed, deferred. Pairs with the Lisp/homoiconic core and the と-quotative (と(いう)), which is mention-vs-use — a quotation boundary for macros to take a predicate as data.

### 8. て-form — the sequencing/pipe operator

食べて寝る = eat-then-sleep: a connective that chains clauses into one flow sharing a subject. That is ano's sequenced effects (`;`-batched, one barrier) exactly — chain effects over a shared selection, then the barrier. て is the native "and then."

## The engine underneath

### Rotary conjugation = vowel-indexed paradigm

書く factors into an invariant consonant root `kak-` and a vowel that rotates through あ-い-う-え-お to select the grammatical function:

- a 書か → negative/causative/passive base (未然)
- i 書き → the masu-stem, nominal, compounding form (連用)
- u 書く → plain nonpast (dictionary)
- e 書け → potential/conditional/imperative (已然/命令)
- o 書こ → volitional

This is indexing a 5-row paradigm table by vowel; the forms are generated, never memorized. Grammatical function is a base-5 index; the root is the data; selecting a form is a gather `forms[mood]`. And the same vowel-rotation applies uniformly across every godan verb — one operation broadcast over the whole class. That is pervasion / rank polymorphism: one operator, element-wise, type-agnostic.

The factoring is the deep part: shared root + varying inflection is columnar storage. The stem is the dense shared data, the vowel is the per-cell variation. Japanese conjugation stores verbs the way ano stores components — invariant down the column, raggedness only at the inflectional cut.

### Total, two exceptions

ano's spec wants totality from finite combinators — every form derivable, no general fixpoint. Japanese verb conjugation is a total function (verb, function) → form with exactly two irregulars (する, 来る) and one fully-regular ichidan shortcut. That near-zero exception rate across every politeness register is the same aesthetic: a closed generating system where surface variety is all derived, never stored. The consistency across registers is the conjugation being orthogonal to register — politeness is a separate axis layered on the same engine.

### Enumeration + particle = pervasion

りんご と みかん と バナナ — the particle distributes one relation across a heterogeneous list, indifferent to each member's individual type. The particle is the operator; the list is the vector; application is element-wise. ano's masked column op is the same move: one operator pervades a selection uniformly, regardless of per-entity variation. Japanese lists are vectorized expressions where the particle is the verb.

### Modifiers fold in

i-adjectives conjugate on the same regular schedule (高い → 高く adverbial, 高かった past, 高くない negative); な-adjectives borrow the copula. Adjectives are the same engine, which is Cure Dolly's whole point about adjectives being secretly verb-like. One engine, three costumes.

### The punchline

Japanese is a head-final, totally-regular, closed-combinator system operating on type-tagged nouns through postfix particles, with inflection as vowel-indexed gather and lists as pervaded vectors. Drop the phonology and that description is an array-relational language. ano and Japanese are two surfaces over the same underlying machine — selection, pervasion, gather, total combinators, no parens. The particle mapping was discovery because the machine was already shared.

## Further regularities

### Conjugation is recursive

Japanese verb forms compose by attaching helpers, and each helper is itself a verb or an い-adjective that re-enters the same stem system. 食べる takes the causative as 食べさせる, which takes the receptive as 食べさせられる, which takes the desiderative たい as 食べさせられたい, which negates as 食べさせられたくない, which goes to the past as 食べさせられたくなかった. させる is a verb, られる is a verb, たい is an い-adjective, ない is an い-adjective, and every one conjugates on the same あ-い-う-え-お wheel as the root. The system is self-similar: each layer is one more rotation of the one wheel, so the structure is a helix, each layer a further turn of the same gear. ano's modifiers are themselves ano expressions that re-enter the evaluator, so the transform stack is closed under the language, with no separate suffix table and no meta-layer.

### Tense is marked once

Tense lands once per sentence, on the outermost helper of the stack, and never on the inner layers. 食べさせられたくなかった carries past only on the final ない turning to なかった; the causative, receptive, and desiderative layers under it stay tenseless. A continuous statement puts the tense on the auxiliary いる, so 走っている becomes 走っていた and the main verb is untouched. ano applies aspect and commit once, at the top of a stacked predicate, the same single-application discipline as the one-barrier model for sequenced effects.

### た and て are one transformation

The past marker た and the connective て share identical sound-change rules and differ only in the final kana. 書く gives 書いて and 書いた, 飲む gives 飲んで and 飲んだ, 行く gives 行って and 行った. The euphonic shift is computed once, and the terminal kana selects connective or past. The and-then form and the past form are one operation with two terminals. ano treats sequencing and commit the same way: chaining effects over a shared selection and closing the barrier are two terminals of one form.

### Negation is an adjective

Negation is carried by ない, an い-adjective meaning non-existent, attached at the あ-stem, so 書く becomes 書かない. Because ない is an ordinary adjective, it inflects by the ordinary adjective rules, and the past negative is 書かなかった by the universal い-to-かった shift, the same shift that takes 高い to 高かった. Polarity is a word in the lexicon, and absence is a value. ano treats absence the same way: the present, any-value, and absent states of a component are ordinary predicate values, and absence composes like any other.

### One logical role per noun

The logical particles が, を, に, へ, で mark structural roles, and a single noun carries exactly one of them per clause; two logical particles cannot sit on the same noun. The non-logical markers は and も sit outside this set: they layer over に, で, へ, and から (には, では, からは, stacking a topic over a target, a location, or a source), and they replace が or を instead of stacking on them. Role assignment is one per operand, and scope is an orthogonal layer above it. ano gives each operand one structural role per clause and lets scope markers layer over role markers without conflict.

### Transitivity comes in regular pairs

Japanese pairs a self-move intransitive with an other-move transitive for the same event, and the pair is derived by regular law: 開く and 開ける for opening, 上がる and 上げる for rising and raising, 閉まる and 閉める for closing, 出る and 出す for leaving and taking out. The self-move family descends from ある, the other-move family from する, and stem shape predicts which is which. ano derives the value-effect form and the structural-effect form of an operation from one root by a regular transform.

### Three terminals behind one engine

Every sentence is the が-engine joining a carriage to a predicate, and the predicate terminates in one of exactly three kinds: a う-ending verb for an action, だ with a noun for an identity, or an い-adjective for a property. 桜が走る is an action, 桜が学生だ is an identity, 桜が高い is a property. One engine resolves to three terminal categories and no others. ano's selection engine resolves to the same three predicate kinds: action, identity, and property.

## The full Japanese surface

Here's what ano looks like if it leans all the way into Japanese grammar. Each example is a real grammatical Japanese sentence that *is also a program*, followed by ascii ano lang right under.

### 1. Gold to the mighty (the canonical)

```text
両手が六十を超える北に、金を千与える。
```
```text
Nord & TwoHanded > 60 , Gold += 1000
```
Reads as "to the Nords whose two-handed exceeds sixty, give a thousand gold." The `WHERE` is a relative clause (両手が六十を超える), `に` is the `+=` target, the 、 is the comma. The selection is literally a Japanese subordinate clause.

### 2. The fallen master

```text
師匠が死んだ北は、訓練を失う。
```
```text
Nord & mentor.Dead , -Trained
```
Reads as "Nords whose master has died lose their training." The dotted hop `mentor.Dead` is the relative clause 師匠が死んだ — "master-[が]-died" modifying 北. The foreign-key join is just a subordinate clause. `は` sets the topic, `を失う` is the effect.

### 3. The county's wealth (a fold)

```text
北全員の金の総和。
```
```text
+/ Gold @ Nord
```
Reads as "the grand total of all Nords' gold." A pure noun phrase. Each `の` is a gather-hop (全員 の 金 = everyone's gold), and 総和 is the `+/` reduction. The fold is a named thing here, which is exactly what a reduction is.

### 4. Fibonacci down the row (spatial generation)

```text
升目の列で、各升の高さは前の二升の和となる。
```
```text
Cell @ row , Height = prev.Height + prev.prev.Height
```
Reads as "in the row of squares, each square's height becomes the sum of the two squares before it." `で` scopes to the lattice, `は` distributes over each cell (pervasion), the recurrence is 前の二升の和 ("the sum of the previous two squares"), and `となる` is the becoming, a structural generation.

### 5. The cellar (counters + becoming)

```text
蔵で三ヶ月より熟成したチーズは、値が倍になる。
```
```text
Cheese @ cellar & Aged > 3mo , Price *= 2
```
Reads as "cheese aged in the cellar longer than three months — its price doubles." This is the magnificent one: 三ヶ月 is a **frame-typed numeral** (the ヶ月 counter *is* the `3mo` unit, checked by the grammar), 熟成した is the selection-clause, and 値が倍になる is the value effect via なる. The coordinate-frame open question and the action both fall out of ordinary counting and ordinary becoming.

The thing that makes these sing: in every one, the program parses as a sentence and the sentence parses as a program. The relative clause is the selection, `の` is the join, the counter is the type, `に` is the assignment target, `なる` is the structural effect, 、 is the comma. The discovery underneath: a 1000-year-old grammar was already a query-and-update language, and nobody noticed.

## Citations

- Cure Dolly. *Organic Japanese with Cure Dolly*, video lessons and articles. [YouTube channel](https://www.youtube.com/@organicjapanesewithcuredol2667), [learnjapaneseonline.info](https://learnjapaneseonline.info/).
- Cure Dolly. *Unlocking Japanese: Making Japanese as Simple as It Really Is*. 2016. ISBN 978-1539485506.
