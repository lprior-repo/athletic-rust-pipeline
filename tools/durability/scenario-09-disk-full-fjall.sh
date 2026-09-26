#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROBE_BIN="$REPO_ROOT/target/debug/examples/enospc"

if ! command -v unshare >/dev/null 2>&1; then
    echo "SKIPPED: unshare not available"
    exit 0
fi

if ! unshare --user --map-root-user --mount --propagation private true 2>/dev/null; then
    echo "SKIPPED: unprivileged user+mount namespace not supported"
    exit 0
fi

echo "Building ENOSPC probe..."
if ! (cd "$REPO_ROOT" && cargo build --example enospc -p census-store --quiet 2>&1); then
    echo "FAIL: probe compile failure"
    exit 1
fi

PROBE_BIN="$REPO_ROOT/target/debug/examples/enospc"
if [ ! -x "$PROBE_BIN" ]; then
    echo "FAIL: probe binary not found"
    exit 1
fi

SCRATCH_DIR="$(mktemp -d "${TMPDIR:-/tmp}/enospc-09-XXXXXX")"

cleanup() {
    if [ -n "$SCRATCH_DIR" ] && [ -d "$SCRATCH_DIR" ]; then
        rm -rf "$SCRATCH_DIR"
    fi
}
trap cleanup EXIT INT TERM

TMPFS_MNT="$SCRATCH_DIR/tmpfs"
STORE_DIR="$TMPFS_MNT/store"

unshare --user --map-root-user --mount --propagation private -- \
    bash -c '
        mkdir -p '"$TMPFS_MNT"'
        cd '"$REPO_ROOT"'

        if ! mount -t tmpfs -o size=64M tmpfs '"$TMPFS_MNT"' 2>/dev/null; then
            echo "SKIPPED: mount tmpfs not supported inside unprivileged namespace"
            exit 0
        fi

        exec '"$PROBE_BIN"' '"$STORE_DIR"'
    ' > "$SCRATCH_DIR/probe.out" 2>&1 || RC=$?

RC=${RC:-0}
OUTPUT="$(cat "$SCRATCH_DIR/probe.out")"

if ! echo "$OUTPUT" | grep -q "^SKIPPED:"; then
    echo "$OUTPUT"
fi

if echo "$OUTPUT" | grep -q "^SKIPPED:"; then
    echo "$OUTPUT" | grep "^SKIPPED:" | head -1
    exit 0
fi

if [ "$RC" -ne 0 ]; then
    echo "$OUTPUT"
    echo "FAIL: probe exited with code $RC"
    exit 1
fi

if echo "$OUTPUT" | grep -q "^FAIL:"; then
    echo "$OUTPUT"
    echo "FAIL: $(echo "$OUTPUT" | grep "^FAIL:" | head -1)"
    exit 1
fi

if echo "$OUTPUT" | grep -q "^PASS:"; then
    echo "$OUTPUT" | grep "^PASS:" | head -1
    echo "PASS: bounded tmpfs ENOSPC drill verified"
    exit 0
fi

echo "$OUTPUT"
echo "FAIL: probe completed without PASS or SKIPPED marker"
exit 1
