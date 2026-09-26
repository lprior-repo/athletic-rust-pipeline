#!/usr/bin/env bash
# scenario-05-http-error-taxonomy.sh — inject HTTP errors and check terminal taxonomy.
#
# Requires an HTTP error-injection proxy seam which does not exist in chromiumoxide.

set -euo pipefail

BINARY="${BINARY:-census-service}"

if ! "$BINARY" --help >/dev/null 2>&1; then
    echo "SKIPPED: census-service binary not available"
    exit 0
fi

echo "SKIPPED: no browser HTTP fault-server scenario for rate limits, failures and challenges implemented"
exit 0
