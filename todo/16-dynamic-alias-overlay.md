# 16 — dynamic alias overlay

Status. Ruled 2026-07-21. Implementation and tests are pending in Steel and Kore.

## Ruling

`!` is mask negation. `^` selects a dynamic alias that may change between statement steps.

Bare and sigiled lookup are separate for the same stem. `Whiterun` is the registered column or binding. `!Whiterun` negates its mask. `^Whiterun` reads the live alias named `Whiterun`, which may target another column or an existing mask, when one exists and otherwise falls through to bare `Whiterun`. Installing, rebinding, or deleting that alias changes only sigiled lookup. It never destroys, replaces, or mutates the bare binding. Deleting the alias restores the fallback.

The notation `^Whiterun = value` states the binding relation here. It does not by itself rule a program-level assignment surface. The host/API spelling for installing, rebinding, and deleting aliases must be fixed when this task executes.

## Current implementation

Steel lexes `^name` separately but `steel/src/emit.rs` strips the sigil and performs an ordinary registry lookup. This accidentally provides bare fallback but has no distinct live overlay, no alias lifecycle, and no way for a same-stem alias to shadow only sigiled lookup. Kore therefore has no complete host-side contract for the ruling.

The registry directive `alias name ...` currently stores fixture masks. It must not be mistaken for the live overlay without an explicit migration decision.

## Work

1. Add a live alias overlay keyed with the registry's name-folding rule but separate from the bare registry namespace.
2. Resolve `^name` overlay-first and then fall through to bare `name`; keep bare `name` on the existing path.
3. Expose host operations to install, rebind, and delete an alias between statement steps. Capture one alias environment for each statement evaluation so a gather observes one snapshot.
4. Preserve alias changes in the deterministic input or statement log required for replay.
5. Add Steel and Kore tests for fallback, same-stem shadowing, rebinding, deletion, bare-name survival, `!Whiterun`, and `!^Whiterun`.
6. Decide whether the fixture `alias` directive seeds the live overlay or remains a stored-mask fixture, then update the registry docs and demos without overloading the two meanings silently.

## Invariants

- `!` remains ordinary pointwise mask NOT.
- Alias lifecycle never mutates or removes a column, binding, or other bare registry entry.
- A live alias may use the same stem as a bare entry; that collision is the intended overlay case.
- `^name` and bare `name` differ only when the live overlay contains `name`.
- Removing the live alias makes `^name` resolve exactly as bare `name` again.
- One statement evaluates against one world and alias snapshot. Aliases may change between statement steps, not halfway through one.
