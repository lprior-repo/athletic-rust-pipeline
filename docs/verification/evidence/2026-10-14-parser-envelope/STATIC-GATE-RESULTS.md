# Terminal static gate results

The executed strict-size scan FAILED (command exit 1), and module seams FAILED (command exit 1). Comments and panic-extraction PASSED. A serial wrapper exit of 0 does not override an individual command failure. Exact invocations, requested environment settings, command exits and serial-session exits are preserved in `raw/root-command-receipts.json` and `GATE-STATUS.json`.

The first scan attempt failed during prerequisite chromiumoxide_cdp compilation (SIGKILL; command1/internal101), before a scan ran. Root interrupted the following comments attempt (serial-session130; no per-command terminal result). Both original logs are retained. The retry reached actual xtask execution.

## Strict handwritten size: FAIL

14 functions exceed 60 trusted logical lines. None of their paths is in the exact 38-path parser-envelope change. Every listed failed-site file is byte-identical between parent `7996734d87d0305eb8bb2060d5e65199cee95a6e` and source commit `771072a1013017a4f8b4993751f801df16f789a2`. This proves delta disjointness and unchanged files, not a waiver or a passing gate.

| Source | Function | Logical lines |
|---|---|---:|
| xtask/src/contract/boundaries.rs:55 | crate_boundaries | 65 |
| crates/census-store/src/installation/eligibility.rs:44 | require_empty_metadata | 61 |
| crates/census-store/src/acquisition/transition/work.rs:9 | register_acquisition_work | 66 |
| crates/census-store/src/acquisition/terminal.rs:132 | reuse_acquisition_cache | 66 |
| crates/census-store/src/acquisition/read.rs:99 | read_work | 77 |
| crates/census-store/src/acquisition/read/closure.rs:8 | attempt | 64 |
| crates/census-store/src/acquisition/qualification.rs:75 | declaration | 62 |
| crates/census-store/src/acquisition/qualification/inventory.rs:7 | inventory | 94 |
| crates/census-store/src/acquisition/cache.rs:35 | inventory | 82 |
| crates/census-store/src/acquisition/backing.rs:33 | validate_metadata | 63 |
| crates/census-service/src/bootstrap/serve.rs:134 | spawn_endpoint | 61 |
| crates/census-domain/src/affiliation_candidates/selection_reduce.rs:9 | classify_assertion | 64 |
| crates/census-crawl/src/net/execute/fetch.rs:22 | fetch_with_session | 63 |
| crates/census-crawl/src/net/execute/dispatch.rs:9 | dispatch_at | 64 |

The report checks 1585 strict files and 18374 callables, with no unresolved sites and zero forbidden-construct counts in its nine package summaries. It also retains 1689 advisory >25-line sites, 138 >5-parameter warnings and a file-length inventory entry for census-store backup/integrity.rs (668 lines). These inventories are preserved in full rather than silently dropped or represented as newly waived.

`SCAN-STRUCTURED-EXTRACT.json` is a derived machine-readable copy of the emitted report. Only explicitly listed stderr lines interleaved inside the JSON were excluded for parsing; `raw/xtask-scan-retry.log` remains unchanged and includes every error.

## Module seams: FAIL

Three installation → bootstrap edges at `crates/census-service/src/installation/contracts.rs:42`, `:43`, and `:44`; zero crate-seam violations. This file is outside the envelope paths and unchanged across the envelope source commit. The gate still fails and requires separate repair.

## Passing lanes

- Comments: 3002 Rust files checked; no comments; command exit 0.
- Panic-extraction: 3002 Rust files and 3 rendered templates checked; command exit 0.

Requested retry controls are recorded exactly per command, including jobs/incremental/debug/thread settings and the actual timeout (1800s for scan; 900s for the remaining retry commands). The successful dev build reports unoptimized without debuginfo; individual successful rustc command lines were not printed, so no stronger flag-by-flag observation is asserted.

These results block full acceptance. The narrow 107-test/API/basic-gate passes remain valid scoped evidence, with all limits and failures visible.
