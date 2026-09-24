#!/usr/bin/env bash
# scenario-04-rolling-upgrade.sh — start v1, launch a long workflow, register v2;
# the old invocation continues on v1 while new work uses v2; then drain and remove v1.
#
# Pass observable: old invocation completes on v1 with same result; new invocations use v2;
# after draining v1, final result is identical to a clean v2 run.

set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/04}"
BINARY="${BINARY:-census-service}"
ADMIN_PORT="${ADMIN_PORT:-19095}"
SERVICE_PORT="${SERVICE_PORT:-9080}"

echo "--- scenario 04: rolling upgrade v1 → v2 ---"

# Verify prerequisites
if ! "$BINARY" --help >/dev/null 2>&1; then
    echo "SKIPPED: census-service binary not available"
    exit 0
fi

if ! curl -sf "http://127.0.0.1:$ADMIN_PORT/health" >/dev/null 2>&1; then
    echo "SKIPPED: Restate admin not reachable on port $ADMIN_PORT"
    exit 0
fi

if ! curl -sf "http://127.0.0.1:$SERVICE_PORT/health" >/dev/null 2>&1; then
    echo "SKIPPED: Census endpoint not reachable on port $SERVICE_PORT"
    exit 0
fi

STORE="$SCRATCH_STORE"
rm -rf "$STORE" && mkdir -p "$STORE"

# Step 1: Check if the endpoint supports versioned deployments
# The census-service uses Restate service definitions; versioning is at the Restate node level.
# Two endpoints registering the same service with different wire names would be different services.
# The census-service does not expose a versioned deployment mechanism — all definitions are
# registered under their struct name (Census, JurisdictionCensus, etc.).
#
# To test rolling upgrades, we would need two separate endpoint processes registering the same
# service definitions. Restate's service registration is by name, so two processes with the same
# wire name would conflict. The only way to have two versions is to use different service names
# (e.g., JurisdictionCensus:v1 and JurisdictionCensus:v2), which requires code changes.
echo "SKIPPED: no dual-version endpoint registration seam in census-service; Restate registers services by struct name and two processes with the same name would conflict. A rolling upgrade would require versioned service names (e.g., JurisdictionCensus:v1) which is a code change, not a configuration-level seam."
exit 0
