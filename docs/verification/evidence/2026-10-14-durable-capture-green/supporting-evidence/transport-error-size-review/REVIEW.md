# Capture Store error-size repair: focused source review

**Result: no blocking source defect found in the approved seven-file delta.**

Review date: 2026-10-14 UTC. Scope is the detached error-size repair and its current integrated, formatting-only equivalent. This is source review, not compilation, test execution, lint acceptance, performance certification, or release approval. No build, test, formatter, network operation, commit, or live-tree write was performed. The separate service `AdmittedCapture` change was neither edited nor reviewed.

Reviewer role: `sol-reviewer`. The repository reviewer definition selects `openai-codex/gpt-6.1-sol:high`; the runtime does not independently expose a model-version identity for this report.

## Exact reviewed identities

Detached proposal: `/workspace/shared/athletic-recovery-20261013-2203/transport-error-size-repair`.

- `changes.patch`: `07ba7e8839dcc396a3c1b0f86c0378b6199619ad63e979d915c23f995997cc82`
- `proposal-pins.sha256`: `8760eacbb80a3bd444ca892f3fdb769c99b57009816b8c0fb78fd3989ab3e9c9`
- All seven proposed files and all six retained base files independently passed their SHA256 manifests. Exact per-file identities are in `reviewed-proposal-pins.sha256` and `reviewed-base-pins.sha256`.
- Live checkout HEAD observed: `5111a603cc1d945e2da3f85c00d149796552e43e`. HEAD alone does not identify its working-tree content; use `reviewed-live-pins.sha256` and `context-pins.sha256`.
- By this review's live inspection, root had already integrated the patch. Six live files exactly match detached output. The seventh, `crates/census-crawl/src/net/types/tests.rs`, differs only in whitespace and a permitted trailing argument comma, as captured in `live-format-only.diff`; its reviewed live SHA256 is `0014b818baf207615ee508d00b4f9280fdffcd305006f2361f432a97f5637be3`.
- The implementer reports that the six input bases matched live at freeze time 07:04:54 UTC. This reviewer verified retained base hashes, not the historical live state at that instant. Do not reapply the patch to already-integrated live output.

## Contract and source findings

1. **Only the approved payload is boxed.** `source/crates/census-crawl/src/net/types.rs:10-11,95-99` retains `#[error(transparent)]`, changes only `CaptureStore` to `Box<StoreError>`, and implements `From<StoreError>` explicitly. No `CrawlError`, `StoreError`, other fetch variant, serialized shape, dependency, or policy rule is changed. The representation retains the concrete type and moves the original value into one box, without stringifying, cloning, or reclassifying it.

2. **Conversions are complete.** `source/crates/census-crawl/src/net/acquisition.rs:162-165` maps `CaptureError` through `StoreError::from` and the single new `FetchError::from`. Existing `?`, `into()`, and `From<StoreError>` consumers retain their conversion path. The wildcard classifier matches need no change; the only direct fixture constructor is migrated. A workspace Rust search for `CaptureStore` and the conversion implementations found no remaining old deep pattern or constructor. No new `From<Box<StoreError>>` is required because that conversion was not previously supplied.

3. **Transparent diagnostics and sources are preserved.** Locked `thiserror` and `thiserror-impl` are version 2.0.20. Its enum expansion at `thiserror-impl-2.0.20/src/expand.rs:230-240` forwards `source()` to the transparent payload's own source, and line 415 delegates Display. The pinned Rust library's `alloc/src/boxed.rs:2416-2424` forwards `Box<E>::source()` to `E::source()`. Therefore `FetchError::CaptureStore(StoreError::Io)` and its transparent `CrawlError::Fetch` wrapper continue exposing the original concrete `std::io::Error` as their source, not a newly inserted StoreError or Box node. A leaf capture refusal still has no source. The new I/O test's direct downcast is the correct chain contract.

4. **All four guards preserve exact old assertions.** The two budget guards in `net/acquisition/tests.rs:98-101` and `net/acquisition/tests/failures.rs:137-140` still require `Err(CrawlError::Fetch(FetchError::CaptureStore(StoreError::Capture(AcquisitionBudgetExhausted))))`. The two recovery guards at `net/acquisition/tests/recovery.rs:81-84,111-114` still require `RetainedOutcomeUnknown` under those same wrappers. `error.as_ref()` crosses only the new box; the guards do not broaden to arbitrary StoreError, arbitrary CaptureError, text matching, or any generic failure. Existing request counts, persisted-record assertions, recovery flow and attempt bounds remain unchanged.

