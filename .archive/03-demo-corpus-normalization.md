# 03 — demo corpus normalization: renumber, split, rename, re-annotate

Four rulings (author, 2026-07-11) executed as one pass, because they all touch every demo file once.


## Substep 1: 1/08 — faction → leader

1/08 has a column named "leader". It doesn't make sense. It should be called leader, as it points to a nord and a person archetype, not to a faction table.

Rename rel faction to rel leader everywhere unless it genuinely points to a different kind of thing. Right now in 1/08 it makes no sense because following faction is a pointer to an index into an individual soldier. That is nonsense. Do not change the example in ano-language.md. The author's reasoning, recorded: the spec's faction is a genuinely different thing, a faction archetype distinct from soldier with its own columns and traits, a separate KIND of entity, while the demo's "faction" is a trait of 1 that is literally an index to a Nord. (`rel faction …` and `ja 派閥 faction`). The ja word I guess could be 頭領 (tōryō). ano-language.md:154 stays as written.

Commit after this step. It's just a simple rename task. 


## Substep 2: 1/s05 split

Currently, this demo is way too dense and tests like 4 completely different things at the same time. Copy out of it and split into separate exampls the independent things that are actually happening. This is supposed to be educational and inforamtive.

Keep the current 1/s05 as a wrap-up example. 

Commit after this step.


## Substep 3: The re-annotation pass

Add teaching comments in the author's register across the corpus. See docs/the-sky.md and /tersify for tone. Every demo should begin with a paragraph of comments explaining WHAT exactly the world being set up is, and introducing each of the operands simply. Do not overspecify HOW, for that much is visible from simply running the demos and inspecting the code. 

The style of K&R's The C Programming Language (2nd Ed) remains the greatest of all time at the pedagogical tone, typography, and exposition I expect.

For example, in the demo where the barrier `;` is introduced, we may say:
"The barrier operator `;` is not `then`, it's `and at the same time`. The operations in it happen simultaneously. See as per the BQN: {bqn} or the APL {APL}. In Haskell this might look like {Haskell}. The point of it is to {idea}."

- 14-reductions: an @ line in the same voice. @ is not "where", it is "within": evaluate this inside that scope, the same one meaning whether it scopes a mask or closes a fold. And the identities: an empty scope answers with the fold's identity. The sum of nothing is 0, the product of nothing is 1, an empty party is vacuously all-alive.
- 15-named-reducer: "threat is a dyadic max, folded pairwise over the scope. {illustrating plainly with two columns of values and a sliding box}"
- 16-scans: the associativity refusal, 3-4 lines of formal explanation, e.g.: `-- Why not -\ or //? Impossible. A fold erases the parenthesization: f/ is well-defined only when (a f b) f c = a f (b f c), the semigroup law. Subtraction and division fail it: (a-b)-c ≠ a-(b-c). APL's -/ answers by redefining the operation (alternating sum). This would be difficult to justify in ano (as it exists right now).` finishing with: `-- / and - are not possible to fold-reduce because they are not associative.`
- 23-expand-alias: what "identical keys" means. Why ex22's `spawn Minion * Count` and ex23's `|> expand Count , spawn Minion` must mint the same keys row for row.
- 24-reshape-positions-b: header subtitle plus original comments. Cover the behaviour of `_`.

Preserve the author's own comments verbatim, for instance s10-pre-state's "Not actually bound to anything lol.".

Also riding this pass: the seven migrated play copies from the 2026-07-10 corpus restore (`.kore/play/` copies of 1-selection/{01,03,06,08}, 2-effects/{12,s10a,s10b}) hold the author's hand edits, some possibly deliberate fixes. Ruled (author, 2026-07-11): archive all seven to a safe location that is neither in demos nor in the reset dir, such as kore/saved. That makes them `>reset`-proof; the per-file adopt-or-discard review then happens from the archive and no longer gates the reset.

Optional at execution, author already approved the idiom: land the soul-gated barrier demo (`nord & dead & soul , soul = 0 ; spawn Ghost`, soul consumed in the same barrier, minted ghosts get the type-zero soul, linear and terminating) as a loop-safe interactive variant beside the barrier-granularity pair. The original stays: its exponential growth under repeated ticks is the pinned point, not a bug.

Commit after this.


## Substep 4: demo renumberings.global ascending numbering

Every demo gets a numerically ascending number prefixing its name, absolute across the whole corpus. No more off-standard variations: the sNN/cN/nN/wN/tN/rN prefixes all fold into the one sequence. Keep the provenance in each file's header comment, not the filename.

What this means is extremely simple. 

We create a counter n. n starts at 1. 

For each folder in demos/, eg 01-selection, 02-effects, and so on, we enter the folder. Then, for each .ano file in the order that they appear in the folder, we change the number prepending its name to n, then increment n by one. Akin to foreach(demo) { XX-filename.ano, XX-filename-nihongo.ano, XX-filename.bqn <- XX = n}.

Then, we rename and reorder all of the registry entries to match, obviously. So they'll be from 1 to n, in the same order that they appear in demos/.


## Invariants

- Both suites green after every rename batch. The corpus runnable at every intermediate commit.
- Quads stay paired (.ano / .bqn / -nihongo / .reg). No demo loses an expect. Comment additions never change program text or pinned values.
- Author's voice preserved. Tighten, don't rewrite.
