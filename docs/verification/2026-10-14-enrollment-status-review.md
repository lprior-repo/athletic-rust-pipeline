# Enrollment and comparable-result status: consolidated review

This is a changed-surface review and focused execution record, not acceptance of the entire
October 11 specification. The full main specification remains unchanged in this repository.

## Reviewed implementation and repairs

- Domain/Store installation configuration and the actual service/CLI enrollment path were
  independently reviewed against the explicit enrollment-only contract. The final proposal
  corrected early migration mutation, physical retained-artifact eligibility, historical
  schema-1 backup compatibility, whitespace lifecycle parsing and the malformed-restore
  test's unreachable candidate-validation branch. Atomicity covers the three-record durable
  enrollment batch; prior identity creation and external filesystem changes are separate.
- A separate acquisition-fence review added the actual fetch --refresh CLI regression with
  a loopback connection sentinel and retained logical/artifact witnesses. The Store test
  module split preserves all 22 moved callable bodies. All 45 named installation tests were
  matched to passing integrated core/service logs.
- Comparable result status review exposed order/replay defects beyond the original pair
  case. Terminal/numeric and event/team/provider-authority conflicts now retain evidence
  and quarantine selection. Missing provider authority is not equal authority. First-pass
  report assertions were strengthened before final green evidence. HyTek's historical
  eight-token classifier remains unchanged; the shared ten-token classifier serves only
  the migrated existing ten-token consumers.
- Policy-18 review covers immutable policy-17/16 preservation, nonempty historical archives,
  exact refusal variants, descriptor identity and actual publication receipt/token/recipe
  cutover. No historical mark shape, identity, policy fixture or artifact is rewritten.
- Root integrated all changes without replacing shared module registries, corrected explicit
  test module paths, updated the one stale future-lifecycle test from ceiling 2 to 3, and
  replaced the new modulo predicate with equivalent is_multiple_of after strict lint rejected
  it. No gate, test assertion or refusal was disabled to obtain a passing result.
- The previously committed bounded raw-reader/writer callback tests are included in this PR.
  They exercise the actual Store traversal and preserve exact writer errors through its bridge.

## Exact coverage and execution ledger

The companion `2026-10-14-enrollment-status-files.json` pins the reviewed implementation
commit and every changed file relative to the recorded main baseline. Review scopes are
attributed per file; an attributed scope is not a claim that every original detached byte
remained unchanged after integration. Root's integration deltas are described above.
The companion evidence JSON records selected lane IDs, counts, log hashes and explicit
failures/not-run boundaries. Documentation-only coverage bookkeeping is committed afterward,
so the ledger does not claim a recursive hash of itself.

## Acceptance boundaries still open

Enrollment-only capability intentionally refuses acquisition, serving and legacy publication.
It is not reusable production synchronization. Distinct durable acquisition-attempt identity,
qualified parser/application reuse, complete selected current-affiliation/cohort projections,
profile-bound workbook/normalized-snapshot parity and full completion evidence remain open.

Native enrollment precommit/postdurable fault/replay, actual predecessor executable refusal,
property/fuzz campaigns, mutation/coverage qualification, full release/security gates,
performance/resource measurement and final workbook readback are not established by these
selected lanes. The unchanged Unix-socket backup fixture is blocked locally at setup (EPERM);
the attempted escalated launcher failed before execution. These limitations must accompany
publication and survive the merge.

The current Beads export was unavailable; historical snapshots were not treated as current.
No live census or production Store/service was used. The home verification commands are in
`2026-10-14-installation-enrollment.md`; full specification acceptance mapping remains partial.

## Final integrated checks

Moon formatting, all-target/all-feature compilation and strict source lint passed (exit 0).
Selected lanes passed: core 303/303, actual service/CLI 43/43, complete profile descriptor
binary 17/17, crawl classifier consumers 3/3, and final reader/resource lane 20/20.
The last lane, `86c5185e-1c0e-4dd0-888d-2ce4b08d199a`, passed with 1725 skipped,
2.810 test seconds and Moon exit 0. It reran the five installation domain tests after the
lint correction and all 15 selected raw-reader/writer callback tests. Counts across lanes
are not summed as unique coverage because selections overlap. Historical failures and
remaining qualification limits above are retained.
