# anoc surface grammar

The implementation contract for the C transpiler in this directory. Derived from ano-language.md (grammar appendix, the 14-level precedence table) and ano-examples.md (ex1-49). This file adds nothing to the language; where it and ano-language.md disagree, ano-language.md wins.

## Lexical

- Source is strict UTF-8, validated whole before either mode lexes: overlongs, encoded surrogates, and codepoints past U+10FFFF are a lex error ("malformed UTF-8"). A well-formed but unexpected character reports by codepoint (`unknown character U+00E9`).
- Comments: `--` to end of line. `--!` is a harness directive, not a comment (below).
- Names: `[A-Za-z][A-Za-z0-9_]*`. Case is preserved; resolution against the registry is case-insensitive on the first letter only (surface `Gold`, registry column `gold`). A name is component, callable, binding, or def — sentence position decides (spec, flat namespace).
- Alias sigil: `^` immediately followed by a name, no whitespace, lexes as one T_ALIAS token. A bare `^` is a lex error. `@` is always the scope operator T_AT: `Merchant @ Whiterun` and `Adj@row` are both `… AT …`, spacing irrelevant.
- Symbols: `:` immediately followed by a name: T_SYM. A backtick is a lex error.
- Numbers: integers and decimals (`60`, `0.5`, `1.05`). A number immediately followed by a name with no whitespace is a counter-typed numeral T_COUNTER (`3mo`: value 3, unit `mo`).
- Strings: `"..."`, no escapes needed for the demos: T_STR.
- Wildcard: `_` alone: T_WILD (`to 4 _`, `(Nord, TwoHanded _)`).
- Fold tokens: `+/ */ &/ |/ #/` and `max/ min/ avg/` lex as one T_FOLD with an op payload; likewise a name directly followed by `/` where the name is a registered reducer. Scan tokens: `+\ *\ max\` lex as T_SCANOP with op payload. No whitespace inside either.
- `/` between value expressions is division; the fold reading requires the `f/` form at expression head (term position). Same for `\`.
- Iota: `↕` (UTF-8) is T_IOTA.
- Operators: `, => ; |> & | ! == != < <= > >= = += -= *= /= + - * / % @ . ' ( ) [ ] <- ~`.
- The tick `'` is postfix on a name: `targets'`, `neighbors'`. Lexes as T_TICK following T_NAME.
- Keywords (closed set, the lexer owns them): `def spawn at to via along order by take desc top grade reduce scan scan2 cross expand`. Everything else, `index rank prev neighbor char x y row` included, is a name resolved by the registry or the prelude.

## Directives

`--! key rest-of-line`, harness-owned, one per line:

- `--! registry <path-or-name>` — load the world. A relative path resolves against the source file (`../registries/x.reg`); a bare name loads `<name>.reg` from beside it; an absolute path is used verbatim.
- `--! expect <col> = v v v ...` — post-state assertion on a numeric/bool column, world order. For symbol columns values are bare names (`Hostile`). For a lattice field, row-major.
- `--! expect-n <k>` — post-state world row count (after spawn/despawn).
- `--! out <v v v ...>` — the next query statement's printed value equals this (scalar or vector).
- `--! ja` — the statement lines of this file are in the Japanese surface; lex with the JA skin.

## Statements

A program is a sequence of lines. Blank lines and comments are skipped. Each surviving line is one statement, one barrier — with one exception: a rule install is not a barrier of its own; rules fire at the harness's clock ticks, all installed rules in one shared barrier per tick (the def-rule entry below).

- `def <name> = <selection-or-column-expr>` — names a predicate or derived column; inlined at use (N_DEF).
- `def <name> = <selection> => <effects>` — a standing rule (installed; the clock fires it). anoc has no clock, so it pretends one tick at the end of each unbroken run of def lines (plain defs don't end a run; a performed statement or end of file does). Each tick fires EVERY rule installed so far — installs persist, so a rule installed early fires again at a later tick, exactly as it would on the engine's next beat (n5-c) — in one shared barrier: every rule's mask gathers the one pre-state, every effect stages into one commit set, the set scatters once — the §11 shared rule barrier. Same-column writes across a tick's rules go through the §10 merge laws, widened by the guard-complement clause: `!X` in one rule's top-level `&`-chain and `X` in another's certify the two masks row-disjoint over the shared pre-state, licensing any pair of write families short of a verb dispatch (bloom/wither on planted, c3-b); a pair with neither is rejected. All rules in a tick must select in one frame (an anoc restriction, recorded in ano-limitations.md).
- `eval "<statement>"` — the quotation splice: the literal string is re-lexed and its single statement parsed in place, at compile time, so the spliced footprint stays visible to every static check. One statement per quotation; the string must be a literal (dynamic strings are interpreter territory, not this). `eval` is a contextual statement head, not a lexer keyword — the keyword set above is unchanged.
- `<selection> , <effects>` — the command form. The first top-level `,` (not inside parens/brackets) is the hinge.
- `<selection> => <effects>` — anonymous standing rule.
- `, <effects>` — leading-comma continuation: new statement, new barrier, over the antecedent's saved mask.
- `~` alone — despawn the saved mask (continuation).
- `<effects>` with no hinge and an effect-verb head (`spawn`, `+Name`, `-Name`, registered effect verb) — elided subject: the saved mask when one exists, else the host default `^cursor`.
- anything else — a query statement: evaluate, print (checked by `--! out`).

## Precedence (loosest to tightest, from the spec appendix)

1. `,` `=>` hinge | 2. `;` | 3. `|>` | 4. effect verbs and assignment (`= += -= *= /=`, `+Comp -Comp ~ spawn`, `at`, `to`, `*` in `spawn X * n`) | 5. `|` | 6. `&` | 7. `!` prefix | 8. `== != < <= > >= =`(selection position) | 9. fold/scan prefixes, `reduce(f)`, `scan(f)…along`, `grade`, `top k` | 10. `+ -` | 11. `* / %` | 12. `@` scope | 13. `.` and `'` | 14. atoms.

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

Token-level, space-separated surface per ano_nihongo.md and ex40-49. The JA lexer maps each token: registry names via their registered ja aliases (`北`→Nord), particles to operators (`と`→`&`, `の`→`.`, `で`→`@` scope, `より`→`>` postfix, `、`→hinge `,`, `に`→TGT, `は`→hinge (topic), `が`→hinge), effect verbs (`たす`→`+=` postfix, `にする`→`=` postfix, ...), kanji and fullwidth numerals (`六十`→60, `千`→1000, `九千九百九十九`→9999, `一・〇五`→1.05, `１２３`→123, `１・０５`→1.05, `三ヶ月`→counter 3mo). Normalization re-roots each postfix operator immediately before its operand and drops the fused TGT (ex40's permutation), yielding the ASCII token stream; parsing proceeds identically. `--tokens` prints the normalized stream for the ex40 equivalence check.

## Pipeline: anoc

`anoc [--tokens] [--emit] [--run] [--rt <path>] [--registry <path-or-name>] file.ano`

Parse directives → load registry → lex (ASCII or JA) → parse → emit BQN (fixture bindings, then per statement: gather mask, compute deltas against pre-state, scatter; expectations as BQN assertions) → `--run` pipes the program to `bqn` (from PATH; the nix dev shell provides CBQN). Exit status is the differential-test verdict.
