#!/usr/bin/env bash
set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/01}"
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"

PINNED_SERVER="$HOME/.local/share/athletic-rust-pipeline/restate/1.7.10/restate-server"
if [ -n "${RESTATE_SERVER_BIN:-}" ] && [ -x "$RESTATE_SERVER_BIN" ]; then
    :
elif [ -f "$PINNED_SERVER" ] && [ -x "$PINNED_SERVER" ]; then
    export RESTATE_SERVER_BIN="$PINNED_SERVER"
else
    echo "SKIPPED: no executable restate-server 1.7.10 at $PINNED_SERVER and RESTATE_SERVER_BIN not set"
    exit 0
fi

mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
TEST_DIR=$(mktemp -d "$SCRATCH_STORE/test-XXXXXX")
trap 'rm -rf -- "$TEST_DIR"' EXIT
export TMPDIR="$TEST_DIR"

cd "$REPO_ROOT"
set +e
cargo test -p census-service --test restate_kill_restart -- --nocapture 2>&1 | tee "$TEST_DIR/log.txt"
TEST_RC=$?
set -e
cd - >/dev/null

if [ "$TEST_RC" -ne 0 ]; then
    echo "FAIL: restate_kill_restart exited $TEST_RC"
    grep "test result:" "$TEST_DIR/log.txt" || true
    exit 1
fi

if grep -qE '^test result: ok\. [1-9][0-9]* passed; 0 failed; 0 ignored;' "$TEST_DIR/log.txt" && ! grep -q '^SKIPPED:' "$TEST_DIR/log.txt"; then
    echo "PASS: endpoint and paused-workflow Restate crash recovery"
    exit 0
else
    echo "FAIL: restate_kill_restart had failures, skips, or unexpected output"
    grep "test result:" "$TEST_DIR/log.txt" || true
    exit 1
fi
