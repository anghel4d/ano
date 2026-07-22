# 15 — the spatial registry surface: the capability ladder, both surfaces, and the five-world prototype path

The registry surface is adjectives applied to an essential noun, then a name, then optional data. Layout is always SoA and is never surface. Nothing here is built without the author's explicit go.

## The ladder

The current mathematics has ten layers between the raw buffer and a placed euclidean point. Three never reach the surface. Each declarable layer is one adjective or one noun, one row of `spatialmaths.md` §22's capability list, and one separately-witnessed flag in the loader's certificate (`todo/13-spatial-formalization.md:196`).

| # | layer | buys | surface | status |
|---|---|---|---|---|
| 1 | ravel — the bytes, SoA | — | never | — |
| 2 | layout `Fin(N) ≅ D_H` (§3) | buffer order | never; relayout invariance proven (`Layout.change_*`) | proven |
| 3 | habitat `D_H` (§2) | alignment, gather, scatter | minted by the declaring noun | proven |
| 4 | product presentation `b_H` (§13) | rank, shape, transpose | `lattice` shape data | defined |
| 5 | lattice chart `Λ_H`, `κ`, `lookup` (§13) | shifts, stencils, explicit edge failure | the `lattice` noun | part proven (nominal box round trips only) |
| 6 | frame — nominal Point/Vector torsor (§14) | point−point, point+vector | the `frame` noun | proven |
| 7 | coordinate presentation (§14) | numeric coords, scaling, weights | `scalar` `dim` `axes` `units` adjectives | defined, not proven (`proofs/lean.md:60`) |
| 8 | placement `χ : D_H → Point(F)` (§14); locator/interpolator bridges (§15–16) | continuous cell positions; points back to cells | `placement` noun, `embedding` flag, bridge nouns | proven in general form |
| 9 | metric (§21–22) | distance, nearest, radius | `metric` adjective | defined only |
| 10 | policies: numeric refinement (`todo/13:207`), boundary (§20) | lawful refusals on IEEE and edges | `numeric` and `boundary` adjectives | defined only |


## The maximal surface — a euclidean 3D volume, every witness explicit

```
frame World:
  scalar f64                  -- module rung: which semiring acts on Vector(World)
  dim 3
  axes x y z                  -- names, not keywords
  units m
  metric euclid               -- distance witness, nothing more
  numeric strict              -- IEEE policy: no algebraic rewrites (or: tolerance 1e-9, interval, host)

lattice Volume 64 64 64       -- rank-3 window: product presentation + chart + lookup in one noun

placement Volume World:
  origin [0 0 0]
  basis  [1 0 0] [0 1 0] [0 0 1]
  embedding                   -- injectivity, its own flag, never inferred from the numbers
  boundary refuse             -- or clamp, or wrap

locator  toCell   World Volume nearest     -- points → cells, tie rule declared
interp   sample   Volume World trilinear   -- cells → points, weights normalized

col Density Volume f32 [ ... ]             -- ordinary field over the lattice habitat
col PosX World.x [ ... ]                   -- entity positions, axis-kinded
col PosY World.y [ ... ]
col PosZ World.z [ ... ]
```

Four nouns — `frame`, `lattice`, `placement`, `col` — plus the two bridge nouns when wanted. Everything else is enum adjectives.

## The minimal surface — the same world under supersane defaults

```
euclid3 frame World

col PosX World.x [2 2 1 4 2 44 2 1 2 1 4]
col PosY World.y [2 1 1 2 4 67 2 1 4 68 4]
col PosZ World.z [0 0 1 0 2 12 0 0 3 1 0]
```

Defaults: `euclid3` mints dim 3 and axes x y z; scalar defaults to the repo's f64 model; metric euclid; the default deterministic numeric policy; no placement line because a root frame needs none. The volume rides one more line: `lattice Volume 64 64 64 in World`, with `in` defaulting the placement to origin zero, unit basis, embedding (the window injects, so the witness is free). The floor: the frame's name can never be defaulted away — auto-minting a frame erases thisness, and nominality is the entire semantic content of rung 6. One frame line, then axis-kinded cols, is the minimum the mathematics permits.

