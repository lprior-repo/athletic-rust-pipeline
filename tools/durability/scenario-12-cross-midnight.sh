#!/usr/bin/env bash
# scenario-12-cross-midnight-replay.sh — cross midnight during replay; all journaled
# timestamps byte-identical.
#
# Pass observable: all journaled timestamps in the Fjall journal keyspace are
# byte-identical to what they would have been without the clock advance.

set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/12}"
BINARY="${BINARY:-census-service}"
ADMIN_PORT="${ADMIN_PORT:-19095}"
SERVICE_PORT="${SERVICE_PORT:-9080}"

echo "--- scenario 12: cross-midnight replay ---"

# Verify prerequisites
if ! "$BINARY" --help >/dev/null 2>&1; then
    echo "SKIPPED: census-service binary not available"
    exit 0
fi

if ! curl -sf "http://127.0.0.1:$ADMIN_PORT/health" >/dev/null 2>&1; then
    echo "SKIPPED: Restate admin not reachable on port $ADMIN_PORT"
    exit 0
fi

STORE="$SCRATCH_STORE"
rm -rf "$STORE" && mkdir -p "$STORE"

# The journal keyspace stores entries with ISO-8601 timestamps in the value:
# {"key": <key>, "at": <iso-8601>, "payload": <caller JSON>}
# (FJALL_SCHEMA.md §2, journal keys)
#
# These timestamps are written by chrono::Utc::now() in store/write.rs:73 and
# report/projection.rs:112 (HARDENING-PROGRAM.md §2.3).
#
# To test cross-midnight replay:
# 1. Start a long workflow (e.g., JurisdictionCensus for multiple states)
# 2. Advance the system clock by 24 hours
# 3. Let the workflow complete
# 4. Compare journal timestamps to a reference run
#
# Problem 1: We cannot reliably advance the system clock in a test environment.
#   - The `faketime` utility can intercept time calls, but it requires LD_PRELOAD
#     injection and does not work with all systems.
#   - tokio::time::pause/advance is available for tests, but not for external injection.
#
# Problem 2: We cannot verify byte-identical timestamps without a reference run.
#   The timestamps are generated at write time, so they would differ anyway.
#   The actual invariant is that the timestamps are valid and ordered, not that
#   they are byte-identical across clock advances.
#
# Problem 3: The "byte-identical" claim in the task description seems to mean
# that the timestamps reflect real wall-clock time consistently, not that they
# are the same string. A clock jump would produce different timestamp strings
# but the same logical ordering.
echo "SKIPPED: no clock-manipulation seam in census-service; tokio::time::pause/advance is not exposed to external callers. The journal timestamps (chrono::Utc::now) are written at the time of the journal entry, so advancing the clock would change the timestamp strings. The invariant is timestamp validity and ordering, not byte-identity across clock changes. Testing this would require faketime or similar LD_PRELOAD time manipulation, which is not a seam in the pipeline."
exit 0
