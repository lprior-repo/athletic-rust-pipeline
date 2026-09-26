#!/usr/bin/env bash
# scenario-04-rolling-upgrade.sh — rolling upgrade v1 → v2.
#
# Requires dual-version endpoint registration seam which does not exist.

set -euo pipefail

BINARY="${BINARY:-census-service}"

if ! "$BINARY" --help >/dev/null 2>&1; then
    echo "SKIPPED: census-service binary not available"
    exit 0
fi

echo "SKIPPED: no two-version rolling-upgrade scenario implemented"
exit 0
