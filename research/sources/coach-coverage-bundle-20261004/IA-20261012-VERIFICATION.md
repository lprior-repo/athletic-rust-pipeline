# Iowa durable coach refusal — 2026-10-12

Bead: `athletic-rust-pipeline-zp5t`. Reviewer/executor: Sol, `openai-codex/gpt-6.1-sol`.

## Decision and scope

Option (b): retain the registered `AccessBlocked` `RefusedSource` named
`ia_missing_coach_sources`; do not claim an armed lane or verified-empty coverage.
No production behavior change is needed. The Iowa registry already binds the refusal
and a nonterminal `Blocked` obligation. This change adds executable acceptance evidence.

Only `crates/census-service/src/restate_services/wire/coach_lanes/ia.rs` and this
research outcome/evidence directory are modified. Existing fixtures, historical
matrix/work orders and other jurisdictions are unchanged. No network fetch, native
Restate invocation, live-store access, service restart, commit or push is performed.

The caller explicitly accepts the evidence ceiling: production IA registry binding
through serialized `JurisdictionState`, real Fjall journal commit, closed-handle store
reopen, deserialization and production owed-work projection. This is not execution
of native Restate `record_plan`/`ctx.set`, restart recovery of Restate, or fresh national
coverage. Main owns appending the summary to the repository-wide dated evidence ledger.

## Binding source references

- `state-work-orders/group-b.md:279-304`: IA held inventory, fill routes, exact
  true-missing line and Bound refusal. No `state-work-orders/IA*` file exists; IA is
  the section in `group-b.md`.
- `STATE-COVERAGE-MATRIX.md:5-8,39`: historical prototype numerator/CCD denominator,
  not current accepted census coverage.
- `probes/gobound/FINDINGS.md:61-79,124-145`, `manifest.json:49-64,139-146` and the
  retained refusal captures: known IA routes were refused; this session reads held
  evidence and does not claim the October 4 status is a new live observation.
- `probes/iatrackcoaches/FINDINGS.md:28-36,119-125`: association-only affiliations
  and mailboxes are not school coaching assignments.
- `probes/sources/SRC-133/FINDINGS.md:14-26,84-90`: advisory-board evidence lacks
  mailboxes and tenure; a board seat is not a current school coaching assignment.
- `crates/census-crawl/tests/fixtures/bound/SOURCE.md:7-13,36-39`: the held Sioux
  Center 2026-27 staff capture lists coaches but has no cache capture timestamp.
  `research/sources/bound/SOURCE_REPORT.md:3-5,112-114` explicitly says no coach
  email and no capture timestamp. Do not manufacture either from an offline replay.
- `research/sources/state-assoc-plains/SOURCE_REPORT.md:107-139`: school/member
  inventory and historical opt-in Bound directory sample; associations publish no
  coach contacts. Historical adapter-status prose is superseded by existing `bound`.
- `crates/census-crawl/src/coach_lanes/` and `research/sources/ia/` do not exist in
  this checkout. The current adapter is `census_crawl::bound`; no second adapter is added.
- `crates/census-service/src/restate_services/wire/coach_lanes.rs:88-94,214-242`:
  registered IA binding, exact refusal and deduplication.
- `crates/census-service/src/restate_services/jurisdiction/stages.rs:141-171`:
  existing production `record_plan` calls the registry and saves the state. Untouched.
- `ARCHITECTURE.md:283-290`, `docs/OPERATIONS.md:88-91`: contacts require same-capture
  school role/program/mailbox plus current eligible tenure; source failure is not absence.

Exact retained reason:

