#!/usr/bin/env bash
# scenario-03-reboot-with-full-census.sh — reboot with active NationalCensus.

set -euo pipefail

BINARY="${BINARY:-census-service}"

if ! "$BINARY" --help >/dev/null 2>&1; then
    echo "SKIPPED: census-service binary not available"
    exit 0
fi

echo "SKIPPED: no isolated whole-machine reboot scenario; process restart does not prove machine recovery"
exit 0
