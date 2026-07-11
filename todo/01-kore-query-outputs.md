# DONE

# 01 — kore query outputs: computed, asserted, never shown

Ruled FIRST (author, 2026-07-11): "This seems like a rather serious error." One dedicated agent. This file gives a deliberately vague pointer and strict invariants. Find the exact seam yourself.

## The problem

Running any corpus demo in kore shows no query results, ever. `+/ threat @ Enemy` in demo 25 computes 390, verifies it, and prints nothing. The whole fold/scan series (demos 14-17) is invisible. Reported symptom, verbatim: "where's --! out stuff going?!? no reduces are ever printed."

## The pointer (vague, by ruling)

Two halves. The emitter's query path (src/emit.c, around emitQuery) either silently asserts a pinned `--! out` expectation or shows an unpinned one. kore's tick (kore/kore.c, world_next's directive filter) strips `--! expect` pins before running but keeps `--! out` pins. Every corpus query is pinned. Draw the conclusion, verify it live, then fix.

There is a latent second failure in the same seam: a kept out-pin that reads a column the demo's own statements mutate will fail the assertion on tick 2+ and hold the world still. That is exactly the failure mode the expect-strip exists to prevent.

## The redesign (author's spec, binding)

The box currently labeled `output` is renamed `history`, made smaller, placed just above the `>` prompt, secondary. It keeps what it shows today: console output, `$` command lines, the effects/copy announcements.

The reclaimed large surface becomes OUTPUTS: actual query results, per tick, each labeled by its source statement (e.g. `q1 · +/ threat @ Enemy → 390`), newest tick first. Folds, scans, grades, top-k: everything that answers without mutating World lands here.

## Invariants that must NOT be broken

- `demos/check.sh` and `src/check-ano.sh` stay green. The pristine-run contract is untouched: `anoc --run` on an unmodified corpus demo still exits nonzero when a pinned out or expect diverges. Pins remain the verification substrate.
- kore never writes any file under `demos/` (the corpus-immutability battery stays green) and never fails a tick over a pin that is only stale relative to a stepped world.
- The `--save` pipe-back stays uncontaminated: 0x1E-sentinel lines are the world channel. Query display is never parsed as world state, and world state never renders as query output.
- Every query statement, pinned or unpinned, produces a visible result in kore after the fix.
- `--! expect` behavior is unchanged in every mode.
- `kore --check` stays at full ok over the registries. PTY batteries stay green. The build stays warning-free under the existing flags.

## Acceptance

Open demo 25 (derived columns), press n: 390 and the descending grade vector appear in OUTPUTS. Open demo 14 (reductions): the whole census, including the empty-scope identities. History still shows the `$ n` line, smaller, above the prompt.