5. **Regression intent matches the acceptance request.** `net/types/tests.rs` wires through `#[cfg(test)] mod tests` and uses the repository's fallible `check!` macro. The three new tests require both public error types to be strictly less than 128 bytes; cover both conversion entry points, exact typed refusal, Display and retryability through CrawlError; and preserve I/O path plus concrete source kind/detail through both wrappers. The coach fixture still checks Failed/invariant classification. These tests were inspected, not run. No actual byte size or passing test result is claimed.

## API, resource and async impact

This is an intentional public Rust payload-type change: direct constructors and nested destructuring require the new box shape. Ordinary conversion-based callers retain their API, and no erased trait-object error is introduced. The caller inventory supports complete in-workspace migration; external users, if any, must adapt direct pattern/constructor usage. No semver tool was run.

The additional allocation is one fixed-size `Box<StoreError>` per capture-store failure conversion, with no new allocation on successful acquisition from this delta. Allocation follows ordinary Box allocator/OOM behavior; this is not a claim of fallible allocation or strict post-initialization-allocation freedom. The approved target is error-layout budget below 128 bytes, guarded by authored tests. No measured latency, throughput, cache, or stack-speed improvement is asserted.

Power-of-Ten review: no new recursion, loops in production, unchecked arithmetic, ignored Result, unsafe, panic extraction, comments, or lint suppression appears in the delta. Production conversions are short and explicit; typed outcomes and minimum scopes are retained. The sole new test loop has two statically listed inputs. Rule 3's no-post-init-allocation ideal is not satisfied by boxing; the caller explicitly authorized this failure-path storage decision. Rule 10 execution evidence remains root-owned. The diff adds no await, spawn, lock, cancellation transition, I/O effect or retry; broader async behavior and service boxing are outside this review.

## Evidence and limits

Executed read-only validation:

- `(cd source && sha256sum -c ../proposal-pins.sha256)`: exit 0, seven OK.
- `(cd base && sha256sum -c ../source-pins.sha256)`: exit 0, six OK.
- `sha256sum changes.patch proposal-pins.sha256`: exact caller hashes above.
- Per-file `diff -u` of detached output against live: six empty diffs; one formatting-only diff retained.
- `rg -n --glob '*.rs' 'CaptureStore|impl From<census_store::StoreError> for FetchError|impl From<census_store::CaptureError> for FetchError' crates`: caller/conversion inventory retained in `caller-inventory.txt`.
- `git diff --no-index --check` on detached base/source: no whitespace diagnostic; exit 1 denotes the differing trees and is not reported as a test or formatting pass.
- Added-line grep for unsafe, unwrap/expect/panic family, suppression/comments, await/spawn/Mutex: no matches.
- Source inspection of the seven-file patch, relevant definitions and exact locked derive/Box implementations as cited above.

All executable Rust acceptance remains unrun by instruction. Root should retain its Moon-only format/check/lint and the three new tests, existing coach classification tests and durable capture regressions against final integrated pins. Actual layout, compile success and runtime preservation remain acceptance evidence to establish separately.

## References read

- Live `AGENTS.md`, `ARCHITECTURE.md`, `.agents/sol-reviewer.md`.
- `durable-capture-attempt-contract/error-taxonomy.md` and proposal `README.md`.
- Recovery skills: `rust-contract/SKILL.md`, `holzman-rust/SKILL.md`, `async-rust-reviewer/SKILL.md`, `scott-ddd-refactor/SKILL.md`.
- Holzman references: `nasa-jpl-standards.md`, `latency-throughput-playbook.md`, `zero-cost-abstractions.md`, `mechanical-empathy-toolchain.md`.
- Async reference: `observability-reference.md`.
- Error/context files and lockfile pinned in `context-pins.sha256`; locked thiserror derive expansion and `AsDynError`; pinned Rust `Box<E>` Error implementation; `tools/fallible_checks.rs`.

No source finding requires repair within this narrowly authorized review. The retained manifests identify exactly what this conclusion covers.
