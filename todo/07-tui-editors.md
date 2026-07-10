# 07 — the editors: kill vim.tiny, fix the internal editor

All the editor complaints, one action item (author, 2026-07-11).

## The diagnosis (recorded, verified)

The "Capital E editor" horror was never kore's editor. `$EDITOR` is unset on this machine. kore's editor hop falls back to `vi` (kore.c:2953), and `/usr/bin/vi` → `/etc/alternatives/vi` → **vim.tiny**, which with no `~/.vimrc` runs in vi-compatible mode: arrow keys insert ABCD in insert mode, no mode indicator, feature-stripped everything. Meanwhile nvim sits in the nix profile with a full config. Verbatim ruling: "Debian's vim.tiny, fuck that. Set it to NVIM. Fix it. At the project level, AND in my nix."

## Work items — the hop

- kore fallback chain: `$VISUAL → $EDITOR → nvim → vim → micro → nano → vi`. The floor is never vim.tiny.
- Project level: ano has a flake (flake.nix at root — the author asked; yes). Take the devshell pattern from anoptic-engine's `feature-profiling` branch flake. Set `EDITOR=nvim` in the devshell only when unset, so it never stomps a user's choice.
- User nix: set EDITOR=nvim declaratively in the author's own config, in the dots repo's shell rc (or home config), per the machine doctrine: declarative in the private `anghel4d/{dots,nixos-config}` repos, never an ad-hoc export. This half executes in those repos, not here.

## Work items — the internal editor

Author verdict: "The internal editor was actually a lot better." What's wrong with it, verified in code, and what's wanted:

- Tab in insert mode is swallowed by the global focus-cycler (kore.c:3159 handles K_TAB before the code surface sees it), so you cannot type a tab character. Fix: insert mode consumes Tab as text (or as N spaces per the corpus convention). Focus-cycling keeps Tab everywhere else.
- The mode is shown only as a status-bar whisper (kore.c:2745 "insert — esc returns to browse"). Wanted: a loud mode indicator in the code pane's own title, not the far corner.
- The cursor is a reverse-video cell, easy to lose, and it doesn't distinguish browse from insert. Wanted: a real cursor treatment (bar/underline in insert via DECSCUSR when the terminal honors it, block reverse in browse).
- Standing wish, recorded from the review: if vim behaviors can't be got right, clone micro instead. Modeless, always-insert, ctrl chords, standard selections. For a debug pane you hop into for one-line tweaks, modeless is the honest shape. The current half-vim is the uncanny valley. Direction: micro-style internal editor, E-hop to real nvim for real editing. Surface the final modeless-vs-fixed-vim call to the author with a keymap sketch before rewriting.

## Adjacent, not this task

TODO.md item 4 (derive kore's palette from the shell theme, OSC 10/11) stays its own item.

## Invariants

- Raw-mode enter/leave stays balanced around the hop (term_leave restores ?25h and cooked mode; SIGINT ownership handed to the child editor and back). Don't regress the existing dance.
- Corpus immutability battery green: E under any focus still guards corpus files into play copies before the editor sees them.
- Zero new dependencies. The internal editor stays inside kore.c's zero-dep C23 discipline.
