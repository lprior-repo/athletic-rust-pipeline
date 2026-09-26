#!/usr/bin/env bash
# scenario-12-cross-midnight-replay.sh — cross midnight during replay.

set -euo pipefail

BINARY="${BINARY:-census-service}"

if ! "$BINARY" --help >/dev/null 2>&1; then
    echo "SKIPPED: census-service binary not available"
    exit 0
fi

echo "SKIPPED: no clock-manipulation seam in census-service"
exit 0
