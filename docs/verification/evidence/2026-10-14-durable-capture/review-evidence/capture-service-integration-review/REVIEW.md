# Narrow post-integration service review

Scope: Root's formatted history re-export, extracted Jobs test imports, revised attempt-cap assertions, and the three additional ServeOptions consumers found by the all-target check. Read-only source inspection; no test/build/format command or live mutation by this worker. Exact reviewed bytes are in reviewed-current-pins.sha256.

## Findings

1. History re-export repair is appropriate. wire/history.rs re-exports only census_domain::model::HistoryWindow and retains HistoricalProgress plus its existing meet/results terminal semantics. wire.rs historically exports HistoricalProgress and HistoryWindow, so removing the unused HistoryError import from its private history module does not remove a previously named public wire export. The constrained HistoryWindow and HistoryError remain publicly exported by census_domain::model, including the original fields, serde shape, constructors, bounds, date semantics and exact failure variants.

2. publish/admission_tests.rs now imports census_store::Store and crate::spawn::Spawner explicitly. Those names previously arrived through the parent module before Jobs was extracted to publish/runtime.rs. This is a test-scope import repair, with no production behavior change.

3. Root's OutcomeUnknown/Armed hop0/TransportFailed assertion and Interrupted inspection expectation match core terminal.rs and facts.rs. The reviewed live file still expected AcquisitionBudgetExhausted from reservation after Interrupted, but transition.rs returns RetainedOutcomeUnknown. This was reported immediately. The separate capture-service-admission-cap-test-delta/attempts.patch corrects that mismatch and adds an independent observed500/BodyIncomplete known-response exhaustion test. That patch remains proposed at the time of these current-byte pins; verify Root's applied and executed state separately.

4. The all-target ServeOptions consumer ledger includes crates/census-service/tests/fjall_restate_e2e/restate_endpoint.rs. Its three literals now each explicitly choose ServeCaptureProtocol::LegacyFresh (reviewed lines 137, 194 and 253), with the enum imported. They preserve the existing e2e lifecycle rather than silently selecting lifecycle4. Root owns the follow-up all-target result; this review does not infer it from source.

## Evidence boundaries

Root reported composed second-run results of 143 pass / 1 fail among 144 tests (nextest e91a3300-7512-40c5-98fd-37af6afe75a9), with all dedicated/split-capacity/cancellation cases passing. The one failure exposed the attempt-outcome distinction described above. This report records Root's execution result, not a worker-run result. Root also reported formatted service production files all at or below 300 lines. New SDK replay bridge tests are a separate worker's pending integration and are not included in this review.

Static disposition: history/import/consumer corrections are consistent with the retained contracts. The stale reservation-error assertion requires the supplied detached delta before claiming the attempt-cap bridge passes. Runtime acceptance and all-target completion remain Root-owned.
