#!/usr/bin/env bash
# Shared parser for the one authoritative demo-quarantine block in todo/TODO.md.

ano_quarantine_load() {
  local root="$1" raw token first last value key expected
  declare -gA ANO_QUARANTINE=()
  expected="$(awk '/^Exactly [0-9]+ demo numbers are decommissioned/ { print $2; exit }' "$root/todo/TODO.md")"
  [[ "$expected" =~ ^[0-9]+$ ]] || { echo "quarantine: todo/TODO.md has no declared demo count" >&2; return 1; }
  raw="$(awk '
    /^Exactly [0-9]+ demo numbers are decommissioned/ { seen=1; next }
    seen && /^```text$/ { block=1; next }
    block && /^```$/ { exit }
    block { print }
  ' "$root/todo/TODO.md")"
  [ -n "$raw" ] || { echo "quarantine: todo/TODO.md has no demo block" >&2; return 1; }
  raw="${raw//,/ }"
  raw="${raw//–/-}"
  for token in $raw; do
    if [[ "$token" =~ ^([0-9]{3})$ ]]; then
      first="${BASH_REMATCH[1]}"
      last="$first"
    elif [[ "$token" =~ ^([0-9]{3})-([0-9]{3})$ ]]; then
      first="${BASH_REMATCH[1]}"
      last="${BASH_REMATCH[2]}"
    else
      echo "quarantine: malformed token '$token'" >&2
      return 1
    fi
    if (( 10#$first > 10#$last )); then
      echo "quarantine: reversed range '$token'" >&2
      return 1
    fi
    for ((value=10#$first; value<=10#$last; value++)); do
      printf -v key '%03d' "$value"
      if [[ -n "${ANO_QUARANTINE[$key]+set}" ]]; then
        echo "quarantine: duplicate demo '$key'" >&2
        return 1
      fi
      ANO_QUARANTINE[$key]=1
    done
  done
  if [ "${#ANO_QUARANTINE[@]}" -ne "$expected" ]; then
    echo "quarantine: expected $expected demo numbers, found ${#ANO_QUARANTINE[@]}" >&2
    return 1
  fi
}

ano_demo_number() {
  local base
  base="$(basename "$1")"
  if [[ "$base" =~ ^([0-9]{3})- ]]; then
    printf '%s\n' "${BASH_REMATCH[1]}"
    return 0
  fi
  return 1
}

ano_demo_quarantined() {
  [[ -n "${ANO_QUARANTINE[$1]+set}" ]]
}
