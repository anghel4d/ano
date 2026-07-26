#!/usr/bin/env bash
# Verifies that each load/save refusal fixture exits with its pinned diagnostic. Save
# refusals must leave no output file. The pinned code is 2 (the module refusal) unless the
# fixture declares `-- expect-exit: <n>` — a staged runtime assertion refuses before the save
# boundary is reached and carries BQN's own code. STEEL overrides the binary; without it a
# fresh release build is made every run, so a stale binary is never tested.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
if [ -z "${STEEL:-}" ]; then
  root="$here/.."
  tdir="${CARGO_TARGET_DIR:-$root/target}"
  case "$tdir" in /*) ;; *) tdir="$root/$tdir" ;; esac
  ( cd "$root" && cargo build --release -p steel ) || exit 1
  STEEL="$tdir/release/steel"
fi
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
  code_want="$(sed -n 's/^-- expect-exit: //p' "$f" | head -1)"
  code_want="${code_want:-2}"
  out="${TMPDIR:-/tmp}/ano-refusal-$$-$(basename "$f" .ano).reg"
  rm -f "$out"
  errout="$("$STEEL" --run --save "$out" "$f" 2>&1 >/dev/null)"
  code=$?
  if [ "$code" -eq "$code_want" ] && [ -n "$want" ] && [ "${errout#*"$want"}" != "$errout" ] && [ ! -e "$out" ]; then
    echo "ok-refuse   $f"
  else
    echo "FAIL-refuse $f (exit $code: $errout)"
    fail=1
  fi
  rm -f "$out"
done
exit $fail
