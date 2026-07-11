#!/usr/bin/env bash
# The negative battery: every fixture under src/refusals/ must REFUSE — exit 2 with the
# diagnostic it pins ('# expect:' in a .reg, '-- expect:' in a .ano) on stderr. Two
# shapes: load-*.reg loads through refusals/probe.ano and must die at reg_load (the
# non-finite seal, outside face); save-*.ano runs under --run --save to a scratch path
# that must not appear afterward (the refused tick, the world stands). save-*-world.reg
# are the clean worlds behind the .ano fixtures, not fixtures themselves.
# Inputs: none (env ANOC overrides the binary, default: anoc beside this script).
# Output: one ok-refuse/FAIL-refuse line per fixture. Exit: nonzero iff any fails.
set -u
ANOC="${ANOC:-$(dirname "$0")/anoc}"
here="$(cd "$(dirname "$0")" && pwd)"
fail=0
for f in "$here"/refusals/load-*.reg; do
  [ -e "$f" ] || continue
  want="$(sed -n 's/^# expect: //p' "$f" | head -1)"
  errout="$("$ANOC" --registry "$f" --emit "$here/refusals/probe.ano" 2>&1 >/dev/null)"
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
  errout="$("$ANOC" --run --save "$out" "$f" 2>&1 >/dev/null)"
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
