# 14 — the registry border: two languages, Σ-immutability, and the placement seam

Surfaced 2026-07-19 in a late-night design session, audited against this branch the same night. Most of the session restated what commits d20bccf/d554bbe/2482c06 already formalized in `docs/spatialmaths.md`, `proofs/foundations.md`, and the Lean kernel; this file carries only the residue that is genuinely new, with the restated ground listed first so no agent re-derives it. The author's endorsement in-session ("that is the language") makes the two-language ruling binding; the sub-questions under each item are open and are never to be resolved silently.

## Already on disk — do not re-derive

- The central type. `proofs/foundations.md:70` — "A performed statement is consequently a partial state transformer `World Σ → Result (World Σ × Output)`" — machine-checked as `perform_ok_wellFormed` / `ticks_wellFormed`; `docs/spatialmaths.md` §26 gives the same `Step` type for repeated ticks.
- Population changes stay inside Σ. `proofs/foundations.md:72` — "Spawn and despawn do not contradict schema closure. They change the current live entity set inside `World Σ`, not `Σ`" — proven as `World.spawn_fixed`, `World.spawn_field`, `World.spawn_wellFormed`.
- Placement transport. `docs/spatialmaths.md:359` names the planet-local-to-system composition law and states "changing or transporting a placement leaves every field `D_H → V` and the habitat `D_H` unchanged."
- Erasure only after legality. `todo/13-spatial-formalization.md:262` — a backend may fuse or erase a lineage vector only after the planner has consumed its typed provenance.
- The backend agreement obligation. `todo/13-spatial-formalization.md:396` — CBQN may remain the initial backend, Steel must reject an invalid plan before generating CBQN, and tests compare backend output with the typed Ano semantics; `proofs/foundations.md` obligation 10 and `proofs/lean.md`'s pending list carry the dense-array refinement. The session's ⟦typed plan⟧_semantic = ⟦Steel-emitted BQN⟧_CBQN is this standing obligation in bracket notation, not a new target.
- The two closed vocabularies, descriptively. `docs/ano-keywords.md:3` — the language owns eighteen reserved words, the registry owns a separate directive set, and the two meet where a registry schema becomes the nouns of a sentence.

## The ruling (2026-07-19, binding)

Two languages sharing resolved names and mathematical meanings, never grammar. The registry file (today's `.reg`, prospectively `.anoreg` — TODO item 3 carries the name) is a purely declarative schema language: it constructs Σ, the universe in which the comma has meaning. `.ano` is the sentence language, typed `Ano_Σ : World Σ → Result (World Σ × Output)`. No registry declaration syntax is ever admitted into `.ano`; the comma remains the only border in a sentence, and schema construction lives outside the sentence entirely. Steel keeps intentionally separate ASTs — registry declarations and statements — meeting only at elaboration against the validated registry.

Σ is immutable during ordinary execution. The lawful mutations under a fixed Σ are exactly three: registered field values per their declared effect permissions; entity populations via explicit structural effects; placement and transform values while their registered types stay fixed (a planet moves without touching the schema). The fourth kind — registry replacement, Σ → Σ′ — is a host/Steel/Kore operation at a world barrier, never a right-of-comma effect. The first two arms are already proven (see above); the third and fourth are the new work, items 2 and 1 below.

## Item 1 — the Σ → Σ′ migration protocol (new, unfilled)

Nothing on disk covers schema replacement; "migration" in `todo/13-spatial-formalization.md` means the Steel/Kore code-porting sequence, not this. The protocol as ruled: parse and validate the new registry; establish the migration Σ ⇝ Σ′; migrate or refuse existing data; invalidate affected compiled plans; re-emit against Σ′; resume at a world barrier. A failed migration leaves the Σ-world standing — the unchanged-world refusal lifted to the schema level.

Work items:

- Record the ruling in `ano-language.md` — the taxonomy and the barrier requirement as rulings, the protocol's concrete spelling under "Open Questions, Next Steps".
- Decide where the migration verbs live (kore commands, steel flags — never `.ano` syntax).

## Item 2 — placements as world values: the seam with the Lean (new, and it cuts against the current model)

The ruling's third arm says placement values may change under a fixed Σ — `Planet & Orbit , planetToSystem = orbitAt Time`, the type staying `PlanetLocal → System3`. The kernel currently models the opposite side of the line: `proofs/Ano/SpatialRegistry.lean:37` — "A spatial schema keeps all frame and spatial authority outside mutable world state" — and `frameMap`/`offsetMap` are static functions of the schema structure. `docs/spatialmaths.md` §14 proves the transport laws timelessly; `SpatialWorld.lean`'s live resolvers rebuild sealed requests against the world supplied at each tick, but the maps themselves never move. The ruling and the kernel currently assert opposite sides; reconciling them is this item.

Open sub-questions (surface at execution, never resolve silently):

- Which placements are world-state? A declared-mutable rider on the placement declaration, defaulting to static, or a separate registered-transform kind?
- Do sealed lineage and interpolation tokens survive a placement write? A CSR interpolation built against one placement is stale after the write — recompute at the barrier, or refuse the combination?
- Is a plan compiled against a placement value invalidated by a placement write, or are plans placement-value-independent by construction (the plan references the token, the token dereferences at tick time)?

Work items:

- Either extend the Lean with a mutable-placement world field behind a declared permission and re-prove the transport laws per tick, or rule placements static and route moving bodies through ordinary registered columns. Both are coherent; the tradeoff goes to the author.
- Whichever side wins, make `spatialmaths.md` §14, `SpatialRegistry.lean`, and the eventual loader say the same thing.

## Item 3 — the adjective-noun registry surface (new spelling, gated)

The session sketched a declaration surface: `euclidean3 f32 frame World`, `product2 rowmajor lattice Ground`, `Ground x f32 coordinate col GroundX`, `entity World Position x f32 position col PosX`. No file carries any of this. Steel's `.reg` today has `lattice <w> <h>` (`steel/src/registry.rs:269`) and `bind … point` as a frame origin (`docs/ano-keywords.md:726`). `todo/13-spatial-formalization.md:196` fixes the law any spelling must obey: injectivity, equivalence, isometry, dimension, and numeric refinement are separate flags backed by separate witnesses, never consequences of a three-number representation — and the sketch violates this as written, since `euclidean3` bundles dimension and metric into one adjective.

Work items:

- Design the declaration surface against todo/13's certificate layering, splitting the bundled adjectives into their witness flags.
- Settle the file-name question alongside TODO item 3's registry taxonomy (`.reg` vs `.anoreg`; `docs/INTERACTIVE.md:36` already lists both as data-at-rest).
- Do not build without the author's explicit go — the same gate TODO item 3 already states.

## Invariants after

- The comma remains the only border in `.ano`; no registry declaration syntax appears on either side of it.
- Every Σ-mutation path is one of the four taxonomy arms; anything else refuses with the world unchanged.
- The Lean, the spec, and the loader agree on which side of the schema/world line placements live; no document asserts one side while the kernel proves the other.
- Docs must not promise unbuilt surface (task 04's law): the adjective-noun sketch stays in this file until ruled and built.
