# anoc surface grammar

The implementation contract for the C transpiler in this directory. Derived from ano-language.md (grammar appendix, the 14-level precedence table) and ano-examples.md (ex1-49). This file adds nothing to the language; where it and ano-language.md disagree, ano-language.md wins.

## Lexical

- Source is strict UTF-8, validated whole before either mode lexes: overlongs, encoded surrogates, and codepoints past U+10FFFF are a lex error ("malformed UTF-8"). A well-formed but unexpected character reports by codepoint (`unknown character U+2195`).
- Comments: `--` to end of line. `--!` is a harness directive, not a comment (below).
- Names: `[A-Za-z][A-Za-z0-9_]*`, extended past ASCII: any codepoint >= U+0080 may start or continue a name, except the blacklist U+3000 ideographic space, U+3001 、, U+30FB ・ (whitespace and numeral machinery), and the retired U+2195. Maximal munch stops at ASCII operator bytes, so unspaced `北&両手>60` lexes NAME AMP NAME GT NUM. Kanji numerals are ordinary identifier chars on this surface — `六十` is a name here, a numeral only under `--! ja`. The case contract: registry names are case-insensitive (ASCII fold, non-ASCII bytes exact); values — `:Sym` symbols, sym column values, char glyph runs — and program-level names — def heads, comprehension binders — are case-sensitive, always. Case is preserved in the token; resolution folds it: registry entries first, then the registry's name aliases (`as`/`ja`, one hop, same fold), so `Gold` reaches column `gold` and an aliased kanji noun is addressable from the ASCII surface too. The word-class boundary: the closed grammar — keywords, `.reg` directive words, the emitter's contextual specials `index x y char row prev` and builtin callables (`rank`, `abs`, `sin`) — stays exact lowercase; only the registry's vocabulary folds. The shadow rule is unchanged, just wider: the probe-guarded specials (`x`, `y`, `row`, `prev`, `neighbor`) take a registry shadow under the fold exactly as they did exact-byte (the `ja x x` identity-alias worlds are the precedent); `index` and `char` never shadowed and still don't. A name is component, callable, binding, or def — sentence position decides (spec, flat namespace).
- Alias sigil: `^` immediately followed by a name, no whitespace, lexes as one T_ALIAS token. A bare `^` is a lex error. `@` is always the scope operator T_AT: `Merchant @ Whiterun` and `Adj@row` are both `… AT …`, spacing irrelevant.
- Symbols: `:` immediately followed by a name: T_SYM. A backtick is a lex error.
- Both sigil names follow the identifier policy above, UTF-8 included: `^世界`, `:山賊` are legal on both surfaces, so kanji sym values and aliases are writable.
- Numbers: integers and decimals (`60`, `0.5`, `1.05`). A number immediately followed by a name with no whitespace is a counter-typed numeral T_COUNTER (`3mo`: value 3, unit `mo`).
- Strings: `"..."`, no escapes needed for the demos: T_STR.
- Wildcard: `_` alone: T_WILD (`to 4 _`, `(Nord, TwoHanded _)`).
- Fold tokens: `+/ */ &/ |/ #/` and any non-keyword name directly followed by `/` (`max/ min/ avg/ threat/`, UTF-8 names included: `脅威/`) lex as one T_FOLD with an op payload — the lexer is registry-blind, so an unregistered name in fold position fails at emit, not here. Scan tokens: `+\ *\ &\ |\` and `name\` likewise, as T_SCANOP. No whitespace inside either; `max/= 2` stays SLASHEQ.
- `/` between value expressions is division and must be spaced on its left (`Dmg*Spd / Rng`): a name with `/` glued after it IS the fold token. Same for `\`.
- The generator: `til` (q's own word) is T_IOTA. `↕` is retired from both surfaces; U+2195 stays blacklisted so a stale `↕5` errors by codepoint.
- Operators: `, => ; |> & | ! == != < <= > >= = += -= *= /= + - * / % @ . ' ( ) [ ] <- ~`.
- The tick `'` is postfix on a name: `targets'`, `neighbors'`. Lexes as T_TICK following T_NAME.
- Keywords (closed set, the lexer owns them): `def spawn at to via along order by take desc top grade fold scan scan2 cross expand til`. Everything else, `index rank prev neighbor char x y row` included, is a name resolved by the registry or the prelude.

## Directives

