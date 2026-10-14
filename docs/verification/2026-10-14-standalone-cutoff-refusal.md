# Standalone cutoff refusal — 2026-10-14

Base b4b6a485; code: `crates/census-service/src/cli/cycle.rs`;
regressions: `crates/census-service/tests/standalone_run_cutoff.rs`.
Standalone `run --as-of` now refuses before routing, opening a store or ingress.
Provider cutoff behavior is unchanged. This prevents an unsupported cutoff from
silently producing an unfiltered export; it does not implement profile execution.

Cloud validation (CI unset; MOON_RUST_BUILD_JOBS=1):
- PASS: `tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-service --test standalone_run_cutoff --no-capture --no-tests fail`.
  9 passed, 0 failed/skipped; 30 isolated real-binary invocations, no-network backstops.
  Nextest bf9467cb-c312-4eb5-bb90-27e84b54cfcc; 0.400s tests, 4m4.694s task.
- PASS: `tools/moon-local run pipeline:fmt` (8.370s) and
  `tools/moon-local run pipeline:check -- --jobs 1` (2m41.506s), on the combined
  cloud tree containing this guard and PR45's R5/R6 repairs, not an isolated head.
- NOT RUN: strict lint, full release gate, live acquisition, national run and representative RSS.

Changed-surface review checked early refusal, default/explicit live and offline,
input/meets combinations, date parsing, help and no-cutoff compatibility.
R1/R8 and full selected-profile acceptance remain open.
