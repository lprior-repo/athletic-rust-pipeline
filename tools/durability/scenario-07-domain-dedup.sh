#!/usr/bin/env bash
# scenario-07-domain-dedup.sh — domain operation dedup across two submissions.

set -euo pipefail

BINARY="${BINARY:-census-service}"

if ! "$BINARY" --help >/dev/null 2>&1; then
    echo "SKIPPED: census-service binary not available"
    exit 0
fi

echo "SKIPPED: no concurrent cross-workflow domain-dedup scenario implemented"
exit 0
