# 2026-10-14 positive roster locator evidence

Technical qualification: **UNVERIFIED**. This is an additive checkpoint for the narrowly adopted in-memory MileSplit roster repair, separate from the prior durable-capture qualification.

**83 distinct final-candidate tests passed**: 75 focused library cases, five roster-observation/jurisdiction integration cases, and three boundary/parity cases. Final source lint, workspace/all-target/all-feature compilation and format check passed. Raw terminal logs and separately attributed Root numeric exits are retained. The earlier 73-test run and both old-code baseline probes are history and are not added to the final count.

The exact repair is local source commit `f4506762506a4c2019b2b610f4554f3b77d005a1` (parent `5ecad4e42525165be8a45a3f7ea9411347c35334`), with all 22 commit paths independently matched to the tested snapshot and bounded review. The worktree was observed clean; no remote merge or publication is claimed. The only first-to-final change adds the two LP04 oversized-row/field regressions; the reviewer closed LR01 as `fixed_with_evidence`. No production bytes changed between those candidate states.

- [Full report and exact claim boundary](evidence/2026-10-14-roster-locator/REPORT.md)
- [Actual commands and raw logs](evidence/2026-10-14-roster-locator/commands.md), [execution ledger](evidence/2026-10-14-roster-locator/execution-ledger.json) and [test overlap](evidence/2026-10-14-roster-locator/test-overlap.json)
- [Exact commit coverage](evidence/2026-10-14-roster-locator/commit-coverage.json), [final source hashes](evidence/2026-10-14-roster-locator/candidate-final/source-files.json), [LP01–LP09 traceability](evidence/2026-10-14-roster-locator/traceability-matrix.jsonl) and [review](evidence/2026-10-14-roster-locator/review/REVIEW.md)
- [Baseline-probe history](evidence/2026-10-14-roster-locator/baseline-history.md): V1 passed with empty teams; stronger V2 raw output and the LF-normalized installed fixture retain distinct hashes
- [Mechanical integrity results](evidence/2026-10-14-roster-locator/integrity-check.json) and [evidence decision](evidence/2026-10-14-roster-locator/final-evidence-decision.md)

The supported change preserves exact source coordinates through parsing and the in-memory service projection boundary. Positive locators are not durable or recoverable from the old Journal. Accepted affiliation, source authority, dated HS status, retained roster interpretation, selected Workbook support and broader release/native-fault/fuzz/security/resource qualification remain separate obligations. Active-context truth-serum remains unavailable; scoped green lanes do not approve landing, production completion or recruiting publication.
