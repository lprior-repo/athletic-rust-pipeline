#!/usr/bin/env bash
set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/15}"
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"

mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
TEST_DIR=$(mktemp -d "$SCRATCH_STORE/test-XXXXXX")
trap 'rm -rf -- "$TEST_DIR"' EXIT
export TMPDIR="$TEST_DIR"

cd "$REPO_ROOT"
set +e
cargo test -p census-service --test backup_restore -- --nocapture 2>&1 | tee "$TEST_DIR/log.txt"
TEST_RC=$?
set -e
cd - >/dev/null

if [ "$TEST_RC" -ne 0 ]; then
    echo "FAIL: backup_restore exited $TEST_RC"
    grep "test result:" "$TEST_DIR/log.txt" || true
    exit 1
fi

if grep -qE '^test result: ok\. [1-9][0-9]* passed; 0 failed; 0 ignored;' "$TEST_DIR/log.txt" && ! grep -q '^SKIPPED:' "$TEST_DIR/log.txt"; then
    echo "PASS: Fjall backup/restore integrity"
    exit 0
else
    echo "FAIL: backup_restore had failures, skips, or unexpected output"
    grep "test result:" "$TEST_DIR/log.txt" || true
    exit 1
fi
