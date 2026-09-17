#!/usr/bin/env bash
# Runs every active explanatory BQN witness. Quarantined skips are always visible.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(git -C "$here" rev-parse --show-toplevel 2>/dev/null || echo "$here/..")"
BQN="${BQN:-bqn}"
# shellcheck source=../src/demo-quarantine.sh
source "$repo/src/demo-quarantine.sh"
ano_quarantine_load "$repo" || exit 1
work="$(mktemp -d "${TMPDIR:-/tmp}/ano-bqn-check-XXXXXX")"
trap 'rm -rf "$work"' EXIT
fail=0
seen=0

while IFS= read -r -d '' file; do
  seen=$((seen + 1))
  number="$(ano_demo_number "$file")" || {
    echo "FAIL-name   $file"
    fail=1
    continue
  }
  if ano_demo_quarantined "$number"; then
    echo "skip-quarantine $file"
    continue
  fi
  out="$work/run-$seen.out"
  if "$BQN" "$file" >"$out" 2>&1; then
    echo "ok-bqn      $file"
  else
    echo "FAIL-bqn    $file"
    sed 's/^/  /' "$out"
    fail=1
  fi
done < <(find "$repo/demos" -type d -name .kore -prune -o -type f -name '*.bqn' -print0 | sort -z)

if [ "$seen" -eq 0 ]; then
  echo "FAIL-account no .bqn demos found"
  fail=1
fi
exit "$fail"
