# ano 時 — time, transformations, and the mission register

Preliminary working note, 2026-07-02. The design conversation behind it: quests are data, the Anoptic engine runs a monotonic tick counter at a fixed rate, determinism is load-bearing, a declarative language like Nix but for gaming. The settled parts already landed in the spec — §11 campaign logic, the Binding types paragraph, the Recurrences/Identity/Rule-retraction open questions. This note is the workspace for what is not settled. Where it touches a spec open question it points in; the spec entry stays the record.

## The one idea

The world is a value on a tick axis. One tick is one rule barrier plus the queued commands, so the game loop is already the scan: state[t+1] = F(state[t]). Every statement and every standing rule is a piece of F, a pure map from pre-state to effect buffer. ano is a language of transformations over a timeseries of worlds; the host folds F along the axis, and that fold is the game. Determinism is a corollary: the same tick-0 snapshot and the same statement log give the same trajectory, so a mission is a text file that replays identically anywhere, a saved game is any prefix of the fold, and a test is a predicate asserted at tick t of a replay. The Nix reading is exact: a mission file is a derivation — inputs pin outputs, and anything recomputable from the log never needs storing.

## The timeseries reading (kdb+)

kdb+ partitions tables by date and answers "as of" with `aj`; Anoptic can partition columns by tick. Sealed ticks are immutable, which buys a strong claim: a read against tick t−k cannot intersect the current barrier's write footprint, because the partition it reads is closed. History reads are read-side by construction — the same shape as the generator-subclause argument, and a foundations-grade claim once stated precisely.

Ground to work out:

- The temporal-read surface. Something must mark "as of t−1" on a predicate or a gather. Options: a tick scope on `@` (`@ t-1`, but `@` already carries two meanings), a dedicated particle, or explicit keying off the axis. The Japanese tiebreaker cuts here: time in Japanese is case-marked — に on time points, から/まで on ranges, た for the past — so the nihongo answer is that history is a case role, not a function call. に is already claimed by the effect frame; that collision is real and needs writing out.
- Window reads. "Damage taken over the last 60 ticks" is q's `wj`: a fold along the tick axis per entity. The axis arrives with intrinsic order, so the Tier-2 no-canonical-previous obstruction dissolves along t exactly the way it does along space — which suggests window folds are §12 folds with the axis as one more orderable domain, not a new form. To be worked, not assumed.
- Retention. Full history, ring buffer of k ticks, or keyframe plus replay-from-log. Replay makes history recomputable — the Nix insight again.
- The host binding. History columns are the readonly data-store's natural cargo: written by the clock, predicated on by scripts, never a scatter target.
- Identity via the axis. The as-of join answer to "the same bandit as last tick" is recorded in the spec's Identity entry; if the temporal surface exists, that entry's third option gets its syntax for free.

## The Noita reading (missions in emergent worlds)

The falling-sand world is the friendly case, and the claims here should be tested against it. Materials are Tier 1 lattice columns; per-material behavior is standing rules over neighborhoods (γ folds and stencils, one ring per tick); and a quest in such a world is a predicate over emergent state — "every gold deposit in the lake has melted" is address-by-description with no scripting glue, the selection is the objective.

Ground to work out:

- Stage advance and retraction. A stage rule must withdraw itself on firing; the retraction surface is the spec's open question and the mission register is where it earns its answer. What the register owes: install, withdraw, and a stance on rules that install rules — which is the staging/quotation question, not a new one.
- Generated missions. Host callbacks (`fib`, noise, spatial queries) key content off indexes today; the generator subclause would bring bounded corecursion in-calculus. Both live in the Recurrences entry. The mission case sharpens the requirement: generation must be deterministic per seed or replay dies.
- The mission file format. Registry declarations, defs, standing rules, a schedule, a seed. Beyond that it owes a tick-0 snapshot reference and version pinning — the lockfile of a mission.

## The transformation reading (F itself)

F is the merge of the installed rule set under the one rule barrier — the `;` commutation law lifted to the whole program. Two questions fall out. Whether F is nameable — a mission as a value the console can inspect, diff, and hot-swap — is the staging/quotation question wearing its most useful clothes. And time-travel debugging comes free once partitions exist: the console querying tick t is the distal demonstrative pointed at the past, and the こそあど reading extends — その時 at that time, あの時 back then, the shared-knowledge あ again. The nihongo doc gets a temporal-deixis section when this firms up.

## Next steps

- A demos/mission/ cluster: a three-stage scenario as a tick-loop fold in BQN — stage rules, retraction on advance, replay determinism asserted (same log, same trajectory), an as-of join answering last-tick identity.
- State the sealed-partition claim (history reads never meet the barrier) precisely enough for proofs/foundations.md.
- Pick a provisional temporal-read surface so examples can be written; mark it provisional.
- Nihongo: temporal case marking (に/から/まで/た) and こそあど over the time axis.
