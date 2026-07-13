#!/usr/bin/env bash
# Verifies that each load/save refusal fixture exits 2 with its pinned diagnostic. Save
# refusals must leave no output file. STEEL overrides target/release/steel.
set -u
STEEL="${STEEL:-$(dirname "$0")/../target/release/steel}"
here="$(cd "$(dirname "$0")" && pwd)"
fail=0
for f in "$here"/refusals/load-*.reg; do
  [ -e "$f" ] || continue
  want="$(sed -n 's/^# expect: //p' "$f" | head -1)"
  errout="$("$STEEL" --registry "$f" --emit "$here/refusals/probe.ano" 2>&1 >/dev/null)"
  code=$?
  if [ "$code" -eq 2 ] && [ -n "$want" ] && [ "${errout#*"$want"}" != "$errout" ]; then
    echo "ok-refuse   $f"
  else
    echo "FAIL-refuse $f (exit $code: $errout)"
    fail=1
  fi
done
for f in "$here"/refusals/save-*.ano; do
  [ -e "$f" ] || continue
  want="$(sed -n 's/^-- expect: //p' "$f" | head -1)"
  out="${TMPDIR:-/tmp}/ano-refusal-$$-$(basename "$f" .ano).reg"
  rm -f "$out"
  errout="$("$STEEL" --run --save "$out" "$f" 2>&1 >/dev/null)"
  code=$?
  if [ "$code" -eq 2 ] && [ -n "$want" ] && [ "${errout#*"$want"}" != "$errout" ] && [ ! -e "$out" ]; then
    echo "ok-refuse   $f"
  else
    echo "FAIL-refuse $f (exit $code: $errout)"
    fail=1
  fi
  rm -f "$out"
done
exit $fail
