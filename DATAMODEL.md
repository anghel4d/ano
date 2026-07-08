# DATAMODEL — The Ladder.

The author's second notebook page (2026-07-08, transcribed in INTERACTIVE.md) restates the Tiers model along a new axis — dimensionality — and bifurcates the Race question into two representations declared equally legal. This file is the plan that coheres that page with the shipped semantics. The verdict up front: it is a restatement, not an upheaval. The audit below finds every rung and every arrow of the ladder already witnessed in the corpus and already designed in ano-ecs.md; exactly one registry spelling is missing, and it lands inside a patch that is already planned (REGFIX.md item 3).

## The ladder

spaces ⇄ matrices → arrays ⇄ columns ⇄ relations → tags. Dimensionality: nd → n×m → dense 1d → dense 1d over I → sparse 1d → 0d points. Each arrow is a mathematical operation, and each already has a witness:

- spaces ⇄ matrices, change of basis. A frame is (o, S), origin plus basis — `pos = o + S·k` (ano-ecs.md §9's `ano_frame`, foundations §2.4); transforms are matrices acting on vec columns; the anchored frame fills o from the world. The one open item here was already on the books before the diagram named its home: the world-to-chunk (o, S) conversion has no surface (ISSUES.md) — it is exactly a missing change-of-basis form.
- matrices → arrays, one col. Projection. Lattice fields are rank-2 arrays flattened row-major (anoc: `field` over latW·latH; the store: ano_lat's row-major directory), and compound pos decomposes SoA into sibling x/y arrays sharing one presence bitmap (ano-ecs.md §3).
- arrays ⇄ columns. An array of length n bound to the slot space is a column; binding to I is the iso, projection forgets it. Registry-resident arrays that bind no index are the taxonomy's arrays rung (REGFIX.md item 6, named-not-started).
- columns ⇄ relations. Rightward: a column whose values are keys stands in relation position — the resolution rule the wand suite added. Leftward: a relation is stored as a column of row indexes — anoc's `rel` lines, the store's T_ID columns ("relationship columns store IDs as data", ano-ecs.md §2), the diagram's Master with `/` for none (sentinel, NIL). The two directions are graph-of-function and function-of-graph.
- relations → tags, membership collapse. Domain, image, and fiber-nonempty are masks: the set hop's image is bits OR'd into a mask and cannot express multiplicity (ano-ecs.md §4); srel fibers filter to masks (`bind' & mark`, w3-c).
- columns → tags, the point predicate. σ by equality is δ_v ∘ col — for sym columns an integer compare over interned symbols (`Faction == :Bandit`, ano-ecs.md §3).

Tier 3 sits at the ladder's 0d end too, from the other side: handles are points of an opaque V, values the engine may only move (naturality in V). The tiers and the ladder classify the same objects on orthogonal axes — the tiers grade by invariance, what a write must commute with (Erlangen; tier = the (operation, view) pair), the ladder grades by dimensionality, how much index and value structure a datum carries (rung = shape). The restatement is exact, and nothing in the implementation moves because of it.

## Denotation over representation

The law the page states, generalized: a noun denotes a mathematical object — mask, column, relation, scalar, space — and the registry chooses its representation. The language's contract is with the denotation only. This is already doctrine twice over in ano-ecs.md: a relationship is one span with three physical forms and "the surface needs no syntax per form — the registry's declaration picks the kernel" (§7), and the tier view functor puts two meanings on one page of bytes (§9). The ladder makes the doctrine total: any wiring that composes rung objects along ladder arrows is legal. The legal wirings are broad because legality is compositional, not enumerated — ano is mathematically defined, not rigidly engineered.

## The tag, three ways

A tag is a mask — the subobject, the working currency every selection evaluates into (ano-ecs.md §4). The characteristic function χ_S and the subset S are two sides of one iso, which is why "sparse or dense" is a storage choice with no semantic content. Three legal namings, two of them shipped:

- Option A, stored. `col nord bool 1 0 1 1 0` — the corpus norm today; in the store, T_TAG, a presence bitmap with no value bytes ("a valueless tag is its bitmap, 1 bit/slot", ano-ecs.md §3). The spec ground is §1's law: a bare component name is the set of carriers. A-tags are writable — `+Marked` is FX_TAG, a bitmap OR.
- Option B, derived in the registry. race is an enum-valued column and the word Nord names the composition δ_Nord ∘ race — a mask recomputed against live race at each use. This is the one missing spelling: `as nord race Nord`, the `as` directive's third arity.
- Option C, derived in the program. `def rich = Gold > 10000` — shipped, and already intensional: ex09's whole witness is that a def is a predicate, not a snapshot (entity 0 crosses the threshold mid-program and the def sees it). The full predicate language is available here.

The boundary between B and C is the registry-as-data doctrine (REGFIX.md): the registry stores data — name-column-value triples are data — never programs; an arbitrary predicate is program text and belongs to def, or someday eval. fn's verbatim BQN body is the acknowledged bend, code carried as opaque data.

Both options are 100% legal and the choice encodes a domain invariant, not a performance preference:

- Exclusivity. B's tags are mutually exclusive per entity by construction — one race value per row. A's independent tags may overlap. Races want B; stackable statuses want A.
- Writability. A-tags take structural effects (+Nord, −Nord). B-tags are read-only as effect targets: setting Nord true is determined (race = :Nord), setting it false is not (race = what?) — δ_v has no inverse on the complement. The write spells `race = :Nord`; anoc rejects a derived tag in effect position by name, pointing at the carrier column.
- Presence. Under B, Nord means present(race) ∧ race = :Nord; the left-join-null rule already covers absence.
- Downstream, nothing can tell them apart. Both produce the one mask currency; σ, counting, γ, every effect is representation-blind. Post-state pins cannot distinguish A from B — the witness demo proves exactly that.

The case contract composes cleanly: tag names are names and fold; tag values are values and never fold. In `as nord race Nord`, nord folds and Nord does not — the first notebook page's own grid draws this, NORD and Nord distinct as race values while Gold folds to the gold column.

## The audit, rung by rung

- spaces: `lattice w h`, fields, frames and @, the anchored frame. Open: world-to-chunk (o, S), unchanged (ISSUES.md).
- matrices: fields (rank-2, flattened), vec columns (n×2, SoA), transforms via frames.
- arrays: `bind vec`, fn results; general registry arrays named-not-started (REGFIX.md item 6).
- columns: `col` num/bool/sym/char/vec, plus `pres` and `default`.
- relations: `rel` (functional, sentinel none), `srel` (fibered), `inv` (computed inverse), the key-column-in-relation-position rule, roles.
- tags: bool columns (A, shipped); `alias` stored masks — extensional snapshots, data, distinct from everything above (05-target-aliases' ^observer); the sym predicate (B's denotation, shipped; B's name, unwired); defs (C, shipped, intensional); `bind mask`.

One thing is missing on the whole board: the B wiring.

## The delta

1. `as <word> <col> <value>`, the third arity of the `as` directive — it lands inside REGFIX.md item 3, the same patch that introduces `as`. Loader: create a derived-tag entry (kind RK_TAG, carrying the column reference and the value) after the usual name checks — word wfree'd and fold-collision-checked like every entry name, col must resolve to a column, value parsed by the column's type, exact bytes for sym. Emitter: in mask position the entry emits the equality mask over the live column through the existing sym/num compare paths; in effect position it is an error naming the carrier column. reg_dump (REGFIX.md item 5) serializes it back as the same line.
2. Witness: an s57 pair. One program — `Nord & Gold > 50 , Gold += 500` over the notebook's own world (Gold 400 600 200 150 221, Master 0→2 and 3→4 with none elsewhere, races Nord Khajiit Nord Nord Orc) — two registries: A declares `col nord bool 1 0 1 1 0`, B declares `col race sym Nord Khajiit Nord Nord Orc` plus `as nord race Nord`. Identical expect pins. Deleting the `as` line must fail; the pin equality across A and B is the representation-independence claim as a running test.
3. Docs: src/GRAMMAR.md gets the arity beside the `as` directive, src/compiler.md one line. ano-ecs.md would take a derived-tag descriptor beside §10's registry sketch — proposed for the author, not performed.
4. Enums as declared value sets (`enum race Nord Khajiit Orc Imperial`: load-time validation, a closed universe for B's tags, checked value types in the C API) stay named-not-started with the rest of REGFIX.md item 6; the ladder gives them their rung — the finite point space tags draw from.

## Admissible, not adopted

- `Gold + 500` for `Gold += 500`. Effect position already determines the write-back, and the plan IR carries (column, op class, operand, mask) — the delta buffer is keyed by op class (ano-ecs.md §5), so the `=` in `+=` carries zero information. The contraction is pure surface and the author's call: terser, against the explicit read `+=` gives a reviewer, and the asymmetry that plain `=` (set) has no contracted form. Recorded, not resolved.
- Sparse tag storage (row lists rather than bitmaps): same denotation, a storage class for the C store — ano-ecs.md §16 already holds the sparse-column open question. No .reg change implied, no action here.