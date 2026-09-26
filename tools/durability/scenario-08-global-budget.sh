#!/usr/bin/env bash
# scenario-08-global-budget.sh — global budget across two endpoint versions.

set -euo pipefail

BINARY="${BINARY:-census-service}"

if ! "$BINARY" --help >/dev/null 2>&1; then
    echo "SKIPPED: census-service binary not available"
    exit 0
fi

echo "SKIPPED: no multi-endpoint global-budget scenario implemented"
exit 0