> `ighsau.org` (girls union; site uncaptured — only PDFs held) · `gobound.com/ia/{ihsaa,ighsau}` (refused to this client) · `iowarunjumpthrow.com` (IATC assoc; E1 #7; probe seeds held, site uncaptured) · school sites for email (**email 11/1339 is the standout gap**).

## Executable behavior and evidence limits

Given the actual Iowa national source plan, when the registered IA binding is applied
twice, then there is exactly one IA `AccessBlocked` refusal with the exact reason,
the serialized state is unchanged, the obligation remains `Blocked`/owed without
acceptance or admitted evidence, and owed-work includes the exact refusal text.

Given that bound state, when it is serialized, durably journaled to a temporary
Fjall store, closed and independently reopened, then readback equals the original
payload, decoding retains the exact refusal and owed-work visibility, rewriting the
same logical journal key leaves exactly one entry and zero coach rows.

These are acceptance tests for existing behavior, not a claimed failing-first parser
regression or a repaired production defect. The former function-pointer signature
smoke test is replaced by executable registry behavior. No baseline/gate is weakened.

## Command observations and append-only ledger

`bd show athletic-rust-pipeline-zp5t` succeeded and showed OPEN before work;
`bd update athletic-rust-pipeline-zp5t --claim --session 20261012-fleet` succeeded.

`env -u CI tools/moon-local run pipeline:tests -- -E 'package(census-service) and test(coach_lane_ia)'`
(before test additions): exit 0; 4 passed, 880 skipped; run ID
`9ecf9e54-4290-441c-9998-975cd22fad8b`. No persistence scenario existed in that baseline.

`sha256sum research/sources/coach-coverage-bundle-20261004/probes/gobound/raw/http-status-evidence.txt research/sources/coach-coverage-bundle-20261004/probes/gobound/raw/www.gobound.com__ia-schools__403.body crates/census-crawl/tests/fixtures/bound/staff-siouxcenter-ia-girls-xc.html`
exited 0 and printed, respectively:

```text
2a07e93f27c5333b104a291399ba699eab0f433c24a24d013c5a1065bb788086
58bf2215b395dcac74c009aa98701854e43cbe54a1cd3a95fee6a647ca9910d4
dec19282ac4f4e3784f7b66619d43585b1e331cb12bedf44bad5e3bc22490c7e
```

[BLOCKED] HIGH crates/census-service/src/restate_services/national/publication_tests.rs — required formatter cannot resolve an unrelated missing module — `env -u CI tools/moon-local run pipeline:fmt` exited 1: `Error writing files: failed to resolve mod publication_tests: .../national/publication_tests.rs does not exist`; reported to Main, outside IA ownership.

## Report-only source policy observation

`research/sources/bound/SOURCE_REPORT.md:31-40` describes changing the user-agent
shape after a client-specific 403 to obtain a 200. That describes a source-policy
risk against `ARCHITECTURE.md:125-128`; this session does not reproduce a live request,
certify the workaround, edit its implementation/report, or widen this bead. Main
explicitly assigned it report-only, separate from this refusal acceptance.

## Round 2 observations

Initial raw command output is retained in `IA-20261012-initial-checks.txt`.

[BLOCKED] HIGH crates/census-service/src/restate_services/national/publication_tests.rs:9 — focused acceptance cannot compile a concurrent partially written module — `env -u CI tools/moon-local run pipeline:tests -- -E 'package(census-crawl) or package(census-service)'` exited 1 (nested Cargo exited 101): `this file contains an unclosed delimiter`, pointing to `fn publication_fixture_frames_preserve_payloads`.

[BLOCKED] HIGH crates/census-service/src/school_address/join/nces_tests.rs:26 — required workspace check cannot compile an unrelated test — `env -u CI tools/moon-local run pipeline:check` exited 1 (nested Cargo exited 101): `error[E0599]: no method named cloned found for enum std::option::Option<GradeSpan>`, at `public.grades().cloned()`; reported to Main.

[OPEN] MEDIUM crates/census-service/src/restate_services/wire/coach_lanes/ia.rs:197,223 — added tests required rustfmt normalization — `env -u CI tools/moon-local run pipeline:fmt` exited 1 with diffs shrinking the registry rebind expression and wrapping the `stats().tables.contains` assertion, alongside unrelated concurrent files. Applied exactly those two IA diffs; subsequent formatter evidence must resolve this entry.

Main reported that the initial IA test mistakenly called private `Store::count`.
This was repaired immediately to use public `Store::stats().tables` after reading
its actual signature. No Store API or production behavior changed. This is
delegated compile-error input, not a command result attributed to this executor.

## Final scoped acceptance and review

The caller subsequently authorized closing this IA slice based on the green IA
module run and workspace compilation, with the unrelated five broad-suite failures
and NCES formatting owned by other workers. These failures are not relabeled PASS.
No changes were made to their files or to `zup7`.

Green IA command (shell `pipefail` preserves the Moon exit through `tee`):

```sh
set -o pipefail && env -u CI tools/moon-local run pipeline:tests -- -E 'test(restate_services::wire::coach_lanes::ia::tests::)' 2>&1 | tee research/sources/coach-coverage-bundle-20261004/IA-20261012-tests-focused.txt
```

Observed exit 0; run ID `ad87deb3-02bf-4436-8fbf-935ce9089a6a`;
`5 tests run: 5 passed, 3631 skipped`. Both the registry idempotency/exact
owed-reason scenario and reopened-store refusal scenario passed. Full raw output
is retained at the `tee` destination.

```sh
set -o pipefail && env -u CI tools/moon-local run pipeline:check 2>&1 | tee research/sources/coach-coverage-bundle-20261004/IA-20261012-check-round3.txt
```

Observed exit 0: `Finished dev profile ... in 2.48s`; `Tasks: 1 completed`.

[FIXED] HIGH crates/census-service/src/restate_services/national/publication_tests.rs:9 — initial missing/unclosed concurrent module no longer blocks compilation — the final IA test command above exited 0, compiled the workspace test targets, and ran 5 passing IA tests; disposition `fixed_with_evidence` for the compile blockage only, not the three national behavior failures.

[FIXED] HIGH crates/census-service/src/school_address/join/nces_tests.rs:26 — initial `Option<GradeSpan>.cloned()` compile blockage no longer remains — final `pipeline:check` command above exited 0; disposition `fixed_with_evidence` for compilation only.

[BLOCKED] HIGH crates/census-service/src/school_address/join.rs:15 — intermediate workspace check observed a partial NCES cutover with missing `postal` module and missing `Job::record_postal`/`Job::visit` methods — `env -u CI tools/moon-local run pipeline:check 2>&1 | tee research/sources/coach-coverage-bundle-20261004/IA-20261012-check-final.txt` printed task failure/Cargo exit 101; the pipeline itself lacked pipefail, so its shell exit is not evidence of check success. Raw failure retained at the destination.

[FIXED] HIGH crates/census-service/src/school_address/join.rs:15 — intermediate missing-module/method compile blockage is resolved — final pipefail-protected `pipeline:check` command above exited 0; disposition `fixed_with_evidence`.

[FIXED] MEDIUM crates/census-service/src/restate_services/wire/coach_lanes/ia.rs:197,223 — IA formatting corrections are accepted by rustfmt — `set -o pipefail && env -u CI tools/moon-local run pipeline:fmt 2>&1 | tee research/sources/coach-coverage-bundle-20261004/IA-20261012-fmt-final.txt` exited 1 solely with NCES-path diffs, none in IA; disposition `fixed_with_evidence` for the IA diffs only.

[BLOCKED] HIGH crates/census-service/src/restate_services/national/publication_tests.rs — required broad focused suite retains three publication behavior failures — `env -u CI tools/moon-local run pipeline:tests -- -E 'package(census-crawl) or package(census-service)'` exited 1: completed national publication services `[]` instead of `["Report","Bests","Workbook","Census"]`, and failed-workbook/unready-seal cases returned `national finished without awaiting its publication stage`; raw full tool output `artifact://4386`, permanent exact excerpts `IA-20261012-tests-round2.txt`; disposition `blocker` for broad-suite certification, tracked by the caller under PublicationChain.

[BLOCKED] HIGH crates/census-service/src/school_address/join/nces_tests.rs — same broad suite retains two NCES/PSS name-only linkage failures — the preceding exact command reported `city=None linked=1 no_match=0` (expected zero links) and `PSS city=None decision=Linked(...)` (expected `NoMatch`); total suite summary `2090 tests run: 2085 passed (1 slow), 5 failed, 2 skipped`; disposition `blocker` for broad-suite certification, tracked by the caller under NcesAddressFill.

[BLOCKED] MEDIUM crates/census-domain/src/school_directory/tests/nces_join_tests.rs:1 — final required formatter still has unrelated NCES diffs — the pipefail-protected final `pipeline:fmt` command above exited 1; full output `IA-20261012-fmt-final.txt` names domain NCES test imports/setup and service school-address `apply.rs`, `attest.rs`, `nces_tests.rs`, `postal.rs`, `join.rs`, no IA path; disposition `blocker` for workspace formatting, owner handling explicitly required.

Reviewer assessment of the IA-only slice: exact contract reason is asserted in both
`RefusedSource` and `CoachLaneRefusal`, idempotency catches duplicate IA entries,
lost refusal/changed kind/reason/status and missing owed visibility all fail the
tests, and the durability scenario closes the writing handle before readback.
No current timestamp, mailbox, contact or acceptance is invented. No new production
allocation/parser/arithmetic/async path is introduced. Owned Rust lexical scan
found no comments, unsafe, unwrap/expect, panic/todo/unimplemented/unreachable/dbg,
lint suppression or documentation attributes.

Truth-serum/evidence packaging disposition: scoped refusal evidence is exercised
and acceptable at the caller's stated ceiling; workspace-wide readiness remains
BLOCKED by the explicitly retained failures. No formal-verifier, mutation,
coverage, native Restate, release or fresh-national-run certification is claimed.
Standards references used: `holzman-rust/references/nasa-jpl-standards.md`,
`test-writer/references/rust-test-ecosystem.md`,
`test-reviewer/references/behavior-test-rubric.md`,
`truth-serum/references/adversarial-audit.md` and
`evidence-packaging/references/evidence-audit-checklist.md`.

## Tracker outcome

[BLOCKED] MEDIUM athletic-rust-pipeline-zp5t — ordinary close is blocked by the broader planning dependency — `bd close athletic-rust-pipeline-zp5t --session 20261012-fleet` exited 1: `blocked by open issues [athletic-rust-pipeline-zup7] (use --force to override)`.

The caller explicitly authorized `--force`: IA's own refusal acceptance is met;
`zup7` is the broader all-jurisdiction planning bead and remains open/unchanged.
The close reason records this rationale, scope limits and external gate failures.

[FIXED] MEDIUM athletic-rust-pipeline-zp5t — caller-authorized scoped close succeeded without changing the dependency — `bd close athletic-rust-pipeline-zp5t --session 20261012-fleet --force --reason "IA option (b): persisted AccessBlocked refusal carries the exact work-order reason; production registry -> serialized JurisdictionState -> reopened Fjall journal -> owed-work exact reason proved by 5/5 green IA tests. pipeline:check exit 0. Owner authorizes overriding broader all-jurisdiction planning dependency athletic-rust-pipeline-zup7, which remains open and unchanged. Native Restate record_plan unexecuted. Broad suite and NCES formatting failures are tracked by their owners. Evidence: research/sources/coach-coverage-bundle-20261004/IA-20261012-VERIFICATION.md."` exited 0 and printed `Closed athletic-rust-pipeline-zp5t`; disposition `owner_approved_no_action` for bypassing the broader dependency, not claiming it completed.

`bd show athletic-rust-pipeline-zp5t --json | jq '.[] | {id, status, close_reason}'`
exited 0 and reported `"status": "closed"` with the recorded scope/override rationale.
No code commit, Git push, Dolt push, merge, rebase or live-state change was performed.

## Requested post-closure formatter read

Main reported running global `pipeline:fmt-write` and requested a confirming read.
That report is not substituted for this reviewer's own observed result.

[FIXED] MEDIUM crates/census-domain/src/school_directory/tests/nces_join_tests.rs:1 — earlier NCES formatting diffs no longer appear — `set -o pipefail && env -u CI tools/moon-local run pipeline:fmt 2>&1 | tee research/sources/coach-coverage-bundle-20261004/IA-20261012-fmt-round4.txt` exited 1 with diffs only in `crates/census-crawl/src/recording/spool.rs`; disposition `fixed_with_evidence` for the earlier NCES formatting group only.

[BLOCKED] MEDIUM crates/census-crawl/src/recording/spool.rs:1 — requested formatter read cannot confirm zero workspace diffs — the preceding exact command exited 1, task `f2b3dabb`; raw full output retained at its `tee` destination. Only this peer-owned file appears in the diffs; no IA diffs. Main notified with the exact command/result; no peer file edited.

Bead remains closed under its already recorded scoped authorization. Own IA
format acceptance remains clean; global formatter remains blocked. The two earlier
broad-suite failure groups have not been re-executed or recertified in this round.

