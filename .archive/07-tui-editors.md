# DONE

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

## The modeless sketch (surfaced, not implemented — the call is the author's)

The micro-style keymap the rewrite would take, for the modeless-vs-fixed-vim decision. Done 2026-07-11: Tab-as-text in insert, the INSERT/BROWSE chip in the pane title, DECSCUSR bar in insert / block reverse in browse, and the hop chain — the half-vim is now honest enough to live with while this call waits.

| key | action |
| --- | --- |
| any printable, Enter, Tab | text, always — no modes; Tab is two spaces |
| arrows, Home, End, PgUp, PgDn | motion; Shift+arrows extend a selection |
| Ctrl+S | save (the explicit save stays; n still refuses on dirty) |
| Ctrl+Z | undo (the existing snapshot stack) |
| Ctrl+K | delete line (today's dd) |
| Ctrl+X / Ctrl+C / Ctrl+V | cut / copy / paste, internal register over the selection |
| Ctrl+F | find (base-letter matching stays); Enter next, Shift+Enter previous |
| Ctrl+G | go to line (today's [count]G) |
| ESC | clear selection or search, then out to the home surface |

The cost to surface with it: modeless steals the letter verbs inside code focus — r, n, u, m, w, E, q all type. The world verbs then live only outside code focus (one Tab away) or grow Ctrl chords of their own (Ctrl+R reset, Ctrl+N tick); either way the "one key steps the world" feel changes whenever the code pane holds focus. Counts (5j, 3dd, 12G) and word motions go entirely. What's bought: zero uncanny valley, every key does what a stranger expects, and real editing hops to nvim anyway.

## Adjacent, not this task

TODO.md item 4 (derive kore's palette from the shell theme, OSC 10/11) stays its own item.

## Invariants

- Raw-mode enter/leave stays balanced around the hop (term_leave restores ?25h and cooked mode; SIGINT ownership handed to the child editor and back). Don't regress the existing dance.
- Corpus immutability battery green: E under any focus still guards corpus files into play copies before the editor sees them.
- Zero new dependencies. The internal editor stays inside kore.c's zero-dep C23 discipline.
