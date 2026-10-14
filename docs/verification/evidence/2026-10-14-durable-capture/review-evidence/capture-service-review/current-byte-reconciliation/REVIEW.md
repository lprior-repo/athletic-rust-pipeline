# Service current-byte coverage reconciliation

Date: 2026-10-14 UTC. Static delta review only; Root owns every execution lane.

## Disposition

**All 40 requested paths have exact current-byte static review coverage. No new confirmed defect was found.** The current bytes matched all 40 hashes in `capture-evidence-proposal/review-gaps-by-owner.json`, both when copied and at final verification. `reviewed-current-pins.sha256` is the machine-readable current coverage manifest; `coverage.json` supplies the per-path disposition, prior baseline hash/provenance and classification. `source/`, `base/`, and `diffs/` preserve exactly what was compared.

This closes the specified current-hash review gap, not test or release acceptance. No builds, tests, formatters, linters, live edits, services or network operations were run. Root reported that serial all-target check 57308 was in progress; this review does not assert its outcome. Existing reports and original OPEN findings are retained unchanged.

## Delta classification

- **38 paths:** formatting, import ordering, test-module ordering or equivalent closure/block formatting only. All assertions, test inputs, bounded capacities, checked arithmetic, branch conditions, lock/admission order, outcomes and cleanup logic remain equivalent to the previously reviewed sources.
- **1 path, capture_admission/attempts.rs:** the approved distinction between retained unknown transport outcomes and known-response exhaustion, plus independent no-fourth-reservation checks.
- **1 path, tests/fjall_restate_e2e/restate_endpoint.rs:** import `ServeCaptureProtocol` and explicitly select `LegacyFresh` in the three existing `ServeOptions` literals. Existing native fixture lifecycle is preserved.

The formatting group includes every Spawner/dedicated/Region/drain descriptor source and fixture in the gap list. In particular, independent async/worker descriptor bounds remain N + N, the worker lane stays separate from Coordinator admission, queued unstarted jobs preserve cancellation behavior, and owned OS-thread handles remain retained through completion and drain re-entry. The changes do not move owner/permit drops, introduce a nested pool wait, weaken the capacity-one parent/child assertions, or change tracing propagation.

The three SDK replay bridge files only changed formatting. Their production method calls, protocol tags/completion IDs, exact output/state/error checks, durable snapshot comparisons, same-key readmission and drain reconciliation are unchanged. The earlier bridge review's evidence limits still apply.

## Attempt-cap regression semantics

`attempts.rs` now explicitly checks the real Store transition from a dispatch failure with `AcquisitionFailureV1::Transport` to `OutcomeUnknown { checkpoint: Armed(hop 0), cause: TransportFailed }`. After three such continuations, it expects `AcquireDecision::Interrupted` and `CaptureError::RetainedOutcomeUnknown` on another reservation. That matches `acquisition/facts.rs` and `acquisition/transition/work.rs`; it does not relabel an uncertain physical outcome as an ordinary exhausted failure.

The added `observed_retryable_response_failures_exhaust_the_original_owner_lifetime_cap` uses actual Store APIs to register the same original owner/target, reserve ordinal 1 through 3, arm hop 0, and record response status 500 before settling `BodyIncomplete`. It checks `FailedUncaptured { checkpoint: ResponseObserved(hop 0), failure: BodyIncomplete }`, then `Exhausted` and `AcquisitionBudgetExhausted`. This is the complementary known-response case, not a manually injected terminal or a copied protocol implementation.

Both cases take the before-snapshot after registering the final continuation but before the refused reservation. They compare exact raw `work/` and `slot/` namespace contents through bounded snapshot fingerprints and require exactly three reserved roots before and after. Thus these assertions prove refusal does not append a fourth reservation or mutate those namespaces; they do not claim continuation registration itself had no prior effect. The unchanged identity comparison in the unknown-outcome case and target-bound registration in both cases retain the original operation identity.

No physical HTTP request or native Restate restart is performed by these Store-level fixtures. Their runtime results remain distinct from the executed transport/SDK/native lanes.

## SelectedHistory constructor coordination

`jurisdiction/capture_admission.rs` is not one of the 40 gap paths. The requested supporting check confirmed its current constructor call passes `SelectedHistoryV1::new(request.history, year)?`; the value object's constructor enforces year membership and `ResultsCaptureScopeV1::new` copies the same window/year into the existing scope fields. The service still passes the actual key, request jurisdiction/run, retained manifest digest, source admission and refresh-derived cache intent. No caller bypass or default history was introduced. Exact supporting read hashes are recorded separately in `supporting-read-pins.sha256`; this does not replace the core review's ownership of that constructor migration.

## Baseline provenance and coverage boundary

For 36 paths the preserved corrected-v2 snapshot supplies the baseline. For the three SDK fixture paths the preserved replay-bridge snapshot supplies it. The native e2e helper's prior integration review retained a pin but no source snapshot, so its complete small delta was inspected against repository HEAD: only the enum import and three explicit fields differ. Its earlier integration disposition is consistent with that full delta.

For `attempts.rs`, the authoritative comparison in `diffs/09.diff` runs from the preserved, reviewed v2 bytes through all current changes. `diffs/09-cap-patch.diff` additionally preserves the narrower approved patch comparison. This avoids treating an unretained intermediate integration pin as if its bytes had been reconstructed.

`coverage.json` records all 40 paths without broadening this into another whole-service audit. The separate `supporting-read-pins.sha256` contains only the five constructor/terminal decision dependencies read to interpret the semantic deltas. Earlier static dispositions continue to supply review of unchanged behavior.
