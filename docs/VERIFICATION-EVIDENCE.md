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

## 2026-10-10 Review Repairs

Exact commands and results for the results/cache/perf/gate repairs below, run from the repository
root on this tree: `env -u CI tools/moon-local run pipeline:tests` — 3206 passed, 4 skipped
(the focused nextest filter covering the six changed surfaces ran 12/12 first);
`pipeline:lint-src`, `pipeline:check` and `pipeline:fmt` — green;
`pipeline:report-test` — 313 passed; `tools/moon-local ci --force --summary detailed` — all five
tasks pass (tests 3m51s, report-test 4m02s).

- Wayzata results (`crates/census-crawl/src/wayzata/results/walk.rs`): each meet's performance rows
  join one batch and the batch is committed before any count is reported; `ResultsWalk::collected`
  is `u64` and `wayzata::walk` adds it to `report.rows`. Regression:
  `wayzata::tests::scenarios::results::collected_result_rows_are_committed_with_the_schedule_meet`
  (a schedule row linked to `/links/abc123` commits one performance and `report.rows == 2`).
- Performance gate (`xtask/src/perf/bench/runtime.rs`, `xtask/src/perf/bench.rs`): each of the
  three runs keeps its own criterion root; the run with the lowest measured RSS supplies output,
  RSS and criterion root together, and declarations/tails are read from that root instead of a
  stale top-level directory. Regressions:
  `perf::bench::tests::the_best_measured_run_supplies_its_own_criterion_root` and
  `runs_without_memory_readings_keep_the_first_criterion_root`.
- HTTP cache integrity (`crates/census-store/src/backup/http_cache.rs`): the live
  `<key>.body`/`<key>.meta.json` pairs and the archived
  `bodies/<digest>.body` + `captures/<digest>/<meta-digest>.meta.json` trees are checked against
  the bytes' SHA-256 and the recorded `content_digest`; orphaned, missing, mismatched and
  unexpected entries are reported. Regressions: `backup::http_cache::tests` (7 cases).
- Access-condition accounting (`crates/census-service/src/census/seal.rs`): `retained_access`
  counts `Forbidden` and `HumanRequired` as refusals; `RobotsDisallowed` is cooldown-bounded
  (robots is pacing, not an admission gate). Test:
  `census::seal::tests::access_conditions_split_into_refusals_and_throttles` — (7, 2, 5).
- Consolidate fault (`crates/census-service/src/restate_services/jobs.rs`): behind
  `native-fault-injection` the fault fires on a non-empty `ATHLETIC_FAULT_EXIT_AFTER_CONSOLIDATE`
  environment variable; the unused in-process injector and its atomic were removed.
- Spawner (`crates/census-service/src/spawn.rs`): `blocking` refuses a closed task region before
  waiting for capacity and returns `Outcome::Cancelled`.
