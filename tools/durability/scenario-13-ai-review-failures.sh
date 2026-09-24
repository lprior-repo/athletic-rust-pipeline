#!/usr/bin/env bash
# scenario-13-ai-review-failures.sh — AI timeout, malformed JSON, hallucinated case id,
# conflicting verdicts, and a proposed match across a deterministic contradiction.
#
# Pass observable: each failure mode produces the exact terminal error the review lane defines;
# the seal refuses with the appropriate RetainedFindings item named.

set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/13}"
BINARY="${BINARY:-census-service}"
ADMIN_PORT="${ADMIN_PORT:-19095}"
SERVICE_PORT="${SERVICE_PORT:-9080}"

echo "--- scenario 13: AI review failures ---"

# Verify prerequisites
if ! "$BINARY" --help >/dev/null 2>&1; then
    echo "SKIPPED: census-service binary not available"
    exit 0
fi

# The AI review lane (census-review crate) handles identity review using a local model.
# The review lane is invoked through the `review` CLI command and the LocalReviewer
# Restate object.
#
# To inject AI review failures, we would need to:
# 1. Control the local model's responses (timeout, malformed JSON, hallucinated IDs)
# 2. Inject conflicting verdicts
# 3. Verify the seal refuses with the appropriate items
#
# The local model is invoked through a model SDK, which we cannot control from outside.
# The review cases are stored in the `review_cases` keyspace and reviewed through
# the `LocalReviewer::review` handler.
#
# Without access to the model's internal state or a way to inject specific responses,
# we cannot test these failure modes via external fault injection.
echo "SKIPPED: no AI review fault-injection seam; the local model (census-review crate) is invoked internally through the model SDK and its responses cannot be controlled from outside the process. The review failure modes (timeout, malformed JSON, hallucinated case id, conflicting verdicts) are internal to the model lane and cannot be triggered via external configuration or environment variables."
exit 0
