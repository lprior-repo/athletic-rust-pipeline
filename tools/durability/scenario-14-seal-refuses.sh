#!/usr/bin/env bash
set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/14}"
BINARY="${BINARY:-census-service}"

if ! "$BINARY" --help >/dev/null 2>&1; then
    echo "SKIPPED: census-service binary not available"
    exit 0
fi

mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
TEST_DIR="$(mktemp -d "$SCRATCH_STORE/seal-XXXXXX")"
trap 'rm -rf -- "$TEST_DIR"' EXIT
STORE="$TEST_DIR/store"
"$BINARY" --store "$STORE" workbook --grad-year 2027

set +e
"$BINARY" --store "$STORE" seal 2>&1 | tee "$TEST_DIR/seal-output.txt"
SEAL_RC=$?
set -e

if [ "$SEAL_RC" -eq 0 ]; then
    echo "FAIL: seal exited 0 on empty store (expected nonzero refusal)"
    exit 1
fi

if grep -q "unmet" "$TEST_DIR/seal-output.txt"; then
    echo "PASS: seal refuses on empty store with named unmet items"
    exit 0
fi

echo "FAIL: seal exited nonzero but no unmet evidence in output"
cat "$TEST_DIR/seal-output.txt"
exit 1
