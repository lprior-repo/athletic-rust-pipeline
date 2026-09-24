#!/usr/bin/env bash
# scenario-05-http-error-taxonomy.sh — inject HTTP errors and check the exact terminal taxonomy.
#
# Errors to inject: 429+Retry-After, 403, 404, 500, timeout, connection reset,
#   invalid HTML, corrupted JSON, oversized bodies.
#
# Pass observable: each injected error produces the exact terminal error the crawl lane defines;
# no retry exhaust leaks into the report.

set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/05}"
BINARY="${BINARY:-census-service}"
ADMIN_PORT="${ADMIN_PORT:-19095}"
SERVICE_PORT="${SERVICE_PORT:-9080}"

echo "--- scenario 05: HTTP error taxonomy ---"

# Verify prerequisites
if ! "$BINARY" --help >/dev/null 2>&1; then
    echo "SKIPPED: census-service binary not available"
    exit 0
fi

# The crawl lane reads from sources like athletic.net via a headed browser (chromiumoxide).
# HTTP error injection requires a local proxy or MITM that can intercept and modify
# responses. This is not a seam exposed by the current codebase — the browser session
# is opaque from the outside.
#
# The census-crawl crate uses chromiumoxide for browser automation; it does not use
# reqwest for direct HTTP. The error taxonomy (CrawlError variants) is defined in
# the browser transport, not in a reqwest-based fetcher.
#
# Without a proxy or the ability to control the HTTP response from the browser,
# we cannot inject specific HTTP status codes into the crawl lane.
echo "SKIPPED: no HTTP error-injection proxy seam in census-crawl; the crawl uses a headed browser (chromiumoxide) which does not expose a controllable HTTP response surface. The crawl error taxonomy (CrawlError::RateLimited, CrawlError::Forbidden, etc.) is defined in the browser transport and cannot be triggered via external HTTP injection."
exit 0
