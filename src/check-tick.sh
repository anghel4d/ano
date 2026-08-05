#!/usr/bin/env bash
# Re-derives every demo's tick-dynamics class against src/tick-classes.txt and runs the
# absorption/saturation witness worlds. The tick is Kore's n staged headlessly: registry
# retargeted to a scratch copy (sidecar beside it), out/expect/expect-n pins stripped,
# `steel --run --save --label`. A class mismatch is a real finding: the test was never good,
# a grammar intentionally changed, or the implementation broke. ANO_TICK_SATURATE=1 also
# runs the measured `saturate=K` fixed points (~1000 ticks each). Proofs: proofs/tick-induction.md.
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
manifest="$here/tick-classes.txt"
work="$(mktemp -d "${TMPDIR:-/tmp}/ano-tick-XXXXXX")"
trap 'rm -rf "$work"' EXIT
fail=0

# stage_tick <demo> <world> <out>: the demo with its registry retargeted and pins dropped.
stage_tick() {
  awk -v w="$2" '
    { line=$0; sub(/^[ \t]+/, "", line) }
    !done && line ~ /^--! registry / { print "--! registry " w; done=1; next }
    line ~ /^--![ \t]*(out|expect|expect-n)([ \t]|$)/ { next }
    { print }
  ' "$1" > "$3"
  grep -q "^--! registry $2\$" "$3"
}

# run_ticks <play> <K>: tick K times, hashes into HASH[1..K]; nonzero + TICK_ERR on a failed tick.
run_ticks() {
  local play="$1" limit="$2" k
  HASH=()
  for ((k = 1; k <= limit; k++)); do
    if ! "$STEEL" --run --save "$play/world.reg" --label --label "$play/next.ano" >"$play/tick.out" 2>&1; then
      TICK_ERR="tick $k failed: $(tail -1 "$play/tick.out" | cut -c1-100)"
      return 1
    fi
    HASH[$k]="$(sha256sum "$play/world.reg" | cut -c1-16)"
  done
}

