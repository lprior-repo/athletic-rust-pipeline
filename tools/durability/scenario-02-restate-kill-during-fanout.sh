#!/usr/bin/env bash
# scenario-02-restate-kill-during-fanout.sh — Restate kill during NationalCensus fanout.

set -euo pipefail

RESTATE_BINARY="${RESTATE_BINARY:-restate-server}"

if ! "$RESTATE_BINARY" --help >/dev/null 2>&1; then
    echo "SKIPPED: restate-server binary not available at $RESTATE_BINARY"
    exit 0
fi

echo "SKIPPED: no isolated NationalCensus mid-fanout crash scenario; paused Consolidate recovery does not prove fanout recovery"
exit 0
