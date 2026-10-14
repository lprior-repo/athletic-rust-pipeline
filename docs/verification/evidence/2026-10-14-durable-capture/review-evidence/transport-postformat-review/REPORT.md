# Transport post-format delta review

## Scope and result

Reviewed only the integrated transport proposal, additive coach regressions, Root's coach consumer classifications and the missing net test import. No broader audit, live writes, build, test, formatter, network or commit was performed. No new semantic defect was found in this delta.

The frozen transport input is changes.patch SHA256 b716c818490fde0f5bb052a82af20502b521ee7f4b55f4b8cd097ad515169f35. The additive coach input is changes.patch SHA256 975eb50af500087a067818d08b48640b360460b7ffaa417d87752ce2a23ed431. Exact reviewed integrated bytes for105 paths are retained under source/ and pinned by reviewed-live.sha256.

## Delta checks

- Forty original proposal paths differ byte-for-byte after formatting. Source/token delta inspection found whitespace/line wrapping, trailing commas, import/reexport/module ordering and expression block insertion only; reviewed changed blocks preserve expression values and control flow.
- The existing net/execute/tests.rs import adds FetchOptions, required after the execute implementation was split. No test condition or assertion was weakened.
- Root's probe_utils.rs maps CaptureStore, CaptureDomain and Capture FetchErrors plus direct CrawlError::Capture to invariant, retaining the original diagnostic.
- Root's research_failure.rs maps these variants to Failed. The additive tests cover all three FetchError forms and direct poisoned/consumed/target context refusals, checking both research/probe classifications and nonretryability.
- The full Run::request fixture retains exact three projected source-key assertions, date and projection receipt fields, then actual Store reopen and unchanged performance/source-receipt/acquisition roots plus original occurrence. No assertion was deleted in formatting.
- Actual malformed429, truncated-body,304 pinned comparison, same-second two-dispatch, eligible reuse, grant refusal and explicit deterministic interruption tests retain their frozen conditions.
- Reviewed production Rust files stay within300 lines after formatting; production-lines.json records the measured paths. The309-line TLS fixture is a test module, not a production file.

## Execution evidence attribution

Root reported all22 new transport tests and both coach tests passing in the composed lane. The broader composed run had143 passes and one unrelated service assertion failure being corrected by its owner. This worker did not execute or independently certify that lane. Basic check/lint were running when this review began; this report does not claim their completion, native Restate cancellation/drain, kill/restart qualification, backup/restore qualification, or full gates.

The review snapshot is bounded to the pinned bytes. Later Root lint repairs require their own exact delta check.
