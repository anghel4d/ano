#!/usr/bin/env bash
# Runs every .ano demo under demos/ through anoc --run (bqn from PATH), then checks
# every X-nihongo.ano conjugate emits byte-for-byte what its twin X.ano emits — two
# surfaces, one BQN program, enforced corpus-wide on every run. Inputs: none
# (env ANOC overrides the binary, default: anoc beside this script).
# Output: one line per file (ok/FAIL) and per pair (ok-emit/FAIL-emit).
# Exit: nonzero iff any demo or pair fails.
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
while IFS= read -r f; do
  twin="${f%-nihongo.ano}.ano"
  [ -f "$twin" ] || continue
  if cmp -s <("$ANOC" --emit "$f" 2>/dev/null) <("$ANOC" --emit "$twin" 2>/dev/null); then
    echo "ok-emit   $f"
  else
    echo "FAIL-emit $f"
    fail=1
  fi
done < <(find "$root/demos" -name '*-nihongo.ano' | sort)
exit $fail
