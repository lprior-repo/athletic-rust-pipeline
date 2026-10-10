# Verification Evidence

## Performance Gate

The performance gate runs each benchmark N=3 times and uses the best (lowest time) measurement
for comparison against the baseline. This reduces the impact of single-pass environmental noise
(VM scheduling, thermal throttling, page cache state) while preserving the regression-detection
ability of the baseline.

## Durability Scenarios

Durability scenarios are tested via `tools/durability/run.sh`. Scenario-06 (no-duplicate-evidence)
injection at 7 crash boundaries, proving evidence idempotency and receipt reuse across restarts.

## Integrity Checks

The `store-integrity` command now includes HTTP cache pair verification, checking that:
- All body files have valid SHA-256 digests
- All meta files reference existing body files
- No orphaned or unexpected files exist in the cache