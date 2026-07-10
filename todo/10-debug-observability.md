# 10 — debug observability: the dead-link diagnostic and the tick trace

Mission-critical by ruling (author, 2026-07-11). The semantics stay silent; the debugger gets loud. Three observability features, bundled. Depends on 01 (the history/OUTPUTS split must exist to have somewhere to speak) and coordinates with 02 (the hop's found-guard is the hook).

## The dead-link diagnostic

Author's design, specified in the s10b ruling (the message is observability, never semantics — recorded in 02): when a relational hop crosses a link whose target no longer exists, the algebra silently clears the mask bit (left-join-null, §5); the debug surface reports it, in the author's format: `RELATION <column> <origin> -> <sink> IS DEAD !`. In Steel this is the gen-mismatch compare; in the anoc/BQN prototype the hook is the hop's found-guard landing with 02's fix — a debug mode (e.g. `anoc --trace`, or a kore toggle) that emits one line per dropped link and per empty-fiber row failure, plumbed into kore's history pane. Off by default; zero cost when off; never changes post-state.

## The tick trace

One line per tick summarizing structural effects: rows before/after, spawned k, killed j, per statement. This is the feature that would have made s10a's exponential obvious at a glance instead of "strange writes according to the terminal buffer": `tick 5: 48 rows -> 96 (spawn Ghost: +48)`. The world pane title carrying the live row count is the cheap half; the per-statement line in history is the real one. Source: the save pipe-back already knows the post-state; the emitter knows the per-statement deltas — pick the seam that stays out of the hot path.

## The repro channel

The `.kore` session log records every prompt statement and seam already — document it in kore.md as the canonical repro artifact: "when something looks wrong, the session log plus the play registry IS the bug report." Two standing unreproduced reports wait on exactly this: the 2/12 multi-effect that "wasn't firing off in some strange case" (retracted as not replicable — nothing order-dependent found in the emit path; leave a note that the report stands unreproduced), and anything the tick trace doesn't explain of s10a's terminal-buffer complaint.

## Invariants

- Post-state byte-identical with tracing on and off; both suites green with the flag in both positions.
- Trace lines go to history (01's small pane), never to OUTPUTS (query results only) and never into the 0x1E save channel.
- Off by default; the corpus check scripts run without it.
