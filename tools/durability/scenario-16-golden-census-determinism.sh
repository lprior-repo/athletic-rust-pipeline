#!/usr/bin/env bash
set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/16}"
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"

mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
TEST_DIR=$(mktemp -d "$SCRATCH_STORE/test-XXXXXX")
printf 'EVIDENCE: %s\n' "$TEST_DIR"
export TMPDIR="$TEST_DIR"

cd "$REPO_ROOT"
set +e
cargo test -p census-service --test parity_pipeline rebuilding_the_fixture_store_reproduces_semantics_not_publication_identity -- --exact --nocapture 2>&1 | tee "$TEST_DIR/log.txt"
TEST_RC=$?
set -e
cd - >/dev/null

if [ "$TEST_RC" -ne 0 ]; then
    echo "FAIL: parity_pipeline exited $TEST_RC"
    grep "test result:" "$TEST_DIR/log.txt" || true
    exit 1
fi

if grep -qE '^test result: ok\. [1-9][0-9]* passed; 0 failed; 0 ignored;' "$TEST_DIR/log.txt" && ! grep -q '^SKIPPED:' "$TEST_DIR/log.txt"; then
    echo "PASS: fixture semantic rebuild parity; other scenario-16 obligations are not certified by this regression"
    exit 0
else
    echo "FAIL: parity_pipeline had failures, skips, or unexpected output"
    grep "test result:" "$TEST_DIR/log.txt" || true
    exit 1
fi
