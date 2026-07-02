#!/usr/bin/env bash
# Runs every demo under demos/. Inputs: none (env BQN overrides the interpreter, default: bqn on PATH).
# Output: one line per file (ok/FAIL). Exit: nonzero iff any demo fails.
set -u
BQN="${BQN:-bqn}"
fail=0
while IFS= read -r f; do
  if "$BQN" "$f" >/dev/null 2>&1; then
    echo "ok   $f"
  else
    echo "FAIL $f"
    fail=1
  fi
done < <(find "$(cd "$(dirname "$0")" && pwd)" -name '*.bqn' | sort)
exit $fail
