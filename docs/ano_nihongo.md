考案: Anghel (Anghel4d)

# ano 日本語 — a Japanese surface mode


ano's canonical surface is ASCII. This document defines a second concrete surface where Japanese grammatical particles inform the overall structure of the language at a semantic and syntactic level. This is because I think Japanese is pleasant, simple (grammatically), and has an exceptional internal consistency for an extant spoken language.

## Premise

ano's core form is head-final: `source & predicate , effect`. Selection on the left, effect on the right, the comma between. 

Japanese is also head-final and agglutinative. Its particles are postfix markers that attach to a noun and name its role. A postfix role-marker is a tacit postfix operator. Ano and Japanese have the same head-final postfix shape.

## Architecture: a reader skin

One AST, many readers. The grammar is defined over abstract token kinds (`AND`, `OR`, `HOP`, `GT`, `COMMA`, `ASSIGN_ADD`, `IDENT`, `NUM`). Each surface is a lexer that maps concrete glyphs to those kinds. と and `&` emit the same `AND`; everything downstream is identical. The Japanese mode is a lexer table plus a numeral reader. The parser, the type/footprint resolution, the bytecode, the JIT, none of them learn which surface produced the tokens. One scope note, argued at the end of this document: the lexer-table claim holds for the spaced particle surface defined here; the full-sentence surface keeps it only in the なる voice.

The 日本語ーmode is also a litmus test for ano's semantics. If adding the Japanese reader is trivial, the lexer, parser, and semantics are cleanly separated. Note to self: this project might be my opportunity to introduce my long-cherished coinage, the word "lexiom". I don't know what it means yet.

## Particle → operator map

| ano | particle | gloss | note |
|---|---|---|---|
| `&` and | と | "and", the listing particle | |
| `\|` or | か | "or" | か is also the question particle |
| `.` the dotted hop, `rel.Comp` | の | genitive "'s" | the relationship hop is literally the possessive |
| `@` region or scope | で | locative "at / in" | で marks where the action happens |
| `,` selection-effect separator | 、 / が | subject-predicate hinge | 、 is the surface mark; が is the grammar |
| omitted subject | ゼロが | zero pronoun | block-local anaphora or default subject |
| `>` | より | "than" | postfix order fixes the direction; bare 超 remains a noun |
| `<` | 未満 | "under" | the sign-board word, 18歳未満 |
| `==` | 同 | "same" | |
| `+=` | に … たす | "to X, add" | に marks the target, たす is add |
| `-=` | に … ひく | subtract | |
| `*=` | に … かける | multiply | |
| `/=` | に … わる | divide | |
| `=` set | を … にする | "make X be" | |
| `!` not | ない | the negator | postfix; spaced input keeps the boundary visible |

The standout is の. ano's relationship hop, `mentor.TwoHanded`, reads `師匠 の 両手`, mentor's two-handed. The genitive particle is the foreign-key join with no adaptation at all. で for `@` is as clean: a scope is a place, and で is the particle for the place an action occurs.

## Worked example

The canonical task, +1000 gold to every Nord with Two-Handed over 60.

ASCII ano:
```text
Nord & TwoHanded > 60 , Gold += 1000
```

Intermediate state:
```haskell
あの Nord & TwoHanded > 60 が Gold += 1000
```

Japanese, kanji:
```text
北 と 両手 六十 より 、 金 に 千 たす
```

北 Nord, と and, 両手 Two-Handed, 六十 より over sixty, 、 the written が, 金 に 千 たす is Gold += 1000. The token order is the ano token order with the operators moved to their postfix positions, which is where Japanese already puts them.

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

General Japanese segmentation is hard because the vocabulary is open. ano's vocabulary is closed. Component names are declared in the registry before evaluation, so at lex time the vocabulary is fixed and known: registry names, the particle and verb set, the numerals. Segmentation is maximal-munch against that closed dictionary, the registry being the dictionary, an ordinary lexer problem. Remaining ambiguity, a component name that ends in a particle homograph, resolves by longest match plus grammar position, since the parser only accepts a particle where an operator is legal. Spaced input needs none of this and is a pure table; ship it first and treat the unspaced reader as harder.

The spaced skin is registry-blind: every noun lexes to its surface spelling and resolves at emit. The unspaced reader inherits that constraint. It may segment by maximal munch, but it resolves the munched spans against the registry after lexing, never during, so the dictionary stays a resolution-time oracle and the two skins keep one parser.

## Negation, the one that fights back

