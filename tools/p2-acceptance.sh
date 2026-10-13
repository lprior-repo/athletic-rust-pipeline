#!/usr/bin/env bash
# Acceptance verification for the five v20-v29 P2 beads (TFRRS-CONVERSION-01, GATE-RATCHET-01,
# IDENTITY-CASE-01, ADDRESS-CAPTURE-01, ADDRESS-CAPTURE-02) plus the TEST-FIXTURE-01 prerequisite.
#
# Every lane runs through tools/moon-local only; nothing here builds, tests or lints outside Moon.
#
#   tools/p2-acceptance.sh                 run the fast acceptance lanes
#   tools/p2-acceptance.sh <lane> [...]    run only the named lanes
#   tools/p2-acceptance.sh --gate          add the selected strict-ratchet gate lane (slow)
#   tools/p2-acceptance.sh --fresh         add the fresh-checkout test-compile lane (slow)
#   tools/p2-acceptance.sh --list          print the lane names
#
# Each lane prints its nextest summary and PASS/FAIL; the script exits non-zero if any lane failed.
set -uo pipefail
root=$(realpath "$(dirname "${BASH_SOURCE[0]}")/..")
cd "$root"
moon="$root/tools/moon-local"

declare -A lane=(
  [crawl-tfrrs]="pipeline:tests-scoped -- -p census-crawl -E test(conversion)+test(tfrrs)"
  [report-selection]="pipeline:tests-scoped -- -p census-report -E test(ties)+test(adjustment)"
  [reconcile-identity]="pipeline:tests-scoped -- -p census-reconcile -E test(supersed)"
  [domain-identity]="pipeline:tests-scoped -- -p census-domain -E test(identity)"
  [service-capture]="pipeline:tests-scoped -- -p census-service -E test(capture)+test(replay)"
  [service-provenance]="pipeline:tests-scoped -- -p census-service -E test(provenance)"
  [coach-lane-fixtures]="pipeline:tests-scoped -- -p census-service -E test(coach_lane)"
  [xtask-ratchet]="pipeline:tests-scoped -- -p xtask -E test(ratchet)+test(baseline)"
  [gate-ratchet]="pipeline:gate -- --lane ratchet"
)
fast=(crawl-tfrrs report-selection reconcile-identity domain-identity service-capture service-provenance coach-lane-fixtures xtask-ratchet)
selected=()
with_gate=0
with_fresh=0
for arg in "$@"; do
  case "$arg" in
    --gate) with_gate=1 ;;
    --fresh) with_fresh=1 ;;
    --list) printf '%s\n' "${!lane[@]}" | sort; exit 0 ;;
    *) selected+=("$arg") ;;
  esac
done
if (( ${#selected[@]} == 0 )); then selected=("${fast[@]}"); fi
failures=()

run_lane() {
  local name=$1 line=$2 output status summary
  printf '== lane %s: tools/moon-local run %s\n' "$name" "$line"
  read -r -a argv <<<"$line"
  output=$(env -u CI "$moon" run "${argv[@]}" 2>&1)
  status=$?
  summary=$(printf '%s\n' "$output" | grep -E 'Summary \[' | tail -n 1)
  if (( status == 0 )); then
    printf 'PASS %s: %s\n' "$name" "${summary:-no summary line}"
  else
    printf 'FAIL %s (exit %d): %s\n' "$name" "$status" "${summary:-no summary line}"
    printf '%s\n' "$output" | grep -E 'FAIL|error\[|^error' | head -n 12
    failures+=("$name")
  fi
}

for name in "${selected[@]}"; do
  if [[ -z "${lane[$name]:-}" ]]; then
    printf 'unknown lane: %s (see --list)\n' "$name" >&2
    failures+=("$name:unknown")
    continue
  fi
  run_lane "$name" "${lane[$name]}"
done

if (( with_gate )); then
  log=$(mktemp /tmp/p2-gate-ratchet-XXXXXX.log)
  printf '== lane gate-ratchet: tools/moon-local run %s (log %s)\n' "${lane[gate-ratchet]}" "$log"
  if env -u CI "$moon" run pipeline:gate -- --lane ratchet >"$log" 2>&1; then
    printf 'PASS gate-ratchet: %s\n' "$(grep -E 'gate: (PASS|FAIL)' "$log" | tail -n 1)"
  else
    printf 'FAIL gate-ratchet (exit %d): %s\n' "$?" "$(grep -E 'gate: (PASS|FAIL)' "$log" | tail -n 1)"
    tail -n 20 "$log"
    failures+=("gate-ratchet")
  fi
fi

if (( with_fresh )); then
  wt=$(mktemp -d /tmp/p2-fresh-XXXXXX)
  rev=$(git rev-parse HEAD)
  printf '== lane fresh-checkout: worktree at %s in %s\n' "$rev" "$wt"
  if git worktree add --detach "$wt" "$rev" >/dev/null 2>&1; then
    if output=$(cd "$wt" && env -u CI "$moon" run pipeline:tests-scoped -- -p census-service -E 'test(coach_lane)' 2>&1); status=$?; printf '%s\n' "$output" | tail -n 40; test "$status" -eq 0; then
      printf 'PASS fresh-checkout: committed fixtures compile and validate without developer-local state\n'
    else
      printf 'FAIL fresh-checkout\n'
      failures+=("fresh-checkout")
    fi
    git worktree remove --force "$wt"
  else
    printf 'FAIL fresh-checkout: git worktree add failed\n'
    failures+=("fresh-checkout")
  fi
fi

if (( ${#failures[@]} > 0 )); then
  printf 'p2-acceptance: %d lane(s) failed: %s\n' "${#failures[@]}" "${failures[*]}"
  exit 1
fi
printf 'p2-acceptance: all selected lanes passed (%d)\n' "${#selected[@]}"
