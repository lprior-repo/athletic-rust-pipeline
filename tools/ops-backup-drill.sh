#!/usr/bin/env bash
# cold-copy backup drill: copy a store, open the copy, compare fjall-stats +
# consolidate/report counts. Emits PASS/FAIL with raw output.
#
# Failure modes documented in docs/OPERATIONS.md §Backups:
#   * Lock held — Fjall returns a lock error; stop the source unit first.
#   * Cache absent — a store copy without the cache costs re-fetching only.
#   * Marker present — legacy import marker on a re-import causes duplicate
#     observations until the marker is cleared; see OPERATIONS.md §Restore.
#
# Usage: tools/ops-backup-drill.sh <store-dir>
set -euo pipefail

STORE="${1:?Usage: tools/ops-backup-drill.sh <store-dir>}"
DRILL_DIR="$(mktemp -d)"
BINARY="${BINARY:-target/debug/census-service}"

fail() { echo ""; echo "FAIL: $*"; cleanup; exit 1; }
pass() { echo ""; echo "PASS: $*"; cleanup; exit 0; }

cleanup() { rm -rf "$DRILL_DIR"; }
trap cleanup EXIT

# --- Step 1: copy the store ---
echo "--- step 1: cold copy ---"
cp -a "$STORE" "$DRILL_DIR/source"
cp -a "$STORE" "$DRILL_DIR/copy"
echo "  source: $DRILL_DIR/source"
echo "  copy:   $DRILL_DIR/copy"
echo ""

# --- Step 2: fjall-stats on both ---
echo "--- step 2: fjall-stats comparison ---"
FJALL_SRC="$($BINARY --store "$DRILL_DIR/source" fjall-stats 2>&1)" || fail "fjall-stats on source failed"
FJALL_CPY="$($BINARY --store "$DRILL_DIR/copy" fjall-stats 2>&1)" || fail "fjall-stats on copy failed"
echo "source:"
echo "$FJALL_SRC"
echo ""
echo "copy:"
echo "$FJALL_CPY"
echo ""

# Compare observation counts
# Anchor the key: `source_observations` also ends in `observations`, and matching both made
# `SRC_OBS` a two-line value, which turned every count comparison below into a vacuous pass.
SRC_OBS=$(echo "$FJALL_SRC" | awk -F'\t' '$1 == "observations" {print $2}')
CPY_OBS=$(echo "$FJALL_CPY" | awk -F'\t' '$1 == "observations" {print $2}')
if [ "$SRC_OBS" != "$CPY_OBS" ]; then
    fail "observation count mismatch: source=$SRC_OBS copy=$CPY_OBS"
fi
echo "  observations match: $SRC_OBS"

# --- Step 3: consolidate on the copy ---
echo "--- step 3: consolidate on copy ---"
CONSOLIDATE="$($BINARY --store "$DRILL_DIR/copy" consolidate 2>&1)" || fail "consolidate on copy failed"
echo "  $CONSOLIDATE"
echo ""

# --- Step 4: compare report output (if source has data) ---
echo "--- step 4: report comparison (if data present) ---"
if [ "$SRC_OBS" -gt 0 ]; then
    REPORT_SRC="$($BINARY --store "$DRILL_DIR/source" report --print 2>&1)" || fail "report on source failed"
    REPORT_CPY="$($BINARY --store "$DRILL_DIR/copy" report --print 2>&1)" || fail "report on copy failed"
    SRC_ROWS=$(echo "$REPORT_SRC" | grep -c '^[A-Za-z]' 2>/dev/null || true)
    CPY_ROWS=$(echo "$REPORT_CPY" | grep -c '^[A-Za-z]' 2>/dev/null || true)
    echo "  source report rows: $SRC_ROWS"
    echo "  copy   report rows: $CPY_ROWS"
    echo ""
    if [ "$SRC_ROWS" != "$CPY_ROWS" ]; then
        fail "report row count mismatch: source=$SRC_ROWS copy=$CPY_ROWS"
    fi
    echo "  report row counts match: $SRC_ROWS"
    echo ""
else
    echo "  no data to compare (0 observations), skipping report diff."
    echo ""
fi

pass "backup drill completed successfully"