`--! key rest-of-line`, harness-owned, one per line:

- `--! registry <path-or-name>` — load the world. A relative path resolves against the source file (`../registries/x.reg`); a bare name loads `<name>.reg` from beside it; an absolute path is used verbatim.
- `--! expect <col> = v v v ...` — post-state assertion on a numeric/bool column, world order. For symbol columns values are bare names (`Hostile`). For a char column the value is the glyph run, compared exactly (`--! expect 印 = .#..#.`). For a lattice field, row-major.
- `--! expect-n <k>` — post-state world row count (after spawn/despawn).
- `--! out <v v v ...>` — the next query statement's printed value equals this (scalar or vector).
- `--! ja` — the statement lines of this file are in the Japanese surface; lex with the JA skin.

## Statements

A program is a sequence of lines. Blank lines and comments are skipped. Each surviving line is one statement, one barrier — with one exception: a rule install is not a barrier of its own; rules fire at the harness's clock ticks, all installed rules in one shared barrier per tick (the def-rule entry below).

- `def <name> = <selection-or-column-expr>` — names a predicate or derived column; inlined at use (N_DEF). A def name is a program variable: exact-byte, outside the registry's case contract — `def ridge` beside a column `Ridge` is two names, the def hit only at its exact spelling, every other spelling folding to the column. An exactly lexer-reserved word cannot head a def (`def til` rejects; `def Til` is a distinct name and legal).
- `def <name> = <selection> => <effects>` — a standing rule (installed; the clock fires it). anoc has no clock, so it pretends one tick at the end of each unbroken run of def lines (plain defs don't end a run; a performed statement or end of file does). Each tick fires EVERY rule installed so far — installs persist, so a rule installed early fires again at a later tick, exactly as it would on the engine's next beat (n5-c) — in one shared barrier: every rule's mask gathers the one pre-state, every effect stages into one commit set, the set scatters once — the §11 shared rule barrier. Same-column writes across a tick's rules go through the §10 merge laws, widened by the guard-complement clause: `!X` in one rule's top-level `&`-chain and `X` in another's certify the two masks row-disjoint over the shared pre-state, licensing any pair of write families short of a verb dispatch (bloom/wither on planted, c3-b); a pair with neither is rejected. All rules in a tick must select in one frame (an anoc restriction, recorded in ISSUES.md).
- `eval "<statement>"` — the quotation splice: the literal string is re-lexed and its single statement parsed in place, at compile time, so the spliced footprint stays visible to every static check. One statement per quotation; the string must be a literal (dynamic strings are interpreter territory, not this). `eval` is a contextual statement head, not a lexer keyword — the keyword set above is unchanged.
- `<selection> , <effects>` — the command form. The first top-level `,` (not inside parens/brackets) is the hinge.
- `<selection> => <effects>` — anonymous standing rule.
- `, <effects>` — leading-comma continuation: new statement, new barrier, over the antecedent's saved mask.
- `~` alone — despawn the saved mask (continuation).
- `<effects>` with no hinge and an effect-verb head (`spawn`, `+Name`, `-Name`, registered effect verb) — elided subject: the saved mask when one exists, else the host default `^cursor`.
- anything else — a query statement: evaluate, print (checked by `--! out`).

## Precedence (loosest to tightest, from the spec appendix)

1. `,` `=>` hinge | 2. `;` | 3. `|>` | 4. effect verbs and assignment (`= += -= *= /=`, `+Comp -Comp ~ spawn`, `at`, `to`, `*` in `spawn X * n`) | 5. `|` | 6. `&` | 7. `!` prefix | 8. `== != < <= > >= =`(selection position) | 9. fold/scan prefixes, `fold(f)`, `scan(f)…along`, `grade`, `top k` | 10. `+ -` | 11. `* / %` | 12. `@` scope | 13. `.` and `'` | 14. atoms.

Worked: `Cheese @ cellar & Aged > 3mo` = `(Cheese @ cellar) & (Aged > 3mo)`; `Cow & Weight < avg/ Weight @ Cow` = `Cow & (Weight < (avg/ (Weight @ Cow)))`.

The fold prefix at level 9 takes the longest expression at level ≥10 to its right, then an optional `@ scope` (the scope binds to the fold: `+/ Gold @ Nord` folds Gold gathered by Nord). `#/` takes a parenthesized mask: `#/ (Nord & TwoHanded > 60)`, `#/ (livestock' & Cattle)`.

## Selection forms

- name (component mask), `^alias`, proper-noun binding, `def` name.
- `&` `|` `!` over masks; comparisons over column expressions (`=` compares left of the hinge).
- `(A, B, ...)` presence tuple in predicate position: `(Nord, TwoHanded > 60)` present+constrained, `(Nord, TwoHanded _)` present any, `(Nord, !TwoHanded)` absent.
- `rel.Comp` functional hop (dangling `¯1` fails the row), chainable. `sel.rel'` image (source position). `rel'` fiber (under folds/quantifiers only). `rel'.Comp` gathered fiber column. A key-valued num column may itself stand in relation position: `Col'` fibers are the inverse image over the stable-id column — the value-level rel (w3-c).
- `mask @ Binding` locative scope. `expr @ w h` / `expr @ n` fold scope over a lattice/line.
- `mask @ frame(args…) at originExpr` — the anchored frame: `@` fixes the frame, the locative `at` fills its origin. The origin must be one point: a pair mirror-read (`Firebolt.pos`, `^cursor.pos`) or a `point` binding (`impact`) — a bare alias is a mask, not a point, and is rejected. The registered frame fn runs per cell as `Fn ⟨cell, origin, args…⟩` over the x/y coordinates (registered fields win, else the lattice frame's computed coords) and its mask ANDs into the left arm.
- numeric shape in source position: `12` (line), `8 8` (lattice; coordinate columns x, y).
- `"glyphs" to 8 8` board literal source, `char` the per-cell column.
- pipeline: `src |> order by <col> [desc] |> take k |> expand <col>` in any sensible order.
- `top k (grade [desc] <col> [@ scope])` — the graded top-k selection.
- comprehension: `[ <sel> , <effect> | a <- A, b <- B, <filter>... ]` — a statement form (σ over the cross of generators; binders name per-side rows; filters may call registered functions `dist(t, c)`).

## Effects

`;`-separated, all against the statement's pre-state, committed at one barrier:

- `Col op expr` with op in `= += -= *= /=`; target may be `pos.x` (field projection).
- `+Comp` / `-Comp` — add/remove component (presence write).
- `~` — despawn.
- `spawn Proto` | `spawn Proto * countExpr` | `spawn Proto at posExpr` | `spawn (pieceOf char) ...` — mint rows; `index` is bound per copy (0.. within each source's copies); copies read their source's columns. A statement may batch several spawn effects: each appends its own row group in effect order, keys mint once across the batch.
- `Verb args` — registered effect verb (`Knockback 5`, `Flash :Red`, `runBehaviorTree`).
- `fn via Col` — Tier-3 dispatch (`shortestPath via Adj`).

Same-column batches commit through the spec's merge laws (§10): the additive family (`+=` `-=`) and the multiplicative family (`*=` `/=`) compose by rebasing the later accumulate on the earlier commit (each RHS still observes pre-state); presence writes of one kind compose idempotently; pair-field SETs compose when the fields differ. Any other pair on one column — `+=` beside `*=`, a double SET, anything beside a verb — is rejected at emit time ("no merge law: written twice in one barrier").

RHS evaluation space: a column expression on the effect side is evaluated over the full world, gathered by the selection at the scatter (`sel/rhs`), and written back under the mask; `index` is the ordinal within the selection (or within the copy run under spawn-replicate); `to shape` produces exactly count-of-selection positions. A `scan(f) … along order` value arrives in along order and its write-back conjugates through the order's grade (sort, act, unsort — the Tier-2 law); it keeps its order through scalar arithmetic and is rejected when composed against a differently-ordered column. Scan steps: `+ * max min`; anything else is rejected, never guessed.

## Japanese skin (`--! ja`)

Token-level, space-separated surface per ano_nihongo.md, covering the whole demo corpus. Per word, in order: `^alias` and `:Sym` sigils (identifier names, UTF-8 included: `:山賊`); the particle/keyword/fold table — the closed grammar outranks all nouns, so と is the particle whatever the registry says; kanji, fullwidth, and Arabic numerals (`六十`→60, `千`→1000, `九千九百九十九`→9999, `一・〇五`→1.05, `三ヶ月`→counter 3mo); a fused reducer word — a name with `/` or `\` glued after it (`脅威/`, `threat\`), the ASCII fold/scan fusion as one word; then any legal identifier, ASCII or UTF-8, as a keyword (`til`, `def`) or T_NAME carrying its surface spelling; a `"string"`; anything else errors by word. Nouns resolve at emit, never in the lexer: registry entry names first, then the registry's alias table (`as`/`ja`), every hop under the one case fold — the alias is optional plumbing, not the noun mechanism; a natively Japanese registry (`col 金 num …`) needs no alias lines at all, and an alias works from either surface. The system nouns `前`/`行`/`番号`/`字` are global table entries emitting T_NAME prev/row/index/char; a registry entry of that name wins at resolution. The loader rejects any entry name or alias source word the lexer owns on either surface under the fold (particles, keywords, numerals, fused reducers) — by word, at load.

The table splits by position. Postfix (written after the operand, re-rooted before it): comparisons `より`/`超`→`>`, `未満`→`<`, `以上`→`>=`, `以下`→`<=`, `同`→`==`, `不同`→`!=`, negation `ない`→`!`; the assignment family `たす`/`ひく`/`かける`/`わる`→`+= -= *= /=` and `にする`→`=` (with `に`→TGT marking the target, dropped after the re-root); the scope `で`→`@`; presence `付`→`+Comp`, `除`→`-Comp`. Infix particles: `と`→`&`, `か`→`|`, `の`→`.`, `、`/`が`/`は`→hinge. Prefix (ASCII order): the folds `総和 総積 総数 最大 最小 平均 皆 或`→`+/ */ #/ max/ min/ avg/ &/ |/`, the scans `累和 累積 累大 累皆 累或`→`+\ *\ max\ &\ |\`, the generator `連番`→`til`, `なる`→`=>`, and the keywords `定義 生成 於 至 経由 沿 整列 別 取 降順 上位 格付 縮約 走査 二重走査 交差 展開`→`def spawn at to via along order by take desc top grade fold scan scan2 cross expand`. Arithmetic `+ - * / %`, grouping `( ) [ ] ; <- |> ' _`, and `=` in a def head stay ASCII glyphs; `て`→`;`.

Normalization re-roots each postfix operator before its operand span — one primary: an atom, a matched `(…)`/`[…]` group (with a leading callee name folded in, so `blast(5)`→one operand), a `.`-hop chain, a numeric/wildcard shape run (`8 8`, `4 _`), or a leading `to`. The `に` target barriers an assignment's left side from the callee grab, so `Col に (RHS) たす` re-roots the verb to just after `Col`. A right side wider than one primary is parenthesized by the author. The result is the ASCII token stream; parsing proceeds identically. `--tokens` prints the normalized stream with surface spellings; `--! same-tokens <ascii>` asserts kind/num equality against a hand-written ASCII line, names compared through the resolver (ex40).

## Registry files (.reg)

Line-based, space-separated, `#` comments at word boundaries only (mid-word `#` survives glyph payloads). The world a demo runs against; `reg_load` (registry.c) is the text frontend's constructor and `--dump` its write-out — staged file, then rename(2), the atomic commit.

- `n <count>` — entity rows; precedes anything counted against it. `lattice <w> <h>` precedes `field`.
- `col <name> <num|bool|sym|char|vec> <values>` — num/bool take n numbers, sym takes n words (values, exact bytes), char takes the raw line tail (length checked in bytes against the row count), vec takes `|`-separated pairs. `field <name> <type> <values>` is the same over w·h cells.
- `pres <col> <mask>` and `default <name> <v>` — presence and spawn default, declared after their entry. Both reject on a `unique` column (a key column is total; its values are minted, never defaulted).
- `unique <name> <values>` — a num column under the injectivity constraint: pairwise-distinct, checked at load (the repeat is named with both rows). Subsumes the id/keys name magic as declared evidence; spawn mints `1+max`, effects may never write it.
- `rel <name> <n values>` — functional, -1 dangling, keyed to the row index. `rel <keycol> <name> <n values>` — the keyed form: two names before the data means the first is the key column (a declared `unique` column, negatives refused) and the second the rel's name; the hop then resolves stored ids by index-of against the key with one found-guard, so a despawned target fails the row, never faults. LL(1)-clean: rel/srel data is always numeric-or-`|`. `srel <name> <fibers |-separated>` — set-valued, empty fibers legal; keys the same way. `inv <name> <rel>` — the inverse read, fibers computed at load; the inverse of a keyed rel is keyed automatically.
- `def <name> [<col>=<v> ...]` — a proto, the registered archetype: named numeric fields over declared columns (`def Marine soldier=1 hp=100`), the registry's first row-oriented named-value construct. `spawn <name>` fills through it: proto value, else the column's `default`, else the type zero. Unique and key columns refuse proto fields (minted, not defaulted).
- `reap <seal|host>` — the `~` reclamation policy, world-level, once: `seal` (the default) reaps at tick seal, `host` hands reclamation to the host. Schema only in anoc; the mask-level meaning of `~` never changes.
- `alias <name> <n-mask>` — a stored mask: a value with a name, distinct from the name alias below.
- `bind <name> <entity|mask|point|num|vec> <values>`. `fn <name> [verbatim BQN body]`.
- `role <keys|id|parent|proto|pos> <col>` — points a system role at a column. Resolution is one path: the declared column, else `reg_find` on the literal role name (entries, then aliases); the declared line is the explicit override whenever plumbing must be pinned.
- `as <word> <name>` — a pure name alias: one hop, no transitivity, outranked by real entries. `ja <word> <name>` fills the same table; the ja spelling documents the Japanese surface at the declaration site.
- `as <word> <col> <value>` — a derived tag: the word names the equality mask over the live column (present ∧ col = value), recomputed at each use, hops included. Num, bool, and sym carriers only; char and vec carriers are rejected at load. Read-only as an effect target — assignment, presence writes, and spawn protos all reject, naming the carrier (`race = :Nord` is the write) — and unpinnable by `--! expect`, which pins storage. The word folds like every name; the value never does.

The case contract holds at load: names fold, values never. Rejections: an entry name or alias source the lexer owns under the fold (`Til` exactly as `til`); two entries whose names fold together; two alias sources likewise; a redeclared `n` or `lattice` (one header — a world validated against two counts has no one-header spelling to dump back). Alias-vs-entry is not rejected — entries outrank aliases at resolution, so that is deterministic shadowing, and the identity-alias worlds (`ja x x`) depend on it.

## Pipeline: anoc

`anoc [--tokens] [--emit] [--run] [--label] [--trace] [--dump <path>] [--save <path>] [--rt <path>] [--registry <path-or-name>] file.ano`

Parse directives → load registry → lex (ASCII or JA) → parse → emit BQN (fixture bindings, then per statement: gather mask, compute deltas against pre-state, scatter; expectations as BQN assertions) → `--run` pipes the program to `bqn` (from PATH; the nix dev shell provides CBQN). Exit status is the differential-test verdict. `--dump <path>` writes the loaded world back to .reg text right after the registry loads (alone it stops there; with `--run`, `--tokens`, or an explicit `--emit` the pipeline continues): dump → load → dump fixpoints byte-identically, and a dumped world passes the same pins as the in-memory one. `--save <path>` (requires `--run`) is the pipe-back: the emitted program gains a serializer that prints the post-state data after the pins hold, one 0x1E-prefixed line per datum — `n`, then per data-carrying entry in declaration order (col/field values, pres bits, rel indexes with -1 the none sentinel, srel fibers; schema never pipes — fns, binds, aliases, roles, and derived tags are load-side, and inv fibers recompute at load) — main.c patches the lines into the loaded Registry and commits `<path>` through the same staged rename. The serializer is flag-gated: without `--save` every emit is byte-identical to before, and on a nonzero child exit nothing is written. `--trace` is the debug observability channel: 0x1F-prefixed diagnostic lines, one control byte below `--label`'s 0x1D exactly as 0x1D sits below the 0x1E world channel — `RELATION <column> <origin> -> <sink> IS DEAD !` per dead link a hop's found-guard drops, `FIBER <column> <origin> IS EMPTY !` per empty-fiber row failure, and `s<N>: <before> rows -> <after> (spawn <Proto>: +k, kill: -j)` per structural statement. Observability, never semantics: post-state is byte-identical with the flag on and off, and without the flag the emit is byte-identical, the same seal `--save` and `--label` carry. Registry names that are not BQN-legal identifiers emit as stable `jp<i>` variables (by registry index; `pres_jp<i>`, `Fn_jp<i>` alongside), the human spelling kept as a comment on the fixture line — so a natively Japanese world emits legal BQN, and conjugate demos stay byte-identical under `--emit`.
