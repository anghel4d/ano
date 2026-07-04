#!/usr/bin/env bash
# Runs every .ano demo under demos/ through anoc --run (bqn from PATH). Inputs: none
# (env ANOC overrides the binary, default: anoc beside this script).
# Output: one line per file (ok/FAIL). Exit: nonzero iff any demo fails.
set -u
ANOC="${ANOC:-$(dirname "$0")/anoc}"
here="$(cd "$(dirname "$0")" && pwd)"
root="$(git -C "$here" rev-parse --show-toplevel 2>/dev/null || echo "$here/..")"
fail=0
while IFS= read -r f; do
  if "$ANOC" --run "$f" >/dev/null 2>&1; then
    echo "ok   $f"
  else
    echo "FAIL $f"
    fail=1
  fi
done < <(find "$root/demos" -name '*.ano' | sort)
exit $fail
