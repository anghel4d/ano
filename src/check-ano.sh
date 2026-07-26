#!/usr/bin/env bash
# Runs every demo, compares each Japanese twin's emitted BQN with its ASCII twin, then runs
# the refusal battery. STEEL overrides target/release/steel. Exits nonzero on any failure.
set -u
STEEL="${STEEL:-$(dirname "$0")/../target/release/steel}"
here="$(cd "$(dirname "$0")" && pwd)"
root="$(git -C "$here" rev-parse --show-toplevel 2>/dev/null || echo "$here/..")"
fail=0

# The set of decommissioned demo numbers is read from the quarantine block in todo/TODO.md,
# which is the single authority for it, and is shared by every harness rather than restated
# per script. A demo whose leading three-digit number is decommissioned is skipped, in both
# the run pass and the Japanese-twin emission comparison, and each skip is announced on its
# own line, so an exclusion is always visible and never silent.
# Every demo file is accounted for: each matches the three-digit naming convention and lands
# in exactly one of active or decommissioned, and the harness fails if any file falls outside
# that accounting. A decommissioned number carrying no file of a given kind is normal, not a gap.

echo "src/check-ano.sh: quarantine-aware enumeration is not implemented" >&2
exit 1
