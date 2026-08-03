#!/usr/bin/env bash
# Runs every explanatory BQN witness under demos/. Inputs: none (env BQN overrides the
# interpreter, default: bqn on PATH). Output: one line per file (ok/skip/FAIL). Exit:
# nonzero iff any active witness fails.
set -u
BQN="${BQN:-bqn}"
here="$(cd "$(dirname "$0")" && pwd)"
root="$(git -C "$here" rev-parse --show-toplevel 2>/dev/null || echo "$here/..")"
fail=0

# The set of decommissioned demo numbers is read from the quarantine block in todo/TODO.md,
# which is the single authority for it, and is shared by every harness rather than restated
# per script. A demo whose leading three-digit number is decommissioned is skipped, and each
# skip is announced on its own line, so an exclusion is always visible and never silent.
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
    echo "check: cannot read the quarantine block from todo/TODO.md" >&2
    exit 3
    ;;
esac
qset=" $(printf '%s' "$qlist" | tr '\n' ' ') "

while IFS= read -r f; do
  base="$(basename "$f")"
  case "$base" in
    [0-9][0-9][0-9]-*) ;;
    *)
      echo "check: $f falls outside the three-digit accounting" >&2
      exit 3
      ;;
  esac
  case "$qset" in
    *" $(printf '%s' "$base" | cut -c1-3) "*)
      echo "skip $f (decommissioned)"
      continue
      ;;
  esac
  if "$BQN" "$f" >/dev/null 2>&1; then
    echo "ok   $f"
  else
    echo "FAIL $f"
    fail=1
  fi
done < <(find "$here" -name '*.bqn' | sort)
exit $fail
