# ano 時 — time, transformations, and the mission register


## The one idea

The world is a value on a tick axis. One tick is one rule barrier plus the queued commands, so the game loop is already the scan: state[t+1] = F(state[t]). Every statement and every standing rule is a piece of F, a pure map from pre-state to effect buffer. ano is a language of transformations over a timeseries of worlds; the host folds F along the axis, and that fold is the game. Determinism is a corollary once the inputs are pinned: the same registry behavior, runtime, tick-0 snapshot, ingested inputs, seed, and statement log give the same trajectory, so a mission is a text file that replays identically anywhere, a saved game is any prefix of the fold, and a test is a predicate asserted at tick t of a replay. The Nix reading is exact: a mission file is a derivation — inputs pin outputs, and anything recomputable from the log never needs storing.

## The timeseries reading (kdb+)

kdb+ partitions tables by date and answers "as of" with `aj`; Anoptic can partition columns by tick. Sealed ticks are immutable, which buys a strong claim: a read against tick t−k cannot intersect the current barrier's write footprint, because the partition it reads is closed. History reads are read-side by construction — the same shape as the generator-subclause argument, and a foundations-grade claim once stated precisely.

Rulings:

- The temporal-read surface is relational, not a new operator. The clock prelude registers `ago(k)`, a stable-keyed functional relation from each current entity to its row at sealed tick t−k, and `window(k)`, a stable-keyed ordered fiber over ticks [t−k,t). `ago(1).Health` is the same entity's last sealed health; `+/ window(60)'.Damage` is damage over its last sixty ticks. Existing dot, tick, fold, scope, and lineage laws do all the work. Japanese uses the ordinary genitive relation, 一刻前の体力, rather than overloading effect-side に.
- A history partition is immutable and readonly. The clock alone appends it at tick seal; scripts may gather and predicate over it but never scatter into it. Functional `ago` is silent when the stable key did not exist at that tick, and `window` simply has a shorter fiber at birth.
- Logical retention is part of the mission lock: a declared horizon h makes every query with k≤h reproducible. Physical storage is a materialized ring plus content-addressed checkpoints and the complete trusted-input/command log; the host may replay missing sealed partitions without changing answers. A query beyond the declared horizon refuses instead of silently shortening. Full history is h=∞, not a different semantics.
- Identity across ticks is therefore extensional data. Stable keys and `ago` answer “the same bandit as last tick”; no predicate result or selection handle survives the barrier.

## The Noita reading (missions in emergent worlds)

The falling-sand world is the friendly case, and the claims here should be tested against it. Materials are fields on one named lattice habitat; per-material behavior is standing rules over explicit neighborhood relations and boundaries; and a quest in such a world is a predicate over emergent state — "every gold deposit in the lake has melted" is address-by-description with no scripting glue, the selection is the objective. Repeated ticks must preserve the field habitat and rank.

Rulings:

- Stage advance and retraction are separate barrier records. A named stage rule advances data; after that tick seals, the schedule records `undef stageN` before installing the next stage's rules. Rules never install or retract rules from inside their shared effect barrier.
- Generated missions stay host-side registered functions, not a generator subclause. Every generator declares a semantic version, closed input signature, seed stream, and deterministic output; the mission lock pins all four. An unseeded or environment-reading callback is not replayable and cannot enter a mission.
- A `.mission` file is a host manifest, not Ano code as data. It pins the schema fingerprint, compiler/runtime semantic version, tick-0 snapshot digest, ordered rule/command bundle digests, tick schedule, root seed plus named seed streams, trusted-input log digests, history horizon, and checkpoint policy. Ano source remains ordinary referenced text. Kore may inspect and diff manifests; publication or hot-swap is one validated barrier transaction, never an expression value.

## The transformation reading (F itself)

F is the merge of the installed rule set under the one rule barrier — the `;` commutation law lifted to the whole program. F is nameable only as a schema-stamped host plan and mission-manifest digest, never as an Ano string or first-class code value. Kore may inspect, diff, validate, and atomically hot-swap such plans at a barrier. Time-travel debugging follows from sealed partitions: the console querying tick t is the distal demonstrative pointed at the past, and the こそあど reading extends — その時 at that time, あの時 back then, the shared-knowledge あ again.

## Next steps

- Implement the clock-prelude `ago(k)` and `window(k)` relationship families with stable-key lineage, horizon refusal, and immutable partition stamps.
- Add the sealed-partition noninterference theorem and keyed as-of/fiber laws to `proofs/foundations.md`.
- Implement and validate the line-oriented `.mission` lock manifest, then add a three-stage replay demo with explicit `undef`, deterministic seed streams, last-tick identity, and window aggregation.
- Add Nihongo temporal genitives and こそあど examples without adding a temporal keyword or reclaiming effect-side に.
