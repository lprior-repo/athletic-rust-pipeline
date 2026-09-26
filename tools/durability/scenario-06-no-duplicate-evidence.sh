#!/usr/bin/env bash
# scenario-06-no-duplicate-evidence.sh — no duplicate evidence after Fjall commit.

set -euo pipefail

BINARY="${BINARY:-census-service}"

if ! "$BINARY" --help >/dev/null 2>&1; then
    echo "SKIPPED: census-service binary not available"
    exit 0
fi

echo "SKIPPED: no crash-injected evidence replay assertion; unchanged observation counts alone do not prove evidence idempotency"
exit 0
