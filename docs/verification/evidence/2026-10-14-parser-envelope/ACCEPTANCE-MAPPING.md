# Requirement-to-evidence mapping

The adopted [contract](contract/PARSER-ENVELOPE.md), implementation [contract/limits](proposal/CONTRACT.md), and [independent review](review/REVIEW.md) define this narrow slice. [acceptance-map.json](acceptance-map.json) gives source/test pointers. All source paths below refer to preserved `source/crates/census-crawl/src/` unless specified otherwise.

| Requirement | Implementation and independent review | Execution or negative API oracle | Current evidence |
|---|---|---|---|
| Seal captured bytes and retained provenance; forbid recombination | `net/acquisition.rs`, retained `read.rs`/`session.rs`/`retention.rs`; review PE-1 | API probes 02–07, retained producer regressions | 27 acquisition and 3 durable-capture tests PASS; API probes PASS for their named operations |
| Derive verdict from the intact acquired body through one owner | `milesplit/owned/outcome.rs`; `fetch.rs` and `owned/effect.rs`; review PE-2 | Named sealed-byte/metadata regression; API probes 02–05 | Mapped selected tests PASS; API probes PASS for their named operations |
| Resource refusal returns original retained envelope/allocation | `outcome.rs` resource transition and both existing retention paths; review PE-2 | Named exact ArrayValues/locator/retention/pointer regression and existing resource-parity tests | Mapped selected tests PASS; API probes PASS for their named operations |
| Reject historical/durable incompatibility before parsing or journal effects | `outcome.rs::require_capture_context`; review PE-4 limitation | Both exact `CacheOccurrenceMismatch` tests | Both exact mismatch tests PASS; no exact arbitrary-context identity guarantee |
| Release resident body without altering metadata/verdict or returning transport authority | `outcome.rs::release_body`, `results/run/acquired.rs`; review PE-2 | Byte/metadata release regression; two existing bounded-buffer regressions | Release test and both bounded-buffer tests PASS; no performance claim |
| Erased raw capture remains untrusted | Both consuming `into_capture` methods | Historical reentry regression; API probes 04–06 | Mapped selected tests PASS; API probes PASS for their named operations |
| Synthetic verdict fixture is historical-only and absent from production | `#[cfg(test)]` outcome factory and migrated unit fixtures; review PE-3 | Sixth named regression; external and production-MileSplit absence probes 04/08 | Sixth named test PASS; compiler absence probes PASS |
| Preserve actual production owner/work binding without broadening variant-only guard | Immediate same-context acquisition/interpreter calls; review caller-chain context files | Acquisition target/store/grant and durable replay regression families | Source caller trace and selected acquisition tests PASS; no general envelope-to-context witness claim |
| Preserve M1 shared-file archive-lock behavior and caller migration | Shared patches and source pins; `DELTA-COVERAGE.md` | Acquisition/lock-order regressions and normal all-target compilation | Selected lock-order/acquisition regressions and all-target compilation PASS |
| Preserve historical/error/receipt payloads and persisted contracts | Unchanged raw serde plus accessor-only persistence writers; review persistence section | Exact malformed/resource/partial/revision/receipt replay tests in `review/REGRESSIONS.md` | Selected exact byte/parity/replay regressions PASS; no all-stores certificate |
| Preserve legitimate public read/raw-parser API while rejecting forbidden construction/mutation | `review/API-CHECKS.md` | Positive API probe 01 plus negative probes 02–08 | All eight expected outcomes PASS; cleanup/check/format PASS |
| Record exact failed/pending/passing command outcomes and final source | `raw/`, command receipts, source pins, gate status | Focused test lane, fmt/check/lint/scans, final commit match | Format/check/lint and fifth focused 107/107 PASS; four earlier failures retained; API probes PASS; size scan FAIL14 and seams FAIL3; comments/panic-extraction PASS |

## Executed regressions and incomplete compiler probes

All six new regression definitions in [TEST-COVERAGE.json](TEST-COVERAGE.json) match exact PASS lines in the fifth-run terminal log. They are included within the 107 distinct passing tests in [TEST-RESULTS.json](TEST-RESULTS.json), not an additional count. All nine specially named historical/resource/buffer oracles are reconciled in [NAMED-ORACLE-RESULTS.json](NAMED-ORACLE-RESULTS.json). The 3195 skipped tests are unselected coverage, not passes. First through fourth attempts failed before execution and remain preserved.

The [eight compiler snippets](API-PROBE-STATUS.json) remain a separate obligation. All eight probes executed. Probe 01 compiled, while 02–08 rejected every named operation with exact privacy/type diagnostics. Those results are independently reconciled in API-PROBE-STATUS.json and API-RESULTS.md; they are not inferred from all-target or behavior tests.

## Historical compatibility and authority

Exact payload equality asserted by the selected passing regression scenarios remains narrower than proving that every historical store or every external Rust consumer is compatible. The runtime envelope types are not serialized. `FetchOutcome` serialization, journal keys/phases/payloads, manifests/chunks, receipts, schemas and policy identifiers are not intentionally changed. Rust field construction/mutation is intentionally removed, so semver-compatible Rust source API behavior is not claimed.
