#!/usr/bin/env bash
# Runs every demo, compares each Japanese twin's emitted BQN with its ASCII twin, then runs
# the refusal battery. STEEL overrides target/release/steel. Exits nonzero on any failure.
set -u
STEEL="${STEEL:-$(dirname "$0")/../target/release/steel}"
here="$(cd "$(dirname "$0")" && pwd)"
root="$(git -C "$here" rev-parse --show-toplevel 2>/dev/null || echo "$here/..")"
fail=0
while IFS= read -r f; do
  if "$STEEL" --run "$f" >/dev/null 2>&1; then
    echo "ok   $f"
  else
    echo "FAIL $f"
    fail=1
  fi
done < <(find "$root/demos" -name '*.ano' | sort)
while IFS= read -r f; do
  twin="${f%-nihongo.ano}.ano"
  [ -f "$twin" ] || continue
  if cmp -s <("$STEEL" --emit "$f" 2>/dev/null) <("$STEEL" --emit "$twin" 2>/dev/null); then
    echo "ok-emit   $f"
  else
    echo "FAIL-emit $f"
    fail=1
  fi
done < <(find "$root/demos" -name '*-nihongo.ano' | sort)
STEEL="$STEEL" bash "$here/check-refusals.sh" || fail=1
exit $fail
