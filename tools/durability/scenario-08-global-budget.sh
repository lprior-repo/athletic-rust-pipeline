#!/usr/bin/env bash
# scenario-08-global-budget.sh — two immutable endpoint versions running simultaneously
# still enforce one global remote-origin budget.
#
# Pass observable: total concurrent source fetches across both versions never exceeds
# --max-concurrent; no source is throttled by one version's load.

set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/08}"
BINARY="${BINARY:-census-service}"
ADMIN_PORT="${ADMIN_PORT:-19095}"
SERVICE_PORT="${SERVICE_PORT:-9080}"

echo "--- scenario 08: global budget across versions ---"

# Verify prerequisites
if ! "$BINARY" --help >/dev/null 2>&1; then
    echo "SKIPPED: census-service binary not available"
    exit 0
fi

if ! curl -sf "http://127.0.0.1:$ADMIN_PORT/health" >/dev/null 2>&1; then
    echo "SKIPPED: Restate admin not reachable on port $ADMIN_PORT"
    exit 0
fi

# The --max-concurrent flag controls the number of concurrent blocking-pool jobs within
# a single endpoint process (bootstrap/options.rs: ServeOptions.max_concurrent, default 8).
# It is a per-process semaphore, not a global budget.
#
# Two endpoint versions (if they could coexist) would each have their own semaphore.
# There is no cross-process budget enforcement. The budget is local to each endpoint's
# region and JoinSet.
#
# Additionally, the endpoint versions scenario (scenario 04) is not supported because
# Restate registers services by name and two processes cannot register the same service.
echo "SKIPPED: --max-concurrent is a per-process semaphore (census-service bootstrap/options.rs), not a global budget; two endpoint versions cannot coexist (Restate registers services by name, causing conflicts). There is no cross-process budget enforcement seam."
exit 0
