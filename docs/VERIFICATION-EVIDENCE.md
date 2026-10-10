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

## Perf Gate Repetition Rule

Each benchmark is run N=3 times; the best (lowest) result is used for comparison. This reduces
single-pass noise from VM scheduling and thermal state. The baseline file remains fixed.

## Keyspace A/B Harness Repair

The keyspace A/B harness (crates/census-store/examples/keyspace_ab.rs) was repaired to use
consistent key construction between writes and reads. Previously, the write path used
keyed(table, index, second) while the read/remove path used keyed(table, second, id), causing
reads to use incorrect keys. Both paths now use keyed(table, id, second) consistently.