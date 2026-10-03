#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROBE_BIN="$REPO_ROOT/target/debug/examples/enospc"

for tool in unshare mount findmnt timeout cp; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'SKIPPED: %s not available\n' "$tool"
        exit 0
    fi
done
if ! unshare --user --map-root-user --mount --propagation private true 2>/dev/null; then
    echo "SKIPPED: unprivileged user+mount namespace not supported"
    exit 0
fi

if ! (cd "$REPO_ROOT" && cargo build --example enospc -p census-store --quiet); then
    echo "FAIL: probe compile failure"
    exit 1
fi
if [[ ! -x "$PROBE_BIN" ]]; then
    echo "FAIL: probe binary not found"
    exit 1
fi

SCRATCH_DIR="$(mktemp -d "${SCRATCH_STORE:-${TMPDIR:-/tmp}}/enospc-09-XXXXXX")"
printf 'EVIDENCE: %s\n' "$SCRATCH_DIR"
RC=0
timeout --signal=TERM 300s unshare --user --map-root-user --mount --propagation private -- \
    bash -c '
        set -uo pipefail
        artifact=$1
        probe=$2
        mountpoint="$artifact/tmpfs"
        store="$mountpoint/store"
        probe_pid=""
        preserve() {
            local code=$?
            trap - EXIT INT TERM
            if [[ -n "$probe_pid" ]]; then
                if kill -0 "$probe_pid" 2>/dev/null; then
                    if ! kill -TERM "$probe_pid"; then
                        echo "FAIL: owned probe termination failed"
                        code=1
                    fi
                fi
                local stopped=0
                wait "$probe_pid" || stopped=$?
                printf "CLEANUP: owned probe reaped with exit %s\n" "$stopped"
            fi
            if [[ -d "$store" ]]; then
                if ! cp -a -- "$store" "$artifact/preserved-store"; then
                    echo "FAIL: cold fault-store preservation failed"
                    code=1
                fi
            fi
            printf "CLEANUP: namespace exit %s; artifact retained %s\n" "$code" "$artifact"
            exit "$code"
        }
        trap preserve EXIT
        trap "exit 130" INT
        trap "exit 143" TERM
        mkdir -- "$mountpoint" || exit 1
        if ! mount -t tmpfs -o size=64M tmpfs "$mountpoint"; then
            echo "SKIPPED: mount tmpfs not supported inside unprivileged namespace"
            exit 0
        fi
        "$probe" "$store" &
        probe_pid=$!
        result=0
        wait "$probe_pid" || result=$?
        probe_pid=""
        exit "$result"
    ' _ "$SCRATCH_DIR" "$PROBE_BIN" > "$SCRATCH_DIR/probe.out" 2>&1 || RC=$?

SKIPPED=false
PASSED=false
while IFS= read -r line || [[ -n "$line" ]]; do
    printf '%s\n' "$line"
    case "$line" in
        SKIPPED:*) SKIPPED=true ;;
        PASS:*) PASSED=true ;;
    esac
done < "$SCRATCH_DIR/probe.out"
if [[ "$RC" -ne 0 ]]; then
    printf 'FAIL: owned probe/namespace exited %s; evidence retained %s\n' "$RC" "$SCRATCH_DIR"
    exit 1
fi
if [[ "$SKIPPED" == true ]]; then
    exit 0
fi
if [[ "$PASSED" == true ]]; then
    echo "PASS: bounded kernel ENOSPC, exact cold recovery and new durable writes verified"
    exit 0
fi
echo "FAIL: probe completed without reached ENOSPC/recovery PASS"
exit 1
