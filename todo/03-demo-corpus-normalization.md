# 03 — demo corpus normalization: renumber, split, rename, re-annotate

Four rulings (author, 2026-07-11) executed as one pass, because they all touch every demo file once.

## Ruling 0 — global ascending numbering

Every demo gets a numerically ascending number prefixing its name, absolute across the whole corpus. No more off-standard variations: the sNN/cN/nN/wN/tN/rN prefixes all fold into the one sequence. Folders remain convenience grouping only. The old prefixes were provenance markers (sNN pinned spec §NN or a decision record, letter prefixes were series-local schemes). Keep the provenance in each file's header comment, not the filename.

Mechanics to sweep: each demo's .ano, .bqn, -nihongo.ano, and registries/<stem>.reg move together as a quad. Also: `--! registry` lines, demos.md and the series READMEs, ISSUES.md live references, spec mentions of demo stems (e.g. "demos 11-noita w3-b/w3-c", s16-keygen, t1-reverse). check.sh and src/check-ano.sh are glob-driven but verify. PATCHES.md is a historical ledger, so old names stay. kore's play-copy tags key off realpath, so renames orphan `.kore/play/` dirs. Acceptable (scratch by design); note it in the PATCHES entry at landing.

## Ruling 1/08 — faction → leader

Rename faction to leader EVERYWHERE except ano-language.md. The author's reasoning, recorded: the spec's faction is a genuinely different thing, a faction archetype distinct from soldier with its own columns and traits, a separate KIND of entity, while the demo's "faction" is a trait of 1 that is literally an index to a Nord. Files: 08-relationship-hops.{ano,bqn}, -nihongo.ano, registries/08-relationship-hops.reg (`rel faction …` and `ja 派閥 faction`). The ja word is ruled (author, 2026-07-11): 頭領 (tōryō), as proposed. ano-language.md:154 stays as written.

## Ruling 1/s05 — split

s05-set-hop packs four unrelated .bqn sub-worlds into one 23-row world gated by archetype masks (rows 0-7 frenzy, 8-13 plots, 14-18 animals, 19-22 pens). Split into small single-world demos as part of the renumbering: (a) the frenzy image + in-degree pair (idempotent scatter; `Hits += #/ attackers'`), (b) the plot neighbor quantifiers (any / count, pre-state ring spread), (c) the pen inverse-fiber γ (`&/ livestock'.Healthy` with the vacuous-truth empty pen; `Headcount = #/ livestock'`). Each gets its own registry slice, and expects are re-derived per world. The .bqn witness stays one file or splits with them: implementer's call, suites green either way.

## The re-annotation pass

Add teaching comments in the author's register across the corpus. The style is ruled by example. This remark, verbatim, belongs in the pre-state demo: "; is not 'then', it's 'and', in the same breath." More like it, required:

- s10-pre-state: the line above, verbatim.
- 14-reductions: an @ line in the same voice. @ is not "where", it is "within": evaluate this inside that scope, the same one meaning whether it scopes a mask or closes a fold. And the identities: an empty scope answers with the fold's identity. The sum of nothing is 0, the product of nothing is 1, an empty party is vacuously all-alive.
- 15-named-reducer: "threat is a dyadic max, folded pairwise over the scope."
- 16-scans: the associativity refusal, 3-4 lines of formal explanation, e.g.: `-- Why not -\ or //? Impossible. A fold erases the parenthesization: f/ is well-defined only when (a f b) f c = a f (b f c), the semigroup law. Subtraction and division fail it: (a-b)-c ≠ a-(b-c). APL's -/ answers by redefining the operation (alternating sum); ano refuses the pun.` finishing with: `-- / and - are not possible to fold-reduce because they are not associative.`
- 23-expand-alias: what "identical keys" pins. ex22's `spawn Minion * Count` and ex23's `|> expand Count , spawn Minion` must mint the same keys row for row, the equivalence witness that the two spellings are one semantics, and the determinism-of-minting law underneath (ordered by statement, selected slot, replicate index).
- 24-reshape-positions-b: header subtitle plus original comments. Cover the `_` free axis (infer 64/4 = 16) and that the demo pins prefix-extension: a longer prefix of the same program lands on the .bqn's intermediate state.
- s19 fib pair: the across-ticks framing. The tick loop is itself the scan, state[t+1] = F(state[t]), and the stencil twins iterate exactly that. Only the within-statement recurrence is host-fn territory.

Preserve the author's own comments verbatim, including s10-pre-state's "Not actually bound to anything lol.", ruled kept.

Also riding this pass: the seven migrated play copies from the 2026-07-10 corpus restore (`.kore/play/` copies of 1-selection/{01,03,06,08}, 2-effects/{12,s10a,s10b}) hold the author's hand edits, some possibly deliberate fixes. Ruled (author, 2026-07-11): archive all seven to a safe location that is neither in demos nor in the reset dir, such as kore/saved. That makes them `>reset`-proof; the per-file adopt-or-discard review then happens from the archive and no longer gates the reset.

Optional at execution, author already approved the idiom: land the soul-gated barrier demo (`nord & dead & soul , soul = 0 ; spawn Ghost`, soul consumed in the same barrier, minted ghosts get the type-zero soul, linear and terminating) as a loop-safe interactive variant beside the barrier-granularity pair. The original stays: its exponential growth under repeated ticks is the pinned point, not a bug.

## kore rail subtitles

While in here: the rail shows each demo's first header line as a subtitle, or the renamed stems carry the meaning. Either satisfies the complaint that s10-barrier-granularity-a tells a user nothing.

## Invariants

- Both suites green after every rename batch. The corpus runnable at every intermediate commit.
- Quads stay paired (.ano / .bqn / -nihongo / .reg). No demo loses an expect. Comment additions never change program text or pinned values.
- Author's voice preserved. Tighten, don't rewrite.
