# Transport test-suite source review

**STATUS: REJECTED for executable acceptance, pending Root-owned compilation and execution.** This is not a claim that a test was observed failing. Source review permits the frozen patch to advance to integration gates.

Frozen patch: `b716c818490fde0f5bb052a82af20502b521ee7f4b55f4b8cd097ad515169f35`.

The final `TEST-INVENTORY.md` lists 22 newly authored tests across acquisition, cache, failures, grants, recovery and the actual crawl Run consumer. All are unrun. Module declarations are present. Fixtures use the common production reqwest dispatch path over loopback HTTPS, an explicit test trust root, fixed observation time, exact host mapping and joined futures. No production security bypass, spawned orphan worker, ignored test or sleep-based race setup was found in these new modules.

Meaningful strengthened assertions include original occurrence/date/body after Store reopen; equal-byte distinct occurrences; 304 original linkage and pointer invariance; no work result on metadata mismatch; actual response-refusal status/URL; malformed429 cooldown and no second request; truncated-body ResponseObserved failures and lifetime three-attempt exhaustion; target/consumed grant; prepared archive replay; and real Run projection exact rows/IDs/receipts preserved on reopen.

Open acceptance blockers:

- Q1: All compilation and execution are unrun by owner instruction. No test-suite approval until nonzero named test results exist against integrated source pins.
- Q2: Actual service/Restate owner composition, cancellation/drain and native replay remain outside these transport fixtures. These are mandatory separate lanes, not satisfied by helper calls or legacy service replay tests.
- Q3: Root must verify formatted production-size/AST limits and unchanged parser/resource regressions after integration.

The original empty-selection-tolerant `is_some()` consumer assertions were replaced by exact per-result-set row IDs/counts. The new truncated-body fact-key assertion was corrected from `/observed` to `/response` before freeze. No runtime results are fabricated.

See `source-review.md` for exact source findings and dispositions; `final-snapshot/TEST-INVENTORY.md` and `authored-test-function-locations.txt` identify the tests.
