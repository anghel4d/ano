#!/usr/bin/env bash
# Runs every demo, compares each Japanese twin's emitted BQN with its ASCII twin, then runs
# the refusal battery. STEEL overrides the binary; without it a fresh release build is made
# every run, so a stale binary is never tested. Exits nonzero on any failure.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
root="$(git -C "$here" rev-parse --show-toplevel 2>/dev/null || echo "$here/..")"
if [ -z "${STEEL:-}" ]; then
  tdir="${CARGO_TARGET_DIR:-$root/target}"
  case "$tdir" in /*) ;; *) tdir="$root/$tdir" ;; esac
  ( cd "$root" && cargo build --release -p steel ) || exit 1
  STEEL="$tdir/release/steel"
fi
fail=0

# The set of decommissioned demo numbers is read from the quarantine block in todo/TODO.md,
# which is the single authority for it, and is shared by every harness rather than restated
# per script. A demo whose leading three-digit number is decommissioned is skipped, in both
# the run pass and the Japanese-twin emission comparison, and each skip is announced on its
# own line, so an exclusion is always visible and never silent.
# Every demo file is accounted for: each matches the three-digit naming convention and lands
# in exactly one of active or decommissioned, and the harness fails if any file falls outside
# that accounting. A decommissioned number carrying no file of a given kind is normal, not a gap.

# The fenced block under "## Demo evidence quarantine", expanded: comma-separated three-digit
# numbers and inclusive A–B (en dash) ranges, one %03d number per output line. A token the
# grammar does not own prints MALFORMED so the caller can refuse rather than under-skip.
quarantined_numbers() {
  awk '/^## Demo evidence quarantine/{sect=1; next}
       sect && /^```/{fence=!fence; if (!fence) exit; next}
       sect && fence {print}' "$root/todo/TODO.md" |
    tr ',' '\n' | tr -d ' \t\r' |
    while IFS= read -r tok; do
      [ -n "$tok" ] || continue
      case "$tok" in
        *–*) a="${tok%%–*}" b="${tok##*–}" ;;
        *) a="$tok" b="$tok" ;;
      esac
      case "$a$b" in
        '' | *[!0-9]*)
          echo "MALFORMED $tok"
          return
          ;;
      esac
      i=$((10#$a))
      while [ "$i" -le $((10#$b)) ]; do
        printf '%03d\n' "$i"
        i=$((i + 1))
      done
    done
}

qlist="$(quarantined_numbers)"
case "$qlist" in
  '' | *MALFORMED*)
    echo "check-ano: cannot read the quarantine block from todo/TODO.md" >&2
    exit 3
    ;;
esac
qset=" $(printf '%s' "$qlist" | tr '\n' ' ') "

# Inputs: a demo path. Output: 0 announced skip, 1 active; exits 3 on a file outside the
# three-digit accounting. Callers that must stay quiet redirect the announcement themselves.
triage() {
  base="$(basename "$1")"
  case "$base" in
    [0-9][0-9][0-9]-*) ;;
    *)
      echo "check-ano: $1 falls outside the three-digit accounting" >&2
      exit 3
      ;;
  esac
  case "$qset" in
    *" $(printf '%s' "$base" | cut -c1-3) "*)
      echo "skip $1 (decommissioned)"
      return 0
      ;;
  esac
  return 1
}

while IFS= read -r f; do
  triage "$f" && continue
  if "$STEEL" --run "$f" >/dev/null 2>&1; then
    echo "ok   $f"
  else
    echo "FAIL $f"
    fail=1
  fi
done < <(find "$root/demos" -name '*.ano' | sort)

while IFS= read -r f; do
  triage "$f" >/dev/null && continue
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
