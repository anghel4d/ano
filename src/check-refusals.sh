#!/usr/bin/env bash
# Verifies the load/save boundary fixtures. A `# accept` load must pass. A `-- accept: <text>`
# save must pass, contain that text, and reload. Refusals exit with their pinned diagnostic and
# save refusals leave no output file. The pinned code is 2 unless `-- expect-exit: <n>` says
# otherwise. STEEL overrides the binary; without it a
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
  accept="$(sed -n '/^# accept$/p' "$f" | head -1)"
  want="$(sed -n 's/^# expect: //p' "$f" | head -1)"
  errout="$("$STEEL" --registry "$f" --emit "$here/refusals/probe.ano" 2>&1 >/dev/null)"
  code=$?
  if [ -n "$accept" ] && [ "$code" -eq 0 ]; then
    echo "ok-accept   $f"
  elif [ -z "$accept" ] && [ "$code" -eq 2 ] && [ -n "$want" ] && [ "${errout#*"$want"}" != "$errout" ]; then
    echo "ok-refuse   $f"
  else
    echo "FAIL-bound  $f (exit $code: $errout)"
    fail=1
  fi
done
for f in "$here"/refusals/save-*.ano; do
  [ -e "$f" ] || continue
  want="$(sed -n 's/^-- expect: //p' "$f" | head -1)"
  accept="$(sed -n 's/^-- accept: //p' "$f" | head -1)"
  code_want="$(sed -n 's/^-- expect-exit: //p' "$f" | head -1)"
  code_want="${code_want:-2}"
  out="${TMPDIR:-/tmp}/ano-refusal-$$-$(basename "$f" .ano).reg"
  rm -f "$out"
  errout="$("$STEEL" --run --save "$out" "$f" 2>&1 >/dev/null)"
  code=$?
  if [ -n "$accept" ] && [ "$code" -eq 0 ] && [ -e "$out" ] && grep -Fq -- "$accept" "$out" \
    && "$STEEL" --registry "$out" --emit "$here/refusals/probe.ano" >/dev/null 2>&1; then
    echo "ok-accept   $f"
  elif [ -z "$accept" ] && [ "$code" -eq "$code_want" ] && [ -n "$want" ] \
    && [ "${errout#*"$want"}" != "$errout" ] && [ ! -e "$out" ]; then
    echo "ok-refuse   $f"
  else
    if [ -e "$out" ]; then
      errout="$errout; output: $(tr '\n' ' ' <"$out")"
    fi
    echo "FAIL-bound  $f (exit $code: $errout)"
    fail=1
  fi
  rm -f "$out"
done
exit $fail
