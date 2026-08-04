#!/usr/bin/env bash
# Runs every active demo through Steel, compares each active Japanese twin's emitted BQN with
# its ASCII twin, then runs the numeric and refusal boundary battery. Quarantined skips are always visible.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(git -C "$here" rev-parse --show-toplevel 2>/dev/null || echo "$here/..")"
if [ -z "${STEEL:-}" ]; then
  tdir="${CARGO_TARGET_DIR:-$repo/target}"
  case "$tdir" in /*) ;; *) tdir="$repo/$tdir" ;; esac
  ( cd "$repo" && cargo build --release -p steel ) || exit 1
  STEEL="$tdir/release/steel"
fi
# shellcheck source=demo-quarantine.sh
source "$here/demo-quarantine.sh"
ano_quarantine_load "$repo" || exit 1
work="$(mktemp -d "${TMPDIR:-/tmp}/ano-check-XXXXXX")"
trap 'rm -rf "$work"' EXIT
fail=0
seen=0
twin=0

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
  if "$STEEL" --run "$file" >"$out" 2>&1; then
    echo "ok-run      $file"
  else
    echo "FAIL-run    $file"
    sed 's/^/  /' "$out"
    fail=1
  fi
  case "$file" in
    *-nihongo.ano)
      ascii="${file%-nihongo.ano}.ano"
      twin=$((twin + 1))
      if [ ! -f "$ascii" ]; then
        echo "FAIL-twin   $file (missing $ascii)"
        fail=1
        continue
      fi
      left="$work/twin-$twin-ascii.bqn"
      right="$work/twin-$twin-nihongo.bqn"
      if "$STEEL" --emit "$ascii" >"$left" 2>"$work/twin-$twin-ascii.err" \
        && "$STEEL" --emit "$file" >"$right" 2>"$work/twin-$twin-nihongo.err" \
        && cmp -s "$left" "$right"; then
        echo "ok-twin     $file"
      else
        echo "FAIL-twin   $file"
        sed 's/^/  /' "$work/twin-$twin-ascii.err" "$work/twin-$twin-nihongo.err"
        diff -u "$left" "$right" | sed 's/^/  /' || true
        fail=1
      fi
      ;;
  esac
done < <(find "$repo/demos" -type d -name .kore -prune -o -type f -name '*.ano' -print0 | sort -z)

if [ "$seen" -eq 0 ]; then
  echo "FAIL-account no .ano demos found"
  fail=1
fi
if ! STEEL="$STEEL" bash "$here/check-refusals.sh"; then
  fail=1
fi
exit "$fail"
