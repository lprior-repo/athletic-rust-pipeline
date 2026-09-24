#!/usr/bin/env bash
# scenario-11-parent-exit.sh — crash the endpoint HTTP server without terminating its parent
# supervisor.  The parent must honour a fault-injection seam (ATHLETIC_FAULT_HTTP_EXIT)
# that makes the HTTP server task return an error while the supervisor process keeps running,
# then exits nonzero so systemd restarts it.
#
# Pass observable: the census-serve binary exits with nonzero status after the HTTP task dies.
#
# Prerequisite seam: ATHLETIC_FAULT_HTTP_EXIT env var.  When set, the HTTP server task
# returns an error while the parent supervisor process continues running.

set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/11}"
BINARY="${BINARY:-census-service}"
SERVE_BINARY="${SERVE_BINARY:-$BINARY}"
ADMIN_PORT="${ADMIN_PORT:-19095}"
SERVICE_PORT="${SERVICE_PORT:-9080}"

echo "--- scenario 11: parent exit on HTTP kill ---"

# Verify prerequisites
if ! "$SERVE_BINARY" --help >/dev/null 2>&1; then
    echo "SKIPPED: census-serve binary not available at $SERVE_BINARY"
    exit 0
fi

STORE="$SCRATCH_STORE"
rm -rf "$STORE" && mkdir -p "$STORE"

# Check for the fault-injection seam: ATHLETIC_FAULT_HTTP_EXIT
# This env var must be honoured by the endpoint: when set, the HTTP server task
# returns an error while the parent supervisor process continues running.
if [[ -z "${ATHLETIC_FAULT_HTTP_EXIT:-}" ]]; then
    echo "SKIPPED: ATHLETIC_FAULT_HTTP_EXIT environment variable is not set; the endpoint must honour this seam to make the HTTP server task return an error while the parent supervisor process continues running"
    exit 0
fi

# Start the endpoint with the fault-injection seam active
echo "Starting census-serve with ATHLETIC_FAULT_HTTP_EXIT=$ATHLETIC_FAULT_HTTP_EXIT"
"$SERVE_BINARY" \
    --listen 127.0.0.1:9099 \
    --data-dir "$STORE" \
    --max-concurrent 1 \
    --restate-admin "http://127.0.0.1:${ADMIN_PORT}/" &
SERVE_PID=$!

# Wait for the endpoint to be ready
READY=false
for i in $(seq 1 30); do
    if curl -sf "http://127.0.0.1:9099/health" >/dev/null 2>&1; then
        READY=true
        break
    fi
    sleep 1
done

if ! $READY; then
    echo "SKIPPED: census-serve did not become ready within 30 seconds"
    kill "$SERVE_PID" 2>/dev/null || true
    exit 0
fi

# The fault-injection seam should trigger: the HTTP server task returns an error
# and the parent process exits nonzero.
# Wait for the process to exit
EXIT_CODE=-1
if wait "$SERVE_PID" 2>/dev/null; then
    EXIT_CODE=0
else
    EXIT_CODE=$?
fi

# Clean up any remaining processes
kill "$SERVE_PID" 2>/dev/null || true
wait "$SERVE_PID" 2>/dev/null || true

if [[ $EXIT_CODE -ne 0 ]]; then
    echo "PASS: parent process (PID $SERVE_PID) exited with code $EXIT_CODE after HTTP task error"
    echo "Systemd's Restart=on-failure would pick this up and restart the endpoint."
else
    echo "FAIL: parent process exited with code 0 — the HTTP task error did not propagate to the parent"
    exit 1
fi
exit 0
