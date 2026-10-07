#!/usr/bin/env bash
set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/17}"
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"

mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
EVIDENCE_DIR=$(mktemp -d "$SCRATCH_STORE/scenario-17-XXXXXX")
echo "EVIDENCE: $EVIDENCE_DIR"
export TMPDIR="$EVIDENCE_DIR/tmp"
mkdir -p "$TMPDIR"

cd "$REPO_ROOT"
set +e
tools/moon-cargo test -p census-service --test recovery -- --nocapture 2>&1 | tee "$EVIDENCE_DIR/log.txt"
TEST_RC=$?
set -e
cd - >/dev/null

if [ "$TEST_RC" -ne 0 ]; then
    echo "FAIL: recovery exited $TEST_RC"
    grep "test result:" "$EVIDENCE_DIR/log.txt" || true
    exit 1
fi

if grep -qE '^test result: ok\. [1-9][0-9]* passed; 0 failed; 0 ignored;' "$EVIDENCE_DIR/log.txt" && ! grep -q '^SKIPPED:' "$EVIDENCE_DIR/log.txt"; then
    echo "PASS: recovery test suite"
    exit 0
else
    echo "FAIL: recovery had failures, skips, or unexpected output"
    grep "test result:" "$EVIDENCE_DIR/log.txt" || true
    exit 1
fi
