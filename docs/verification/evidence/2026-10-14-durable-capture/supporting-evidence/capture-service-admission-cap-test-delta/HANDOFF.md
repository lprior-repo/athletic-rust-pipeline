# Capture admission attempt-cap regression delta

TASK: repair the test oracle after the observed composed run and keep outcome uncertainty distinct from known retryable-response exhaustion.

OWNERSHIP: only this detached delta directory was written. Root owns integration and every execution command. Frozen v1/v2 source proposals are unchanged.

INPUT: exact current formatted service capture_admission/attempts.rs, including Root's OutcomeUnknown/Armed hop0/TransportFailed assertions and Interrupted decision repair.

OUTPUT: one contextual patch for that test file. The existing unknown-outcome path now expects RetainedOutcomeUnknown on a refused fourth reservation. A separate original-owner continuation test records actual HTTP 500 response facts through Store and settles BodyIncomplete through fail_acquisition_response; it asserts observed status500, FailedUncaptured/ResponseObserved hop0/BodyIncomplete, Exhausted, then AcquisitionBudgetExhausted.

Both cap paths use bounded public Store snapshot APIs to compare exact raw journal namespace fingerprints before/after refusal for work/ and slot/. Both assert exactly three reserved roots before and after, so refusing another attempt cannot silently append a fourth reservation. No terminal or journal row is injected manually.

ACCEPTANCE: static API/source inspection, exact base comparison and contextual whitespace check only. No test, build, formatter, network or live write was run. Root must format and execute both named tests after applying this delta. No physical HTTP or native Restate qualification is claimed by these Store-level bridge tests.

HANDOFF: attempts.patch, base/, source/, base-pins.sha256 and source-pins.sha256. This delta does not overlap the core worker's SelectedHistoryV1 caller grouping in capture_admission.rs or the SDK replay worker's added tests.
