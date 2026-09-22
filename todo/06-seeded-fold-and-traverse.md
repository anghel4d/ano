# 06 — seeded fold and fallible traverse

Status: open, not begun. These are the two surface gaps recorded in `docs/ano-language.md` §12/§14 after `03` landed. They are not part of `03`. Do not start them by silently extending the unseeded left recurrence, A12's no-row reading, or the existing row-drop/refusal path.

The vocabulary is already named. Haskell `foldl`/`foldr` take a seed. `traverse` is a fold whose step can fail. Ano has neither surface. `03` remains complete without them. A ruling is required before grammar.

## Seeded fold

A13 already provides for a possible explicit seeded form. The unseeded surface does not grow one by implication, and an unseeded scan never emits an extra seed row.

The gap: `max/` over empty yields nothing, and a supplied floor would need a seed. Haskell `foldl`/`foldr` take that seed. OCaml `List.fold` does too. Ano's `+/` and `max/` are the unseeded pair.

Options. Keep only the unseeded form and let a registered identity remain the empty answer, which is today's law and keeps A12's no-row reading for identityless heads. Or add an explicit seed slot so `max/` over empty can yield a supplied floor rather than nothing, at the cost of a second empty-case reading beside A12.

A seed is not an implicit identity, not ±∞, and not a none marker. If adopted, the unseeded glyph forms keep their present empty law, the seeded form is a distinct spelling, and an unseeded scan still never emits an extra seed row.

## Fallible fold

Haskell `traverse` is a fold where each step can fail, collecting effects. `Maybe` or `Either` short-circuits. `Validation` accumulates. Monadic bind sequences, so failure stops at the first step. Accumulation of errors is deliberately not a monad, which is why the two readings want two names.

Ano has no surface for this yet. Options. Keep failure as the existing row-drop and refusal path, which already aborts a statement. Or admit a form that returns a column of successes or a collected refusal, which needs an effect carrier the language does not currently have.

Do not encode short-circuit as a new monad on columns. Do not smuggle an option carrier in through A12. The existing statement-level refusal and identityless row-drop stay the failure path until a distinct spelling exists.
