# 11 — next steps: the blocking residue

What actually blocks, after the 01/02/04/05/07/08/09 landings. Everything decidable was decided and recorded where it executes; the deferred decision points live in 00-open-rulings (Q4, Q9–Q15). This file holds only what needs the author's hand.

- Apply `EDITOR=nvim` in your nix. The declarative half is in place — `~/nixos-config/home/linux.nix` sets `home.sessionVariables.EDITOR = "nvim"` and `nix flake check` passes — but the switch was permission-blocked for the agent. One command: `home-manager switch --flake ~/nixos-config#pyrus`. Until then the shell env still has no `$EDITOR`; kore's own chain already lands on nvim regardless.
- The modeless-vs-fixed-vim call for kore's internal editor. The three concrete fixes landed (Tab-as-text, the INSERT chip, DECSCUSR cursor), so the half-vim is honest enough to live with — but the micro-style rewrite waits on your ruling. The keymap sketch and its cost (letter verbs r/n/u/m/w/E/q become text inside code focus) sit in 07-tui-editors.md, "The modeless sketch"; carry it out of the archive with the file.
