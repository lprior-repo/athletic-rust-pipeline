#!/usr/bin/env bash
# scenario-11-parent-exit.sh — endpoint HTTP server crash without parent termination.
#
# Requires ATHLETIC_FAULT_HTTP_EXIT fault-injection seam which is not implemented.

set -euo pipefail

SERVE_BINARY="${SERVE_BINARY:-census-serve}"

if ! command -v "$SERVE_BINARY" >/dev/null 2>&1; then
    echo "SKIPPED: census-serve binary not available at $SERVE_BINARY"
    exit 0
fi

if [[ -z "${ATHLETIC_FAULT_HTTP_EXIT:-}" ]]; then
    echo "SKIPPED: ATHLETIC_FAULT_HTTP_EXIT environment variable not set; the endpoint must honour this seam to make the HTTP server task return an error while the parent supervisor process continues running"
    exit 0
fi

echo "SKIPPED: ATHLETIC_FAULT_HTTP_EXIT is not implemented in census-service; no fault-injection abort point exists for HTTP server task termination"
exit 0
