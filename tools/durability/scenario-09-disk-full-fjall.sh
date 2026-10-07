#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
if [[ "$TARGET_DIR" != /* ]]; then
    TARGET_DIR="$REPO_ROOT/$TARGET_DIR"
fi
PROBE_BIN="$TARGET_DIR/debug/examples/enospc"

for tool in unshare mount findmnt timeout cp mktemp ls; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'SKIPPED: %s not available\n' "$tool"
        exit 0
    fi
done
if ! unshare --user --map-root-user --mount --propagation private true 2>/dev/null; then
    echo "SKIPPED: unprivileged user+mount namespace not supported"
    exit 0
fi

SCRATCH_ROOT="${SCRATCH_STORE:-${TMPDIR:-/tmp}}"
mkdir -p -- "$SCRATCH_ROOT"
SCRATCH_DIR="$(mktemp -d "$SCRATCH_ROOT/enospc-09-XXXXXX")"
printf 'EVIDENCE: %s\n' "$SCRATCH_DIR"

if ! (cd "$REPO_ROOT" && cargo build --example enospc -p census-store --quiet) > "$SCRATCH_DIR/build.log" 2>&1; then
    printf 'FAIL: probe compile failure; evidence retained %s\n' "$SCRATCH_DIR"
    cat "$SCRATCH_DIR/build.log"
    exit 1
fi
if [[ ! -x "$PROBE_BIN" ]]; then
    printf 'FAIL: probe binary not found at %s; evidence retained %s\n' "$PROBE_BIN" "$SCRATCH_DIR"
    exit 1
fi

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
PASS_COUNT=0
PASS_KIND=""
ENOSPC_COUNT=0
CLEANUP_SEEN=false
while IFS= read -r line || [[ -n "$line" ]]; do
    case "$line" in
        SKIPPED:*)
            SKIPPED=true
            printf '%s\n' "$line"
            ;;
        'PASS: kernel ENOSPC refused '*)
            PASS_COUNT=$((PASS_COUNT + 1))
            PASS_KIND="kernel"
            printf 'PROBE-%s\n' "$line"
            ;;
        'PASS:'*)
            PASS_COUNT=$((PASS_COUNT + 1))
            PASS_KIND="unexpected"
            printf 'PROBE-%s\n' "$line"
            ;;
        'ENOSPC: atomic commit refused '*)
            ENOSPC_COUNT=$((ENOSPC_COUNT + 1))
            printf '%s\n' "$line"
            ;;
        'CLEANUP: namespace exit 0; artifact retained '*)
            CLEANUP_SEEN=true
            printf '%s\n' "$line"
            ;;
        *)
            printf '%s\n' "$line"
            ;;
    esac
done < "$SCRATCH_DIR/probe.out"

if [[ "$RC" -ne 0 ]]; then
    printf 'FAIL: owned probe/namespace exited %s; evidence retained %s\n' "$RC" "$SCRATCH_DIR"
    exit 1
fi
if [[ "$SKIPPED" == true ]]; then
    exit 0
fi
if [[ "$PASS_COUNT" -ne 1 || "$PASS_KIND" != "kernel" ]]; then
    printf 'FAIL: expected exactly one kernel-ENOSPC probe PASS line, observed %s (%s); evidence retained %s\n' \
        "$PASS_COUNT" "${PASS_KIND:-none}" "$SCRATCH_DIR"
    exit 1
fi
if [[ "$ENOSPC_COUNT" -ne 1 ]]; then
    printf 'FAIL: expected exactly one reached kernel ENOSPC refusal, observed %s; evidence retained %s\n' \
        "$ENOSPC_COUNT" "$SCRATCH_DIR"
    exit 1
fi
if [[ "$CLEANUP_SEEN" != true ]]; then
    printf 'FAIL: fault namespace did not report a clean tmpfs exit; evidence retained %s\n' "$SCRATCH_DIR"
    exit 1
fi
if [[ ! -d "$SCRATCH_DIR/preserved-store/fjall" ]] || [[ -z "$(ls -A -- "$SCRATCH_DIR/preserved-store" 2>/dev/null)" ]]; then
    printf 'FAIL: cold fault-store preservation is missing or empty; evidence retained %s\n' "$SCRATCH_DIR"
    exit 1
fi
echo "PASS: bounded kernel ENOSPC refused an atomic commit; exact acknowledged receipts, cold readback, unchanged replay and a new atomic recovery write verified across reopens"
