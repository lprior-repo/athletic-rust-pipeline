#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario}"
for tool in unshare mount curl jq python3 timeout; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "SKIPPED: missing Restate ENOSPC prerequisite $tool"
        exit 0
    fi
done
RESTATE_SERVER_BIN="${RESTATE_SERVER_BIN:-$HOME/.local/share/athletic-rust-pipeline/restate/1.7.10/restate-server}"
SERVE_BINARY="${SERVE_BINARY:-$REPO_ROOT/target/debug/census-serve}"
if [ ! -x "$RESTATE_SERVER_BIN" ] || [ ! -x "$SERVE_BINARY" ]; then
    echo "SKIPPED: Restate ENOSPC requires executable RESTATE_SERVER_BIN and SERVE_BINARY"
    exit 0
fi
if ! unshare --user --map-root-user --mount --pid --fork --propagation private true; then
    echo "SKIPPED: private user, mount and PID namespaces unavailable"
    exit 0
fi
mkdir -p "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
TEST_DIR="$(mktemp -d "$SCRATCH_STORE/restate-enospc-XXXXXX")"
trap 'rm -rf -- "$TEST_DIR"' EXIT
read -r ADMIN_PORT INGRESS_PORT NODE_PORT SERVE_PORT < <(python3 -c '
import socket
sockets = [socket.socket() for _ in range(4)]
for sock in sockets:
    sock.bind(("127.0.0.1", 0))
print(*(sock.getsockname()[1] for sock in sockets))
')
TMPFS_DIR="$TEST_DIR/tmpfs"
FJALL_DIR="$TEST_DIR/endpoint-store"
mkdir -p "$TMPFS_DIR" "$FJALL_DIR"
export RESTATE_SERVER_BIN SERVE_BINARY ADMIN_PORT INGRESS_PORT NODE_PORT SERVE_PORT
export TEST_DIR TMPFS_DIR FJALL_DIR
set +e
timeout --signal=TERM --kill-after=10s 240s unshare --user --map-root-user --mount --pid --fork --kill-child=KILL --propagation private bash "$REPO_ROOT/tools/durability/restate-enospc-probe.sh" 2>&1 | tee "$TEST_DIR/probe.log"
RC=$?
set -e
if [ "$RC" -ne 0 ]; then
    echo "FAIL: isolated Restate ENOSPC probe exited $RC"
    exit 1
fi
if grep -qx 'PASS: acknowledged Restate workflow survives verified ENOSPC and restart' "$TEST_DIR/probe.log"; then
    echo "PASS: isolated Restate ENOSPC recovery"
elif grep -q '^SKIPPED:' "$TEST_DIR/probe.log"; then
    exit 0
else
    echo "FAIL: Restate ENOSPC probe produced no verified result"
    exit 1
fi
