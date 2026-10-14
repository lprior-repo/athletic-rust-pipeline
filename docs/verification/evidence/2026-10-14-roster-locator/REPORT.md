# Positive roster locator: additive verification evidence

Technical qualification: **UNVERIFIED**. This is a detached evidence proposal, not permission to land, release, run production or publish recruiting data.

83 distinct final-candidate tests have terminal passing evidence. Historical probes and the first-candidate 73-test run are excluded from that count.

The repair, now local source commit f4506762506a4c2019b2b610f4554f3b77d005a1, is 22 files: nine production files, nine migrated existing test files, three new test modules and one golden fixture. All nine production files are identical between first and final candidates; the sole follow-on delta is two LP04 regressions plus their helper in the existing located_roster test module.

## Exact source and claim boundary

Production baseline is `3708d6bab0b08c390887c562a89e8c6714255a5d`. Integration HEAD is `5ecad4e42525165be8a45a3f7ea9411347c35334`, a documentation-only successor for these paths. The tested 22-file overlay is now local commit `f4506762506a4c2019b2b610f4554f3b77d005a1`, tree `a5d6f335dbba4c9fbecc8e23a9f69c14f8bab303`. [Exact commit coverage](commit-coverage.json) verifies all 22 commit paths match the [final source hashes](candidate-final/source-files.json) and [full contextual patch](candidate-final/changes.patch). [First-candidate sources](candidate-first/source-files.json) and the [one-file delta](candidate-deltas.json) remain separate. The source worktree was observed clean after this commit. No remote merge or publication identity is claimed.

The narrow supported wording is positive roster locators preserved through parsing and the in-memory service projection boundary, with canonical/Journal/effect compatibility constrained by named tests. Coordinates describe original UTF-8 inner-li bytes and original matched-row ordinals. They do not establish accepted affiliation or durable positive-member readback. See the [adopted contract](inputs/adopted-contract/IMPLEMENTATION-CONTRACT.md) and [specification/qualification boundary](qualification-boundary.md).

## Root execution evidence

| Lane | Source state | Result | Observed tests | Outer exit |
|---|---|---|---|---|
| baseline-probe | baseline-old-production-test-probe | PASS | 1 test run: 1 passed, 3271 skipped | 0 |
| baseline-probe-v2 | baseline-old-production-test-probe | PASS | 1 test run: 1 passed, 3271 skipped | 0 |
| format-first | candidate-first | PASS | No test count | 0 |
| tests-first | candidate-first | PASS | 73 tests run: 73 passed, 3205 skipped | 0 |
| format-second | candidate-final | PASS | No test count | 0 |
| tests-second | candidate-final | PASS | 75 tests run: 75 passed, 3205 skipped | 0 |
| integration-first | candidate-final | PASS | 5 tests run: 5 passed, 8 skipped | 0 |
| boundary-parity-first | candidate-final | PASS | 3 tests run: 3 passed, 0 skipped | 0 |
| lint-first | candidate-final | PASS | No test count | 0 |
| check-first | candidate-final | PASS | No test count | 0 |
| fmt-check-first | candidate-final | PASS | No test count | 0 |

[Exact actual commands](commands.md), [Root-attributed numeric receipts](root-command-receipts.json), [execution ledger](execution-ledger.json), [selected test identities](selected-test-results.json), and [overlap analysis](test-overlap.json) retain evidence without double counting. Raw logs are unchanged; outer exits come from Root attestations of actual tool results, not raw tool exports. Remote-service connection warnings are retained in the logs and are not hidden.

Pending or unverified recorded lanes: none. Recorded failing lanes: none. Absence of a lane from this ledger means no execution evidence is supplied, not PASS.

## Historical golden and regression closure

[Baseline history](baseline-history.md) retains both separately executed old-production probes. V1 passed but had an empty team family, making it weaker than intended. V2 added exact nonempty-output guards before export and is the compatibility oracle. Its original raw output is 4,971 bytes; the installed golden is exactly that file plus one terminal LF, 4,972 bytes. Both hashes are preserved separately in [baseline provenance](baseline-provenance.json). Neither was regenerated from changed production code.

Initial static review identified LR01: LP04 lacked exact oversized-row/field rejection tests. Root added a 65,536/65,537-byte row case and oversized-name/ID cases, without changing production code. The final 75-test focused run includes both tests. [Initial review](review-initial/REVIEW.md), [current review](review/REVIEW.md), [review coverage](review/coverage.json) and the frozen candidate delta preserve the finding history. The final reviewer records canonical `fixed_with_evidence` for LR01. Its statements that later gates remain Root-owned describe the review-time boundary; those later terminal Root results are retained above. See [review reconciliation](review-reconciliation.json).

Current reviewer coverage status: `Approved for exact-delta static/behavior-suite review; broader integration/compiler/lint/format lanes remain Root-owned.`.

## Acceptance traceability

Every LP01–LP09 clause is mapped to exact test names, source declaration hashes, observed execution lanes and limits in [traceability-matrix.jsonl](traceability-matrix.jsonl). The [assurance bundle](assurance-bundle.md) is the readable index. A named-test PASS is not a proof that every requirement in the October 11 specification passes.

## Remaining qualification limits

Positive roster coordinates are not persisted by this slice. FetchOutcome still skips body on serialization; the historical Journal has no positive-member inventory. Current facts/counts can survive reopen while the precise positive coordinates cannot be recovered from that Journal. Accepted school affiliation, publisher authority, independently dated high-school status, selected Workbook evidence closure, durable roster interpretation and lifecycle adoption remain separate contracts.

Broader release/native-fault/parser-fuzz/named-proof/security/resource obligations remain unexecuted by this worker. No performance, allocation-free, ABI-layout, universal correctness, production-run completion or recruiting-publication claim follows. Historical capture-green evidence is not rerun evidence for this candidate.

The active-context truth-serum tool/skill is absent. Mechanical hash/log/JSON validation is not a substitute approval. See [truth-serum boundary](truth-serum-report.md), [integrity results](integrity-check.json) and [final evidence decision](final-evidence-decision.md).
