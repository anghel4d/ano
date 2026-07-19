# 11 — next steps:

Make the cursor visible in kore's edit-mode. Currently it is only visible in normal mode.

Standing 2026-07-19: still open; the hardware-cursor park already exists (`kore/src/term.rs:391` positions, shapes, and shows it when `cur_y`/`cur_x` are set), so the fix is likely setting the park coordinates from edit-mode's cell. Tracked under TODO.md's spatial-views item.
