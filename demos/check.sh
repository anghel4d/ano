#!/usr/bin/env bash
# Runs every demo under demos/. Inputs: none (env BQN overrides the interpreter, default: bqn on PATH).
# Output: one line per file (ok/FAIL). Exit: nonzero iff any demo fails.
set -u
BQN="${BQN:-bqn}"

# The set of decommissioned demo numbers is read from the quarantine block in todo/TODO.md,
# which is the single authority for it, and is shared by every harness rather than restated
# per script. A demo whose leading three-digit number is decommissioned is skipped, and each
# skip is announced on its own line, so an exclusion is always visible and never silent.
# Every demo file is accounted for: each matches the three-digit naming convention and lands
# in exactly one of active or decommissioned, and the harness fails if any file falls outside
# that accounting. A decommissioned number carrying no file of a given kind is normal, not a gap.

echo "demos/check.sh: quarantine-aware enumeration is not implemented" >&2
exit 1
