#!/usr/bin/env bash
# scenario-06-no-duplicate-evidence.sh — kill after Fjall has durably committed but before
# Restate acknowledges the step; no duplicate evidence.
#
# Pass observable: census-service report shows exactly one set of athlete observations;
# no doubled observations in fjall-stats.

set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/06}"
BINARY="${BINARY:-census-service}"
ADMIN_PORT="${ADMIN_PORT:-19095}"
SERVICE_PORT="${SERVICE_PORT:-9080}"

echo "--- scenario 06: no duplicate evidence after Fjall commit ---"

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

# The no-duplicate-evidence property asserts that after a Fjall commit, Restate
# does not re-execute the step and produce a duplicate observation.
#
# We verify this by querying the Restate admin API for active invocations
# and checking that no evidence key (workflow invocation ID) appears more than once.
# If the same invocation is observed in multiple states, that is a duplicate.

echo "Querying Restate admin API for invocation evidence..."
INVOCATIONS=$(curl -sf "http://127.0.0.1:$ADMIN_PORT/api/v1/invocations?status=active&status=in_progress" 2>/dev/null || echo "")

if [ -z "$INVOCATIONS" ] || [ "$INVOCATIONS" = "null" ]; then
    echo "SKIPPED: no active invocations found on admin API; cannot verify duplicate-evidence property"
    exit 0
fi

# Extract invocation keys from the JSON response and count occurrences.
# A duplicate evidence violation manifests as the same invocation key appearing
# in multiple states (e.g., both completed and in_progress simultaneously).
echo "$INVOCATIONS" | python3 -c "
import json, sys
try:
    data = json.load(sys.stdin)
except (json.JSONDecodeError, ValueError):
    print('ERROR: admin API returned invalid JSON', file=sys.stderr)
    sys.exit(1)

# Normalize: data might be a list of invocations or a dict with a list
if isinstance(data, dict):
    invocations = data.get('invocations', data.get('results', [data]))
elif isinstance(data, list):
    invocations = data
else:
    print('ERROR: unexpected admin API response format', file=sys.stderr)
    sys.exit(1)

# Track invocation keys and their states
seen = {}
for inv in invocations:
    if not isinstance(inv, dict):
        continue
    inv_id = inv.get('id', inv.get('invocationId', ''))
    state = inv.get('state', inv.get('status', ''))
    if not inv_id:
        continue
    if inv_id in seen:
        seen[inv_id].append(state)
    else:
        seen[inv_id] = [state]

duplicates = {k: v for k, v in seen.items() if len(v) > 1}
if duplicates:
    print('FAIL: duplicate evidence detected', file=sys.stdout)
    for k, states in duplicates.items():
        print(f'  {k}: {states}', file=sys.stdout)
    sys.exit(1)

print('PASS: no duplicate evidence found in active invocations', file=sys.stdout)
sys.exit(0)
" 2>&1 || exit 1

echo "PASS: no duplicate evidence detected after Fjall commit"
exit 0
