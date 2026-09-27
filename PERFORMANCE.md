# Performance measurement

This guide owns how to measure and what the current tooling can establish. Historical timings,
counts and failed experiments live only in
[VERIFICATION-EVIDENCE.md](docs/VERIFICATION-EVIDENCE.md). Source/size debt is not a throughput
measurement. The [national plan](docs/NATIONAL-CENSUS-PLAN.md) owns delivery thresholds and correctness.

## Commands and records

Run on a quiet machine, with the pinned toolchain and explicit input/store generation.

| Purpose | Command | Output |
|---|---|---|
| Criterion core groups | `cargo bench -p census-service --bench core` | Criterion artifacts |
| Criterion pipeline groups | `cargo bench -p census-service --bench pipeline` | Criterion artifacts |
| Record throughput baseline | `cargo xtask perf record` | `tools/perf-baseline.json` |
| Compare baseline | `cargo xtask perf check --reason 'description of comparison'` | Per-group differences and exit status |
| Profile a selected group | `cargo xtask perf profile <group>` | `perf` recording or an explicit missing-tool message |
| Synthetic census phases | `cargo run --release -p census-service --example bench_census -- --schools 500` | Phase metrics and JSON |
| Store durability cost | `cargo run --release -p census-service --example bench_store -- --rows 200000 --batch 1000 --scan` | Append/scan/consolidate metrics and JSON |

No `tools/perf-baseline.json` was present at the 2026-09-27 documentation check.
`tools/quality-baseline.json` is the separate debt record. An old measured workbook duration does not
populate a Criterion baseline or make a performance release gate pass.

The gate's `--full` mode requests performance and mutation lanes; `--release` includes those lanes
and refuses a missing performance baseline. Default/developer skips are not measurement evidence.
See [TESTING.md](TESTING.md) for gate modes, rather than treating benchmark compilation as execution.

## Current implementation limits

`xtask/src/perf/` is the command implementation. Its record includes CPU, core count, rustc,
revision, corpus-line count and per-group throughput, wall time and optional peak RSS. The comparison
threshold defaults to 5% throughput regression. As inspected on 2026-09-27:

- An entirely empty current benchmark map and a baseline group absent from the current map fail.
  A current group absent from the baseline also fails. This repairs the older empty-map/missing-group
  defects recorded in the evidence ledger; do not keep those historical findings as current claims.
- CPU/core/rustc differences print warnings and continue; revision differences are informational.
  The tool does not enforce a comparable environment merely because metadata is recorded.
- A group's missing throughput is printed as a skip, not a failure. Peak RSS is optional and is not
  thresholded. A throughput pass alone therefore cannot certify every required resource metric.
- The capture wrapper uses `/usr/bin/time -v` around Cargo/bench and a shared
  `/tmp/time-output.txt`; parallel invocations can overwrite that timing evidence. Run it serially
  until it has isolated capture paths and verified process attribution.
- Throughput declarations are discovered under `target/criterion/<bench>/.../new/benchmark.json`.
  Missing/unreadable metadata can leave throughput absent. Validate the actual emitted group set,
  sample count, units and nonzero measurements before accepting a baseline.
- The profiling wrapper forwards `--bench-ids`; its support must be verified against the pinned
  benchmark executable before calling profiling delivered. A printed command is not a profile.

These are tooling gaps, not permission to invent numbers, drop a slow group or weaken an oracle.

## Example harnesses

`bench_census` builds a deterministic synthetic corpus: one school/team/meet, eight athletes and
sixteen performances per school, with deduplicated event rows. Its phases cover append,
consolidation, both census scopes, bests and workbook output. Each phase must validate its expected
semantic rows before reporting a rate.

A previously executed small smoke, `cargo run -q -p census-service --example bench_census -- --schools 2`,
failed with `bests produced 0 rows, expected 16`. The fixture's unresolved meet sport made those
performances ineligible. Repair that fixture against the production participation contract before
recording a baseline; do not remove the expected-row assertion. This failure was not rerun during
the documentation cleanup.

`bench_store` appends the same school names twice with different observation labels: `2 × rows`
observations, `rows` merged schools and two evidence entries per school. The single-append versus
batched-append phases expose durability cost. `--scan` adds read/consolidation checks. It is not a
model of public-host latency, browser contention, production mark diversity or full census export.

Example output uses `metric=<phase>_items`, `_seconds`, `_rate`, `wall_seconds` and a final `json=`
record containing flags, corpus counts, phase metrics and store counters. Preserve the raw output;
compare identical workloads and distinguish wall time, CPU time, physical requests, applied records
and merged entities. A single timing difference does not establish a speedup.

## Measurement contract

1. Pin the revision, build profile, Rust/dependency versions, CPU/memory, store/input generation,
   cohort/scope, cache state and competing workload. Never compare debug and release as an
   optimization A/B or silently substitute a smaller corpus.
2. Validate the workload before timing: no empty parser result, omitted group, incompatible best,
   failed ingestion or partial workbook can count as useful completed work.
3. Measure end-to-end useful records per physical request and elapsed time, plus stage durations,
   peak memory, disk reads/writes, durability barriers, queue depth and backpressure. State the
   process/cgroup used for resource accounting; unavailable mandatory values are failures, not zero.
4. Repeat comparable baseline/candidate runs. Preserve exact counts, key sets and output semantics,
   not only totals. Report variance and identify the measured bottleneck before optimizing it.
5. Re-run the consumer-visible correctness and recovery checks after optimization. Faster omitted
   evidence, disabled sync or a partial export is a defect, not throughput.

## Known export work

The current report paths repeatedly derive census, bests and workbook views. A live Fjall read
snapshot is not restartable export input, and selecting a table in a shared keyspace does not prove
an indexed range scan. The `rust_xlsxwriter` `constant_memory` feature is available, but ordinary
worksheet creation does not activate it. These distinctions must remain visible in measurements.

The national plan requires one recoverable frozen input generation, a bounded shared export dataset,
checkpointed work, and atomic bundle promotion. Profile those stages with the real corpus and
independent artifact readback; changing cache settings, adding workers or reducing allocations in
one comparator alone does not prove the end-to-end goal. Do not introduce a second exporter or
change storage encoding without measured benefit and an explicit migration contract.

## Debt baseline

`cargo xtask scan` and strict-Clippy measurement feed `tools/quality-baseline.json` through the gate.
`ratchet` compares measured debt; `quality-baseline` records it. Neither measures throughput.
The executable policy in `xtask/src/baseline.rs` distinguishes tracked forbidden constructs and
oversized-file membership from informational line counts and the hot-function target. Use the
gate's measured baseline refresh after a real reduction; never raise debt to make a failed change
green or copy historical counters into a new baseline.
