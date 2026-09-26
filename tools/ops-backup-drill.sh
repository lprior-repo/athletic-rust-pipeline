#!/usr/bin/env bash
set -euo pipefail

STORE="${1:?Usage: tools/ops-backup-drill.sh <store-dir>}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY="${BINARY:-$REPO_ROOT/target/debug/census-service}"
command -v jq >/dev/null
[[ -d "$STORE/fjall" ]] || { echo 'FAIL: source has no Fjall database' >&2; exit 1; }
DRILL_DIR="$(mktemp -d "${TMPDIR:-/tmp}/athletic-backup-drill.XXXXXXXX")"
trap 'rm -rf -- "$DRILL_DIR"' EXIT

"$BINARY" --store "$STORE" store-backup --to "$DRILL_DIR/backup"
"$BINARY" --store "$DRILL_DIR/cli" store-restore \
    --from "$DRILL_DIR/backup" --to "$DRILL_DIR/restored"

"$BINARY" --store "$DRILL_DIR/restored" store-integrity | tee "$DRILL_DIR/integrity"
grep -qx $'ok\ttrue' "$DRILL_DIR/integrity"
"$BINARY" --store "$DRILL_DIR/restored" fjall-stats > "$DRILL_DIR/stats"
jq -Rn '[inputs | split("\t") | select(length == 2) |
    select(.[0] != "store" and .[0] != "observations" and
           .[0] != "bytes_on_disk" and .[0] != "store_bytes") |
    {key: .[0], value: (.[1] | tonumber)}] | from_entries' \
    < "$DRILL_DIR/stats" > "$DRILL_DIR/counts.json"
jq -e --slurpfile actual "$DRILL_DIR/counts.json" '.tables == $actual[0]' \
    "$DRILL_DIR/backup/backup.json" >/dev/null

"$BINARY" --store "$DRILL_DIR/restored" consolidate
"$BINARY" --store "$DRILL_DIR/restored" report
jq -e '.scope == "all_sources" and (.totals | type == "object")' \
    "$DRILL_DIR/restored/out/report.json" >/dev/null
jq -S 'del(.store_dir, .generated_on)' "$DRILL_DIR/restored/out/report.json" \
    > "$DRILL_DIR/census-first.json"
"$BINARY" --store "$DRILL_DIR/restored" report
jq -S 'del(.store_dir, .generated_on)' "$DRILL_DIR/restored/out/report.json" \
    > "$DRILL_DIR/census-reopened.json"
cmp "$DRILL_DIR/census-first.json" "$DRILL_DIR/census-reopened.json"
printf 'PASS: verified manifest digests, exact table counts, integrity, consolidation and census across reopen\n'