listed=""
while read -r stem class a1 _; do
  case "$stem" in ''|'#'*) continue ;; esac
  listed="$listed $stem "
  file="$(find "$repo/demos" -name "$stem.ano" | head -1)"
  if [ -z "$file" ]; then
    echo "FAIL-missing $stem (in manifest, not in demos/)"
    fail=1
    continue
  fi
  demo_dir="$(dirname "$file")"
  regline="$(grep -m1 '^--! registry ' "$file" | awk '{print $3}')"
  case "$regline" in */*|*.reg) : ;; *) regline="$regline.reg" ;; esac
  case "$regline" in /*) src="$regline" ;; *) src="$demo_dir/$regline" ;; esac
  play="$work/$stem"
  mkdir -p "$play"
  cp "$src" "$play/world.reg" || { echo "FAIL-registry $stem"; fail=1; continue; }
  [ -f "$src.aliases" ] && cp "$src.aliases" "$play/world.reg.aliases"
  stage_tick "$file" "$play/world.reg" "$play/next.ano" || { echo "FAIL-stage  $stem"; fail=1; continue; }
  # per-class re-derivation; args re-read from the manifest line
  line="$(grep -m1 "^$stem " "$manifest")"
  set -- $line
  shift
  verdict="ok"
  case "$1" in
    fixed)
      K="$2"
      if ! run_ticks "$play" "$K"; then verdict="$TICK_ERR"
      elif [ "${HASH[$K]}" != "${HASH[$((K - 1))]}" ]; then verdict="not fixed at tick $K"
      elif [ "$K" -ge 3 ] && [ "${HASH[$((K - 1))]}" = "${HASH[$((K - 2))]}" ]; then verdict="fixed earlier than tick $K"
      fi
      ;;
    cycle)
      A="$2"; B="$3"
      if ! run_ticks "$play" "$B"; then verdict="$TICK_ERR"
      elif [ "${HASH[$B]}" != "${HASH[$A]}" ]; then verdict="no cycle $A->$B"
      else
        for ((i = A; i < B; i++)); do
          for ((j = i + 1; j < B; j++)); do
            [ "${HASH[$i]}" = "${HASH[$j]}" ] && verdict="shorter cycle $i->$j inside $A->$B"
          done
        done
      fi
      ;;
    evolving)
      K=4
      sat=""
      [ "${2:-}" ] && case "$2" in saturate=*) sat="${2#saturate=}" ;; esac
      if [ -n "$sat" ] && [ "${ANO_TICK_SATURATE:-0}" = "1" ]; then K="$sat"; fi
      if ! run_ticks "$play" "$K"; then verdict="$TICK_ERR"
      else
        for ((i = 1; i <= 4 && i <= K; i++)); do
          for ((j = i + 1; j <= 4 && j <= K; j++)); do
            [ "${HASH[$i]}" = "${HASH[$j]}" ] && verdict="not evolving: tick $i repeats at $j"
          done
        done
        if [ -n "$sat" ] && [ "$K" = "$sat" ]; then
          [ "${HASH[$K]}" != "${HASH[$((K - 1))]}" ] && verdict="did not saturate at tick $sat"
          [ "${HASH[$((K - 1))]}" = "${HASH[$((K - 2))]}" ] && verdict="saturated earlier than tick $sat"
        fi
      fi
      ;;
    *) verdict="unknown class '$1'" ;;
  esac
  if [ "$verdict" = "ok" ]; then
    echo "ok-class    $stem ($line)" | sed "s|$stem ||2"
  else
    echo "FAIL-class  $stem: $verdict"
    fail=1
  fi
done < "$manifest"

# every active non-twin demo must be classified; twins ride their ASCII stem
while IFS= read -r -d '' file; do
  number="$(ano_demo_number "$file")" || continue
  ano_demo_quarantined "$number" && continue
  stem="$(basename "$file" .ano)"
  case "$stem" in *-nihongo) stem="${stem%-nihongo}" ;; esac
  case "$listed" in *" $stem "*) ;; *)
    echo "FAIL-unclassified $file (add its measured class to src/tick-classes.txt with its invariant)"
    fail=1
  ;; esac
done < <(find "$repo/demos" -name '*.ano' -print0 | sort -z)

# witness worlds: the absorbing semantics of proofs/tick-induction.md theorems 3 and 4, every run
wit="$work/witness"
mkdir -p "$wit"
printf 'n 2\ncol alive bool 1 1\ncol gold num 9223372036854775808 4611686018427387904\n' > "$wit/absorb.reg"
printf -- '--! registry %s/absorb.reg\nAlive , Gold += 1000\n' "$wit" > "$wit/absorb.ano"
if "$STEEL" --run --save "$wit/absorb.reg" --label --label "$wit/absorb.ano" >/dev/null 2>&1 \
  && grep -q '^col gold num 9\.223372036854776e+18 4\.611686018427389e+18$' "$wit/absorb.reg"; then
  echo "ok-witness  additive absorption (2^63 frozen, 2^62 climbs by the rounded step)"
else
  echo "FAIL-witness additive absorption"
  fail=1
fi
printf 'n 1\ncol alive bool 1\ncol gold num inf\n' > "$wit/inf.reg"
printf -- '--! registry %s/inf.reg\nAlive , Gold *= 2\nAlive , Gold += 1000\n' "$wit" > "$wit/inf.ano"
if "$STEEL" --run --save "$wit/inf.reg" --label --label "$wit/inf.ano" >/dev/null 2>&1 \
  && h1="$(sha256sum "$wit/inf.reg")" \
  && "$STEEL" --run --save "$wit/inf.reg" --label --label "$wit/inf.ano" >/dev/null 2>&1 \
  && [ "$h1" = "$(sha256sum "$wit/inf.reg")" ] && grep -q '^col gold num inf$' "$wit/inf.reg"; then
  echo "ok-witness  infinity absorbs *= and += and round-trips save"
else
  echo "FAIL-witness infinity absorption"
  fail=1
fi
printf 'n 1\ncol alive bool 1\ncol gold num 0\n' > "$wit/zero.reg"
printf -- '--! registry %s/zero.reg\nAlive , Gold /= 2\n' "$wit" > "$wit/zero.ano"
if "$STEEL" --run --save "$wit/zero.reg" --label --label "$wit/zero.ano" >/dev/null 2>&1 \
  && h1="$(sha256sum "$wit/zero.reg")" \
  && "$STEEL" --run --save "$wit/zero.reg" --label --label "$wit/zero.ano" >/dev/null 2>&1 \
  && [ "$h1" = "$(sha256sum "$wit/zero.reg")" ]; then
  echo "ok-witness  zero absorbs /="
else
  echo "FAIL-witness zero absorption"
  fail=1
fi

exit $fail
