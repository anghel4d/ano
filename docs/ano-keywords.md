# Tokens and forms

The executable tables are in [steel/src/lex.rs](../steel/src/lex.rs); grammar is in [parse.rs](../steel/src/parse.rs). Name resolution and capability checks happen later. This page does not reserve additional words.

## ASCII keywords

`kwkind` recognizes these eighteen words:

```text
def undef spawn at to via along order by take desc top grade fold scan cross expand til
```

| Words | Use |
|---|---|
| `def`, `undef` | Definition and named-rule retraction |
| `spawn`, `at` | Structural creation and placement expression |
| `to` | Legacy coordinate/shape form |
| `via`, `along` | Pipeline input or traversal view |
| `order by`, `desc`, `take` | Ordered selection |
| `top`, `grade` | Prefix ordering/selection forms |
| `fold`, `scan` | Long operation heads |
| `cross` | Materialized outer product |
| `expand` | Replication pipeline |
| `til` | Finite index generation |

Lexer keywords, loader-reserved spellings, and emitter built-ins are different sets. `lex_reserved` additionally checks Japanese tokens/numerals and reducer spellings such as `max`, `min`, and `avg`.

`eval` is a statement-position parse-time literal splice. `rank`, `index`, `prev`, `row`, and legacy neighbor forms have context-sensitive handling. They are not extra entries in `kwkind`. A name's behavior must be checked at its actual resolution site.

## Punctuation

| Form | Use |
|---|---|
| `,` | Selection/effect hinge; also tuple/call structure |
| `=>` | Standing-rule hinge |
| `;` | Effects sharing one barrier |
| `\|>` | Pipeline |
| `&`, `\|`, `!` | Mask operations; Greater/Lesser overload by carrier |
| `=`, `==`, `!=`, `<`, `<=`, `>`, `>=` | Contextual assignment or comparison |
| `+=`, `-=`, `*=`, `/=` | Value updates |
| `+`, `-`, `*`, `/`, `%` | Arithmetic or structural forms by context |
| `@` | Scope; plain mask scope lowers to conjunction |
| `.`, `'` | Functional hop/projection and set-fiber hop |
| `(`, `)` | Group, call, tuple |
| `[`, `]`, `<-` | Comprehension and generators |
| `~` | Despawn/continuation |
| `_` | Wildcard or inferred axis |
| `^name` | One dynamic-alias token |
| `:Name` | Symbol literal |
| `f/`, `f\` | Fused fold/scan head |

A bare caret refuses. A fused head cannot contain interior whitespace. Division reduction uses `fold(/)`, not `//`.

`--` begins a source comment. `--!` directives belong to the fixture runner, not the language grammar.

## Numbers and names

The ASCII reader admits numeric literals and counter forms such as `3mo`. The Japanese reader additionally recognizes kanji numerals and its counter table. A counter spelling is not a nominal affine-frame type.

Registry nouns can use UTF-8. Name lookup and symbol payload equality have different rules: source name matching can fold ASCII case, while symbol values retain their spelling.

## Registry vocabulary

Registry rows are parsed separately in [registry.rs](../steel/src/registry.rs):

```text
n lattice col field unique pres default range rel srel inv
bind alias as ja role def fn array service enum ctor reap
```

The two-name `as` form is a spelling alias; the other admitted `as` form constructs a derived tag. There is no separate `tag` row keyword. `ja` adds a Japanese spelling alias.

Legacy stored-column carriers include `bool`, `nat`, `int`, `num`, `sym`, and `char`, with `vec` as a compatibility representation. Typed descriptors use `mask`, `nat`, `int`, `num`, `sym`, `char`, `entity`, and earlier enum/constructor names. `unit` has a signature role. These are not interchangeable spelling sets.

[ano-registry.md](ano-registry.md) documents the declaration contracts. [ano_nihongo.md](ano_nihongo.md) documents the implemented Japanese reader.