## The elaboration law

Minimal and maximal load to the identical certificate. A macro-adjective (`euclid3`) expands at load; the certificate keeps metric, dimension, module, and injectivity as separate witnesses per `todo/13:196`. Defaults are elaboration, never weaker checking. Surface bundling is legal precisely because it dies at the loader.

## The five-world prototype path — below the certificate line

discrete 2D grid, discrete 3D voxels, continuous 2D euclid, continuous 3D euclid, continuous 3D sphere, without completing the pending proofs. Capabilities compose by conjunction (§22); each world stands on a short stack.

| world | rungs needed | exists |
|---|---|---|
| discrete 2D grid | lattice | yes — `lattice <w> <h>` (`steel/src/registry.rs:269`), Conway demos run |
| discrete 3D voxels | lattice, rank 3 | same code, one more axis |
| continuous 2D euclid | frame-as-convention + per-axis cols | cols and arithmetic are the language |
| continuous 3D euclid | same + one col | same |
| continuous 3D sphere | unit-vector cols + explicit neighbor srel + registered fns | machinery exists, fixture does not |

Three facts make the shortcut lawful. The Conway precedent (`spatialmaths.md:698`): an explicit relation reconstructs the required fibers over a fixed field, so neighborhoods need no chart proofs when materialized as an `srel` — the sphere's adjacency is an icosphere or lat-long srel built at load, queried through existing relation machinery. General placement includes curved and host-defined placements (§14), so a sphere grid placed into World3 is inside the mathematics, in the rung demanding the least; great-circle distance and slerp enter as registered fns (`host_callable`, the phyllotaxis precedent: proof-carrying input, not derived law). And frames can land as inert surface — parsed, stored, arity-checked, enforcing nothing — so the surface and fixtures accumulate while the certificate layer wires in later.

The price is refusal, not function. The unwritten rungs are the type-is-the-license layer: the prototype will not refuse a foreign-habitat join, a frame mix, or a buffer-length coincidence. That is the failure §26 documents — first tick plausible, second tick incompatible lengths. Discipline, non-negotiable:

- Every fixture checks rank, shape, and habitat after each tick and runs a second tick (§26's law as `--! expect` assertions).
- The sphere trap in the fixture, not the footnote: naive lerp leaves the sphere; positions renormalize at the barrier or move by registered slerp.
- Every prototype header says convention-not-certificate. These fixtures must not overclaim.
- Written as `--! expect` fixtures they are not throwaway: they are the acceptance corpus the certified path must later reproduce byte-identically, per `todo/12-spatial-formalization.md`'s framing.

## Open sub-questions (surface at execution, never resolve silently)

- Noun order. Existing `.reg` is noun-first (`col gold num`); the ruling's shape is adjectives-then-noun-then-name. One grammar must win, or the loader accepts both during migration — which?
- The block form (`frame World:` with indented adjective lines) versus the line-based loader. One noun opening a block is new to `.reg`; alternative is repeated `World <adjective> …` lines.
- `in` as placement-default sugar on the `lattice` line: sugar worth its keyword, or spell `placement` always?
- A default frame when exactly one is declared (`col PosX x`): supersane or too clever?

## Work items

- Gate: the author's explicit go. Then, in order:
- Extend `lattice` to rank 3 (chart and lookup generalize mechanically; the emitter's 2D assumptions audited).
- Parse and store the `frame` / axis-kind / `placement` surface as inert metadata; refuse only arity and unknown adjectives.
- Build the five prototype worlds as demo fixtures with the §26 discipline and convention-not-certificate headers; sphere adjacency as srel data in the `.reg`, great-circle and slerp as registered fns.
- Land the elaboration law in the loader: macro-adjectives expand, witnesses stored separately even while unchecked.

## Invariants after

- Layout is never surface; no `rowmajor` word exists in any file.
- The certificate keeps every witness separate regardless of surface bundling.
- Every prototype fixture carries the second-tick checks and the convention-not-certificate header.
- Docs must not promise unbuilt surface (task 04's law): the checked/unchecked line is stated wherever the surface is shown.
