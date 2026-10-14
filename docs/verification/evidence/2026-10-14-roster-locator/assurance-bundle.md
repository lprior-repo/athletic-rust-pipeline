# Assurance bundle

STATUS: UNVERIFIED

Scope: in-memory positive MileSplit roster locator preservation. The final source set is local commit f4506762506a4c2019b2b610f4554f3b77d005a1: the pinned 22-file overlay on parent 5ecad4e42525165be8a45a3f7ea9411347c35334. Packaging runs no project verification commands and approves no release.

## Requirement coverage

| Clause | Requirement | Named final tests | Scope limit |
|---|---|---|---|
| LP01 | Exact positive inner-li UTF-8 span and unchanged payload | 1/1 observed PASS | Coordinates are in-memory raw source DTOs, not accepted claims. |
| LP02 | Rejected rows retain original global ordinal gaps | 1/1 observed PASS | Ordinal is original matched-row position, not compact positive-vector position. |
| LP03 | Duplicate payloads retain distinct source occurrences | 1/1 observed PASS | Source occurrence multiplicity is not a claim of canonical athlete multiplicity. |
| LP04 | Capacity, row/field limits and exact failure locators remain unchanged | 5/5 observed PASS | LR01 adds exact 65,536/65,537-byte row and oversized-name/ID rejection cases. No exhaustive fuzz/property campaign or every possible boundary combination is claimed. |
| LP05 | Real Window and Records borrow exact original members and production counts/cursor consume them | 2/2 observed PASS | Borrow identity and actual apply behavior do not measure allocation or performance. |
| LP06 | Canonical projection and published graduation semantics remain unchanged | 4/4 observed PASS | Prefix differential is strengthened by the independently executed old-code V2 golden; it alone is not a historical oracle. |
| LP07 | Historical canonical/Journal/fact/effect bytes preserve their meanings | 1/1 observed PASS | Finite V2 fixture covers all five nonempty families and rejected locator; unfinished is null. No positive locator serialization is introduced. |
| LP08 | Partial/reopen/resource-refusal and replay compatibility | 6/6 observed PASS | Reopen covers old canonical state and counts; reapplication of a supplied roster is not durable positive-member readback. |
| LP09 | Ownership behavior and library/test/example/benchmark caller compatibility | 4/4 observed PASS | Focused names alone do not establish compilation of all targets. Full compiler/lint/format and named integration lanes require their separate terminal evidence. |

[Full names, declaration hashes and execution observations](traceability-matrix.jsonl). LP09 all-target compilation and integration coverage are separate from the focused-name counts and must be supported by the [actual command ledger](execution-ledger.json).

## Test, review and source evidence

- [Exact Root commands and logs](commands.md)
- [Final candidate source pins](candidate-final/source-files.json)
- [Source-origin and formatting reconciliation](source-origin.json)
- [Old-code golden history](baseline-history.md)
- [Current static review](review/REVIEW.md) and [its coverage/dispositions](review/coverage.json)
- [Initial LR01 finding retained separately](review-initial/REVIEW.md)

## Historical contract findings

The earlier [accepted-affiliation contract review](inputs/adopted-contract/REVIEW.md) records F1–F6 before implementation. Its narrative dispositions are preserved verbatim; this package does not silently convert them to canonical approval codes.

| Finding | Historical disposition and scope |
|---|---|
| F1: persistence bundled with first checkpoint | Resolved in the Root-adopted narrow contract; no persistence is added here |
| F2: unsupported positive-locator reopen claim | Resolved by the in-memory stopping condition; durable positive readback remains absent |
| F3: ambiguous borrowed-payload API | Resolved by exact located-slice API and one borrowed Records input slice |
| F4: genuine authority/HS-status gap | Blocker for positive acceptance, expressly outside this repair |
| F5: selected Workbook consumer branch absent | Preserved as a later explicit handler/snapshot contract; unchanged here |
| F6: later durable history/wire design not frozen | Root design decision before the later durable gate; no version is chosen here |

The actual implementation-review finding is LR01. Its source fix, execution evidence and current canonical disposition remain in the current review artifacts. No owner waiver or debt acceptance is invented.

## Missing assurance lanes

No new proof artifacts, native fault run, bounded parser-fuzz campaign, release/security qualification, whole-workload resource measurement or active-context truth-serum run is supplied. Broad specification qualification remains UNVERIFIED. [Remaining gaps and exact claim limits](qualification-boundary.md).
