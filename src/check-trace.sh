#!/usr/bin/env bash
# Runs the relationship-diagnostic contract of todo/04 against a live BQN backend: which domain
# each crossing reports over, that -1 stays silent, that every runtime line carries its use
# identity, and that enabling the trace changes no denotation. One "ok" line per leg.
#
# Fixtures live in src/trace/ (see src/trace/trace.md). The witness demos are READ-ONLY inputs:
# nothing here edits demos/. STEEL overrides the binary; without it a fresh release build is
# made every run, so a stale binary is never tested.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
root="$(git -C "$here" rev-parse --show-toplevel 2>/dev/null || echo "$here/..")"
if [ -z "${STEEL:-}" ]; then
  tdir="${CARGO_TARGET_DIR:-$root/target}"
  case "$tdir" in /*) ;; *) tdir="$root/$tdir" ;; esac
  ( cd "$root" && cargo build --release -p steel ) || exit 1
  STEEL="$tdir/release/steel"
fi
work="$(mktemp -d "${TMPDIR:-/tmp}/ano-trace-XXXXXX")"
trap 'rm -rf "$work"' EXIT
sep="$(printf '\037')"
fail=0

# Inputs: a leg name and its verdict (0 = pass). Output: the one line the leg owes.
leg() {
  if [ "$2" -eq 0 ]; then
    echo "ok   $1"
  else
    echo "FAIL $1"
    fail=1
  fi
}

# Inputs: a .ano path. Output: its 0x1F trace lines, separator stripped, in emission order.
trace_of() {
  "$STEEL" --run --trace "$1" 2>&1 | grep "^$sep" | sed "s/^$sep//"
}

# Inputs: a fixture stem under src/trace and the expected trace block on stdin.
# Output: 0 when the whole block matches byte for byte.
expect_trace() {
  local got="$work/$1.trace"
  trace_of "$here/trace/$1.ano" > "$got"
  diff -u - "$got" > "$work/$1.diff" 2>&1 || { cat "$work/$1.diff"; return 1; }
  return 0
}

# Inputs: any .ano path. Output: the count of DEAD and EMPTY event lines it emits.
events_of() {
  trace_of "$1" | grep -c "IS DEAD !\|IS EMPTY !"
}

# ---- leg 1: the sentinel is silent, in every phase and in the witness demo ----
rc=0
[ "$(events_of "$here/trace/silent.ano")" -eq 0 ] || rc=1
[ "$(events_of "$root/demos/1-selection/008-relationship-hops.ano")" -eq 0 ] || rc=1
leg "sentinel-silent" $rc

# ---- leg 2: never-existed and despawned keys are DEAD, with their use suffixes ----
rc=0
expect_trace dead-key <<EOF || rc=1
RELATION mentor 13 -> 99 IS DEAD ! USE 0 PREDICATE SOURCE
RELATION mentor 17 -> 98 IS DEAD ! USE 0 PREDICATE SOURCE
RELATION mentor 13 -> 99 IS DEAD ! USE 1 EFFECT SELECTED
TRACE-USE 0 s1:7 PREDICATE SOURCE mentor
TRACE-USE 1 s2:8 EFFECT SELECTED mentor
EOF
expect_trace despawned <<EOF || rc=1
s2: 5 rows -> 4 (kill: -1)
RELATION mentor 13 -> 11 IS DEAD ! USE 1 PREDICATE SOURCE
RELATION mentor 17 -> 11 IS DEAD ! USE 1 PREDICATE SOURCE
TRACE-USE 0 s1:8 PREDICATE SOURCE mentor
TRACE-USE 1 s3:10 PREDICATE SOURCE mentor
EOF
leg "dead-keys" $rc

# ---- leg 3: an in-carrier target past the world runs, drops the row and reports DEAD ----
rc=0
expect_trace unkeyed-oob <<EOF || rc=1
RELATION mentor 2 -> 9 IS DEAD ! USE 0 PREDICATE SOURCE
TRACE-USE 0 s1:6 PREDICATE SOURCE mentor
EOF
"$STEEL" --run "$here/trace/unkeyed-oob.ano" >/dev/null 2>&1 || rc=1
leg "unkeyed-out-of-range" $rc

# ---- leg 4: the phase table — X for a predicate crossing, S for an effect one ----
rc=0
expect_trace excluded-effect <<EOF || rc=1
TRACE-USE 0 s1:6 EFFECT SELECTED mentor
EOF
expect_trace excluded-predicate <<EOF || rc=1
RELATION mentor 17 -> 99 IS DEAD ! USE 0 PREDICATE SOURCE
TRACE-USE 0 s1:6 PREDICATE SOURCE mentor
EOF
leg "phase-domains" $rc

# ---- leg 5: fibers — a scoped dead member, an empty fiber, a gamma in effect position ----
rc=0
expect_trace fiber <<EOF || rc=1
RELATION targets 7 -> 99 IS DEAD ! USE 0 PREDICATE SOURCE
RELATION targets 13 -> 98 IS DEAD ! USE 0 PREDICATE SOURCE
RELATION targets 7 -> 99 IS DEAD ! USE 1 EFFECT SELECTED
RELATION targets 7 -> 99 IS DEAD ! USE 2 EFFECT SELECTED
FIBER targets 11 IS EMPTY ! USE 3 EFFECT SELECTED
TRACE-USE 0 s1:9 PREDICATE SOURCE targets
TRACE-USE 1 s2:10 EFFECT SELECTED targets
TRACE-USE 2 s3:11 EFFECT SELECTED targets
TRACE-USE 3 s3:11 EFFECT SELECTED targets
EOF
leg "fiber-domains" $rc

# ---- leg 6: two rules crossing one relationship in one tick stay two use records ----
rc=0
expect_trace rules <<EOF || rc=1
RELATION mentor 13 -> 99 IS DEAD ! USE 0 PREDICATE SOURCE
RELATION mentor 19 -> 99 IS DEAD ! USE 0 PREDICATE SOURCE
RELATION mentor 13 -> 99 IS DEAD ! USE 1 EFFECT SELECTED
RELATION mentor 19 -> 99 IS DEAD ! USE 1 EFFECT SELECTED
TRACE-USE 0 s1:7 PREDICATE SOURCE mentor
TRACE-USE 1 s1:8 EFFECT SELECTED mentor
EOF
leg "rule-tick-crossings" $rc

# ---- leg 7: the witnesses stay live and stay silent (read-only) ----
rc=0
for w in 1-selection/008-relationship-hops 1-selection/015-keyed-hop \
         2-effects/026-hop-after-despawn 2-effects/027-hop-out-of-range \
         9-nihongo/087-fallen-master; do
  d="$root/demos/$w.ano"
  "$STEEL" --run --trace "$d" >/dev/null 2>&1 || { echo "  $w does not run"; rc=1; }
  # every stored target in these worlds resolves, so the contract is silence, not DEAD
  [ "$(events_of "$d")" -eq 0 ] || { echo "  $w reported an event"; rc=1; }
done
leg "witness-demos" $rc

# ---- leg 8: parity — the trace changes neither post-state, nor output, nor exit code ----
rc=0
for f in "$here"/trace/*.ano; do
  b="$(basename "$f" .ano)"
  "$STEEL" --run --save "$work/$b-on.reg" --trace "$f" > "$work/$b-on.out" 2>&1
  on=$?
  "$STEEL" --run --save "$work/$b-off.reg" "$f" > "$work/$b-off.out" 2>&1
  off=$?
  grep -v "^$sep" "$work/$b-on.out" > "$work/$b-on.strip"
  [ "$on" -eq "$off" ] || { echo "  $b exit $on vs $off"; rc=1; }
  cmp -s "$work/$b-on.reg" "$work/$b-off.reg" || { echo "  $b post-state differs"; rc=1; }
  cmp -s "$work/$b-on.strip" "$work/$b-off.out" || { echo "  $b stdout differs"; rc=1; }
done
leg "trace-parity" $rc

# ---- leg 9: three identical runs are byte-identical ----
rc=0
for f in "$here"/trace/*.ano; do
  b="$(basename "$f" .ano)"
  for i in 1 2 3; do trace_of "$f" > "$work/$b-run$i"; done
  cmp -s "$work/$b-run1" "$work/$b-run2" || rc=1
  cmp -s "$work/$b-run2" "$work/$b-run3" || rc=1
done
leg "determinism" $rc

# ---- leg 10: every emitted line matches the ruled grammar ----
rel_re='^RELATION [^ ]+ -?[0-9][0-9.e+-]* -> -?[0-9][0-9.e+-]* IS DEAD ! USE [0-9]+ (PREDICATE|EFFECT) (SOURCE|SELECTED)$'
fib_re='^FIBER [^ ]+ -?[0-9][0-9.e+-]* IS EMPTY ! USE [0-9]+ (PREDICATE|EFFECT) (SOURCE|SELECTED)$'
use_re='^TRACE-USE [0-9]+ [a-z][0-9]+:[0-9]+ (PREDICATE|EFFECT) (SOURCE|SELECTED) [^ ]+$'
other_re='^(TRACE-ALIAS |s[0-9]+: )'
rc=0
for f in "$here"/trace/*.ano \
         "$root"/demos/1-selection/008-relationship-hops.ano \
         "$root"/demos/1-selection/015-keyed-hop.ano \
         "$root"/demos/2-effects/026-hop-after-despawn.ano \
         "$root"/demos/2-effects/027-hop-out-of-range.ano \
         "$root"/demos/9-nihongo/087-fallen-master.ano; do
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    echo "$line" | grep -qE "$rel_re|$fib_re|$use_re|$other_re" && continue
    echo "  ungrammatical: $line"
    rc=1
  done < <(trace_of "$f")
done
leg "line-grammar" $rc

exit $fail