Japanese negation is the postfix auxiliary ない. The executable authoring form is spaced, so `死 ない` is unambiguous; the unspaced reader may use its dictionary and grammar position but grants no new spelling. Classical ず is not a token, and 非 remains a legal identifier-start, so 非死 is one noun rather than a prefix form. This preserves postfix rhythm and the open noun space with one canonical negator.

## Surface rulings

- Direction of comparison. より is the sole executable `>` spelling. Postfix operand order supplies direction; bare 超 returns to the noun space, while natural full-sentence prose may still say 超える outside the reader skin.
- Negation. ない is the sole executable spelling, per above.
- Reduction and scan. The JA folds are the prefix words 総和 総積 総数 最大 最小 平均 皆 或 and the scans 累和 累積 累数 累小 累平均 累大 累皆 累或, each carrying the ASCII `/`/`\` op as payload; a fused reducer word (`脅威/`, `脅威\`) carries a named reducer the same way.
- Numeral policy is deliberately skin-specific. Kanji numerals read as numbers only under `--! ja`; on the ASCII surface 六十 remains an ordinary identifier, so a native column may own that spelling. The skins converge on AST meaning, not on which concrete words each lexer reserves.
- Verb-aware case frames stop at the shipped table tier. The spaced particle surface and the なる rule voice are executable; the free full-sentence する voice remains explanatory prose because per-verb case frames would be another grammar. `解除 name` is the exact Japanese twin of top-level `undef name`, not a conjugated effect.
- IME ergonomics. Writing kana code needs an input method, and the unspaced form is hard to type as well as to lex. The spaced form is the authoring surface; the unspaced form is accepted but not authored.
- Printing has two contracts. The semantic AST printer chooses the canonical atlas spelling for closed tokens in the requested skin and preserves every identifier byte exactly; a lossless formatter retains the original token stream separately. Semantic printing therefore normalizes operator spellings without rewriting an author's katakana, kanji, or native registry nouns.

## Status

The reader skin is shipped: Steel maps both surfaces to one AST and the active paired demos require byte-identical plans. Keep it a mode. The ASCII surface stays canonical; Japanese is the native teaching skin, and it is good at that because あの is already Japanese and the canonical task is, after all, paying Norsemen in their own grammar.

## Grammar beyond the operators

The structural reading of Japanese here follows Cure Dolly, who taught Japanese as a single consistent engine whose surface variety is all derived. Her channel, [Organic Japanese with Cure Dolly](https://www.youtube.com/@organicjapanesewithcuredol2667), and her book *Unlocking Japanese*, argue that one structure underlies every Japanese sentence: a subject marked by が, often the invisible ゼロが (the zero pronoun), joined to a predicate, with は a separate non-logical topic marker layered on top. Verbs, adjectives, and noun-sentences are that one engine in different costumes. That refusal of special cases is the design ethic ano borrows. Two notes on sources: Cure Dolly's engine is a pedagogical model, and が-centrality is contested in linguistics proper — Kuno's exhaustive-listing が against thematic は is a live literature. And the model has ancestry: the zero-pronoun treatment descends from Jay Rubin's *Making Sense of Japanese*, and the する/なる split in point 2 is Ikegami's typology. The numbered points below are where the grammar suggests features the array-programming surface never raised.

### 1. Relative clauses with zero ceremony

読んだ本 = "read book" = "the book that I read." A full clause sits in front of a noun and restricts it. No relativizer, no that, no WHERE, no lambda — juxtaposition is the filter. This is precisely "the predicate is the entity reference," and Japanese proves the form works with zero syntax. Attributive position means restriction, and entering it needs no keyword. ano can lean harder on this: any predicate adjacent to a component name restricts it, full stop.

### 2. する vs なる — agentive (imperative) vs spontaneous (declarative)

する is do-something-to-a-thing: an actor performs the change. なる is become: the world settles into a state on its own. The split is voice. Both verbs reach either a value or an identity — 値を倍にする makes the value double, 値が倍になる has it become double, the same column either way. So the pair maps imperative vs declarative: する is an effect the script performs, なる is a state a rule computes, the reactive register. The value-vs-structural split (column-write vs archetype-change) runs on a separate axis, carried by different verbs: 与える give, 失う lose, 生成 spawn. する/なる are the voice markers — する on a performed effect, なる on a reactive rule. The split is Ikegami's する-language/なる-language typology (『「する」と「なる」の言語学』, 1981), and the ASCII surface carries it at the hinge: `selection , effect` performs, the する voice; `selection => effect` installs a standing rule, the なる voice. Same left side, same right side, only the hinge changes.

### 3. Counters/classifiers — frame-typed numerals

三本 (3 long-things), 三匹 (3 animals), 三枚 (3 flat-things). The number carries the type/shape of what it counts; 三匹 cannot count a flat thing. ano's open question is coordinate frames — "a world-space `at` cannot silently consume a cell-space coordinate." Counters are the natural-language version of exactly that: a scalar tagged with its frame/unit, and a grammar that rejects the wrong counter for the noun. ano numerals could carry a frame counter checked at the で/`at` boundary. This is the one that turns an open question into a design. Fine print: counters are phonologically irregular — 一本 ippon, 三本 sanbon, 六本 roppon — so the semantic analogy survives and the total-regularity rhetoric does not. The irregularity is sandhi, not type structure; the frame check is regular even where the surface sound is not.

### 4. Agglutination — ordered combinator stacking, no parens

食べ-させ-られ-な-かった: eat-CAUSE-PASS-NEG-PAST. Suffixes stack in a fixed canonical order, each a unary transform on the predicate, and the order never varies, so no brackets are needed to disambiguate. This is APL adverbs/conjunctions and Forth-style postfix in one. ano's column transforms and effect modifiers (polarity, aspect, scope) can stack as ordered postfix morphemes with a total order on modifier classes — a precedence lattice that makes parens unnecessary by construction.

### 5. は vs が — ambient scope vs per-statement selection

Cure Dolly's central correction: が marks the real grammatical subject; は is a non-logical topic that sets a context and persists across sentences. That is a scoping construct. は = "as for Nords, …" establishes a default source for a run of effects; が = the actual selection in each clause. ano gets a native with-block from this: 北は … applies a chain of effects to an established subject without renaming it each time. Topic-chaining = ambient source that carries forward.

### 6. ゼロが (the zero pronoun) — block-local anaphora

Japanese omits any subject that context supplies; the が is frequently invisible and inferred. That is point-free programming. If the block has no subject, the host may let a registered effect declare a default subject. For instance, you may want to register `spawn` so a subjectless `spawn Wheat` lands at the raycast intersect drawn from the player's cursor to the ground. Or have heal target `player`, and so on.

In Japanese, the subject, if omitted from a sentence (as it most often is), is inferred contextually. 

Block-local anaphora (implicit subject)

```haskell
Nord & Dead , spawn Ghost
~   -- same subject, next barrier
```

```haskell
spawn Wheat    -- reasonable host default: cursor , spawn Wheat
```

### 7. こと/もの nominalizers — reify an effect as a value

こと turns an event/action into a noun — "the fact/event of X." This is first-class events. ano's effect buffer is already a described thing the host interprets; こと is the marker that lets a predicate-or-effect be captured as a value, passed, composed, deferred. Pairs with the Lisp/homoiconic core and the と-quotative (と(いう)), which is mention-vs-use — a quotation boundary for macros to take a predicate as data.

### 8. て-form — the sequencing/pipe operator

食べて寝る = eat-then-sleep: a connective that chains clauses into one flow sharing a subject. That is ano's sequenced effects (`;`-batched, one barrier) exactly — chain effects over a shared selection, then the barrier. て is the native "and then."

### 9. こそあど — deixis is the alias system and the frame origin

Japanese demonstratives are one engine indexed by one axis: こ proximal (near speaker), そ medial (near listener), あ distal (far from both), ど interrogative. これ/それ/あれ/どれ, ここ/そこ/あそこ/どこ, この/その/あの/どの — one mechanism, different referent by deictic distance. ano's target aliases are this engine: ^cursor, ^observer, ^world, dore are one deictic pointer indexed by referent. The rule it sets: aliases proliferate freely, one mechanism resolved per evaluation, while an operation stays in the calculus. ^cursor and ^world desugar to the same resolve with a different referent, so they are aliases; the old grid and fib desugar to different expressions, so they are operations wearing nouns.

The deictic axis is the coordinate frame. A generated lattice needs an origin, and the demonstrative supplies it: ^cursor is こ proximal, origin where the pointer meets the world; ^observer/^player is そ medial, origin at the subject; ^world is あ distal, the absolute world origin, "that one over there"; dore is ど, the query form, the ano/dore pair. The frame's origin is which demonstrative the selection points with. The language is named あの, the distal — address-by-description, the default for selecting things out there. The あ-series carries one more property: it marks a referent in the shared knowledge of speaker and hearer — あの人 is that person we both know. Read that way, あの is exactly right for a language whose scripts utter descriptions resolved against the common ground of the game world. Generation, being hands-on, defaults to この, proximal, anchored at the cursor.

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

ano's spec wants totality from finite combinators — every form derivable, no general fixpoint. Japanese verb conjugation is a total function (verb, function) → form with exactly two irregulars (する, 来る) and one fully-regular ichidan shortcut. The fine print: that count covers the conjugation wheel only — ある negates suppletively to ない, 行く takes the irregular onbin 行って, and です/ます run their own paradigm. That near-zero exception rate across every politeness register is the same aesthetic: a closed generating system where surface variety is all derived, never stored. The consistency across registers is the conjugation being orthogonal to register — politeness is a separate axis layered on the same engine.

### Enumeration + particle = pervasion

りんご と みかん と バナナ — the particle distributes one relation across a heterogeneous list, indifferent to each member's individual type. The particle is the operator; the list is the vector; application is element-wise. ano's masked column op is the same move: one operator pervades a selection uniformly, regardless of per-entity variation. Japanese lists are vectorized expressions where the particle is the verb.

### Modifiers fold in

i-adjectives conjugate on the same regular schedule (高い → 高く adverbial, 高かった past, 高くない negative); な-adjectives borrow the copula. Adjectives are the same engine, which is Cure Dolly's whole point about adjectives being secretly verb-like. One engine, three costumes.

### The punchline

Japanese is a head-final, closed-combinator system, regular to within the fine print above, operating on type-tagged nouns through postfix particles, with inflection as vowel-indexed gather and lists as pervaded vectors. Drop the phonology, which is where the residual irregularity lives, and that description is an array-relational language. ano and Japanese are two surfaces over the same underlying machine — selection, pervasion, gather, total combinators, no parens.

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

Japanese pairs a self-move intransitive with an other-move transitive for the same event, and the pair is derived by regular law: 開く and 開ける for opening, 上がる and 上げる for rising and raising, 閉まる and 閉める for closing, 出る and 出す for leaving and taking out. The self-move family descends from ある, the other-move family from する, and stem shape predicts which is which. ano derives the spontaneous (declarative) form and the agentive (imperative) form of an operation from one root by a regular transform — the same voice pairing as point 2.

### Three terminals behind one engine

Every sentence is the が-engine joining a carriage to a predicate, and the predicate terminates in one of exactly three kinds: a う-ending verb for an action, だ with a noun for an identity, or an い-adjective for a property. 桜が走る is an action, 桜が学生だ is an identity, 桜が高い is a property. One engine resolves to three terminal categories and no others. ano's selection engine resolves to the same three predicate kinds: action, identity, and property.

## The full Japanese surface

Here's what ano looks like if it leans all the way into Japanese grammar. Each example is a real grammatical Japanese sentence that *is also a program*, followed by ascii ano lang right under.

### 1. Gold to the mighty

``両手が六十を超える北に、金を千与える。``

```haskell
あの Nord & TwoHanded > 60 が Gold += 1000
```

```haskell
Nord & TwoHanded > 60 , Gold += 1000
```

Reads as "to the Nords whose two-handed exceeds sixty, give a thousand gold." The WHERE is a relative clause (両手が六十を超える), and the selection is literally a Japanese subordinate clause. Mind the case frame, though: に sits on the selected Nords, を on the gold column, and the subject of 与える is a zero-が agent — the script itself. What that costs is settled after example 6.

### 2. The fallen master

``師匠が死んだ北は、訓練を失う。``

```haskell
Nord & mentor.Dead , -Trained
```

Reads as "Nords whose master has died lose their training." The dotted hop mentor.Dead is the relative clause 師匠が死んだ — "master-[が]-died" modifying 北. The foreign-key join is just... a subordinate clause. は sets the topic, を失う is the effect.

### 3. The county's wealth (a fold)

``北全員の金の総和。``

```haskell
+/ Gold @ Nord
```

Reads as "the grand total of all Nords' gold." A pure noun phrase. Each の is a gather-hop (全員 の 金 = everyone's gold), and 総和 is the +/ reduction. The fold isn't a verb here — it's a thing you name, which is exactly what a reduction is.

### 5. The cellar (counters + becoming)

``蔵で三ヶ月より熟成したチーズは、値が倍になる。``

```haskell
Cheese @ cellar & Aged > 3mo , Price *= 2
```

Reads as "cheese aged in the cellar longer than three months — its price doubles." This is the magnificent one: 三ヶ月 is a frame-typed numeral (the ヶ月 counter is the 3mo unit, checked by the grammar), 熟成した is the selection-clause, and 値が倍になる is the value effect via なる. The coordinate-frame open question and the action both fall out of ordinary counting and ordinary becoming. Careful, though: the ASCII keeps the comma, a command performed once; installed as a standing `=>` rule it would re-gather and double every tick. The sentence's becoming voice does not force the rule register on the program.

## What this document is for

This file carries two jobs at once. It is the design's guiding line: Japanese is the intuitive substrate the surface is being fitted to, and a large share of the language's eventual players will read this surface natively or near it — Japanese players outright, and the N5-and-up crowd for whom 北に金を千与える is legible on sight. When an ASCII design question stalls, the Japanese answer is the tiebreaker. And it is the source of a future paper. "Japanese inspired our syntax" is a workshop poster; language-inspired syntax is everywhere, language-inspired semantics is not. The publishable thesis is the strong version the sections above already demonstrate: case-marking grammar is an executable query semantics. The particle system is a role-assignment calculus — に/を/で/が as typed argument slots — the gapless prenominal relative clause is intensional reference with no relativizer and no lambda, selection by juxtaposition, and Ikegami's する/なる typology lands as the command-vs-standing-rule evaluation split. Each is a semantic correspondence with a running artifact behind it, which is what separates the paper from the long tradition of linguistics-flavored notation.

The paper's teeth are the results this document already states, and two of them are negative, which is what makes the mapping falsifiable rather than decorative. The lexer-table boundary result: the full-sentence surface is a pure reader skin exactly in the なる voice, and fails for する/与える because に flips case frames with verb choice — a clean, checkable claim about where grammar-as-syntax ends and verb semantics begins, Levin-style valency theory meeting compiler front-ends. The closed-registry segmentation result: general segmentation is hard because the lexicon is open; the registry closes it before lex time, and unspaced kana reduces to maximal munch — a clean, defensible result on its own. Counters as unit types: semantically exact, phonologically irregular — the analogy lives at the type level and dies at the surface. And the deixis system: こそあど as coordinate-frame origins, with the あ-series shared-knowledge reading giving the language's own name a formal semantics (the common ground is the game world state) — Kaplan-style indexicals implemented as scope resolvers, which no running language has done.

The gap between this document and a draft is mostly literature positioning: engaging Ikegami and Kuno seriously rather than citing them in passing, and setting the contrast class — situated language understanding, SHRDLU through modern instruction-following, where everyone parses natural language into commands. The claim here is stranger and cleaner: the grammar is the command language, no NLU step, just a lexer table within a provable boundary. Beyond that, the missing work is formalizing the particle→operator map as an actual typed translation, stating the する case-frame grammar the boundary forces as a grammar, and the related-work section itself; this file is roughly 60% of the draft. The artifact section is a playable console.

## Citations

- Cure Dolly. *Organic Japanese with Cure Dolly*, video lessons and articles. [YouTube channel](https://www.youtube.com/@organicjapanesewithcuredol2667), [learnjapaneseonline.info](https://learnjapaneseonline.info/). A pedagogical model, cited as such; が-centrality is contested in linguistics proper.
- Cure Dolly. *Unlocking Japanese: Making Japanese as Simple as It Really Is*. 2016. ISBN 978-1539485506.
- 池上嘉彦 (Ikegami Yoshihiko). 『「する」と「なる」の言語学』 (*Suru to Naru no Gengogaku*). 大修館書店, 1981. The する-language/なる-language typology behind point 2.
- Jay Rubin. *Making Sense of Japanese: What the Textbooks Don't Tell You*. Kodansha International, 1998. The zero pronoun behind ゼロが.
- Susumu Kuno. *The Structure of the Japanese Language*. MIT Press, 1973. Exhaustive-listing が against thematic は — the live literature.

## Steel

Two facts from the implementation, recorded so the sections above stay as written.

The spaced skin ships as the pure lexer table claimed above, with one measured boundary: a postfix operator re-roots before exactly one primary — an atom, a matched group, a hop chain, a shape run — so an operand wider than one primary must be author-parenthesized (`金 に ( 千 + 三 ) たす`), and the unparenthesized form is a hard error, never a silent misparse. That parenthesization rule is where the lexer-table claim measurably ends; everything else in the table held.

Registry nouns are natively UTF-8. A world declares `col 金 num …` and both surfaces address 金 by that name — identifiers on either surface admit any codepoint at or above U+0080, and the closed grammar (particles, keywords, numerals) outranks all nouns, so no registry name can shadow と or 千; the loader rejects the collision by word. `ja` lines remain as an optional cross-surface alias — `ja 北 nord` lets one world be scripted idiomatically from both surfaces — applied at name resolution after exact entry names, never in the lexer.
