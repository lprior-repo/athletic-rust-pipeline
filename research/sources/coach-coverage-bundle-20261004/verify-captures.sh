#!/usr/bin/env bash
# Recompute every evidence digest under the bundle and report missing/stale entries.
# Usage: bash research/sources/coach-coverage-bundle-20261004/verify-captures.sh
# Schemas understood: sweep.json files[]; manifest.json captures[]/robots[]/main_verification.captures_sha256.
set -uo pipefail
BASE=$(cd "$(dirname "$0")" && pwd)
cd "$BASE"
total_missing=0; total_stale=0; total_dirs=0
for d in probes/*/ probes/sources/*/ extract/*/; do
  [ -d "$d" ] || continue
  m=""
  [ -f "$d/manifest.json" ] && m="$d/manifest.json"
  [ -z "$m" ] && [ -f "$d/sweep.json" ] && m="$d/sweep.json"
  files=$(cd "$d" && find raw captures robots -type f 2>/dev/null | sort)
  nf=$(printf '%s\n' "$files" | grep -c . || true)
  [ "$nf" -eq 0 ] && continue
  total_dirs=$((total_dirs+1))
  declare -A actual=()
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    actual["$f"]=$(sha256sum "$d/$f" | cut -d' ' -f1)
  done <<< "$files"
  if [ -z "$m" ]; then
    echo "$d: files=$nf manifest=NONE -> all missing"
    total_missing=$((total_missing+nf)); unset actual; continue
  fi
  declare -A recorded=()
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    recorded["${line% *}"]=${line##* }
  done < <(jq -r '
    (.files[]? | select(.file!=null and .sha256!=null) | "\(.file) \(.sha256)"),
    (.captures[]? | select(.file!=null and .sha256!=null) | "\(.file) \(.sha256)"),
    (.robots[]? | select(.capture!=null and .sha256!=null) | "\(.capture) \(.sha256)"),
    (.main_verification.captures_sha256? | to_entries[]? | "\(.key) \(if (.value|type)=="string" then .value else (.value.sha256 // "") end)")
  ' "$m" 2>/dev/null)
  miss=0; stale=0
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    want=${actual[$f]}
    got=${recorded[$f]:-}
    if [ -z "$got" ]; then miss=$((miss+1)); echo "  MISSING-DIGEST $d$f"
    elif [ "$got" != "$want" ]; then stale=$((stale+1)); echo "  STALE $d$f recorded=${got:0:12} actual=${want:0:12}"; fi
  done < <(printf '%s\n' "${!actual[@]}" | sort)
  echo "$d: files=$nf manifest=$(basename "$m") missing=$miss stale=$stale"
  total_missing=$((total_missing+miss)); total_stale=$((total_stale+stale))
  unset actual recorded
done
echo "SUMMARY dirs=$total_dirs missing_digests=$total_missing stale_digests=$total_stale"
