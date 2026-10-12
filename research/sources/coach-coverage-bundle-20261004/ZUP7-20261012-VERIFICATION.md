# All-jurisdiction coach decisions — 2026-10-12

Bead: `athletic-rust-pipeline-zup7`. Executor/reviewer: Sol, `openai-codex/gpt-6.1-sol`.

## Scope and evidence ceiling

The caller assigns the main shared checkout and overrides the old generated bead's isolated-workspace/push defaults. Main explicitly grants `wire/coach_lanes.rs` in addition to the coach-lane directory. Existing 21 registered decisions keep their slugs, reasons and armed/refused semantics. The other 28 receive explicit nonterminal `Blocked` obligations and persisted `RefusedSource` decisions using the retained work orders' exact true-missing text. No contacts, mailboxes, successful acquisition or owner acceptance are invented. Historical captures, work orders and matrix are unchanged.

The acceptance ceiling follows the Iowa delivery: actual source planning and coach registry binding, serialized `JurisdictionState`, a real temporary Fjall journal, all writing handles closed before independent store reopening, and production owed-work projection. No native Restate handler invocation/restart, live fetch, running census mutation, service restart, write under `var/**`, release gate, durability drill, commit or push is performed. Main owns any `docs/**` ledger update. New registry decisions apply when production `record_plan` initially creates a plan; this work does not rewrite existing historical or live run plans.

Binding implementation references: `wire/coach_lanes.rs`, `jurisdiction/stages.rs:141-171` (initial plan binding/save; untouched), `jurisdiction/state.rs:78-105` (owed reasons; untouched), `wire/plan.rs:6-68` (existing serialized shape; untouched), `ARCHITECTURE.md:32-36,283-290,316-345`, ADR-024, and the Iowa delivery `IA-20261012-VERIFICATION.md`.

## Failing-first command ledger

All commands below run from the repository root with a 900-second timeout. `set -o pipefail` preserves the Moon exit through `tee`; the named destinations retain raw stdout/stderr.

[BLOCKED] HIGH crates/census-crawl/src/athleticlive/results/manifest.rs:11 — first acceptance compile encounters an unrelated missing `frozen` module — `set -o pipefail && env -u CI tools/moon-local run pipeline:tests -- -E 'test(restate_services::wire::coach_lanes::registry_tests::)' 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-tests-before.txt` exited 1; Cargo E0583. Reported to TeamsStage.

[BLOCKED] HIGH crates/census-crawl/tests/teams_source_refusals.rs:5 — second acceptance compile encounters an unrelated fixture module path mismatch — `set -o pipefail && env -u CI tools/moon-local run pipeline:tests -- -E 'test(restate_services::wire::coach_lanes::registry_tests::)' 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-tests-before-round2.txt` exited 1; Cargo E0583. Reported to TeamsStage.

[OPEN] HIGH crates/census-service/src/restate_services/wire/coach_lanes.rs:221 — in-scope jurisdictions lack any registered coach decision — `set -o pipefail && env -u CI tools/moon-local run pipeline:tests -- -E 'test(restate_services::wire::coach_lanes::registry_tests::)' 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-tests-before-round3.txt` exited 1; nextest run `f6fe13d1-1b7e-490f-8837-34a11095356b`, 5 tests run, 0 passed, 5 failed, 3678 skipped. Matrix reports `AL has no registered coach decision`; groups A/B/C report IL/OH/ND respectively.

[OPEN] HIGH crates/census-service/src/restate_services/wire/coach_lanes.rs:231 — a refused arm outside the sweepable plan mutates serialized state before returning an error — the same failing-first command reports failure of `coach_registry_failed_binding_leaves_serialized_plan_unchanged`; input is default `SourcePlan`, CA, school year 2026, revision 1, observation 2026-10-12. It returns `OutOfRange { field: "arm" }` but appends `ca_missing_coach_sources`. The fix moves fallible lane/arm validation before recording the refusal; successful old decisions retain their serialized values and ordering.

[BLOCKED] HIGH crates/census-review/src/revocation.rs:174 — initial post-fix acceptance compile encounters concurrent review-intake caller migration — `set -o pipefail && env -u CI tools/moon-local run pipeline:tests -- -E 'test(restate_services::wire::coach_lanes::)' 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-tests-after.txt` exited 1; E0425 missing `next_for_revocation`, E0061 three-argument callers to four-argument `next` in lib.rs:80 and review_intake_tests.rs:77. Reported to ReviewScan; no out-of-scope repair made here.

## Acceptance scenarios and review approach

Each retained work-order group independently enumerates its jurisdiction headings (15/17/17). For each row, assertions bind the real national source plan and registered decision, require exactly one jurisdiction-specific refusal with the exact work-order reason and kind, re-bind three times and compare serialized bytes, and assert actual owed-work output. A blocked decision must have the exact obligation refusal, no fabricated evidence/acceptance and an owed coach lane. An existing armed decision must retain nonzero schools/contact-row evidence and a real sweepable engine arm. Each group durably journals every state, closes the writer, reopens Fjall, compares serialized state again, reasserts the decision/owed reason, and rewrites the logical journal keys without duplicates or any new coach rows.

The denominator test independently walks `UsJurisdiction::CENSUS_SCOPE` and requires all 49 named decisions. The existing unregistered-jurisdiction test is migrated to explicit Alaska/Hawaii exclusion, not deleted or weakened into a no-op. Additional failure atomicity regression was demonstrated failing before repair. Removed `allow(dead_code)` attributes are replaced by test-only compilation for genuinely test-only acceptance helpers, not lint suppression. No serialized domain type or plan shape changes.

Group-C new refusal strings preserve the exact multi-line bullet block; the five pre-existing group-C decisions keep their established ` · ` rendering without final bullet punctuation. Tests normalize only that pre-existing representation, while comparing the complete source text for new entries. No missing site, blocked reason, empty capture or later record is dropped.

## Per-jurisdiction retained decisions

This table records the intended registered decisions at observation 2026-10-12.
Runtime test observations follow below when executed. `A`, `B`, `C` refer to
`state-work-orders/group-a.md`, `group-b.md`, `group-c.md`. Every row, including
the five existing armed decisions, persists the named state's missing-source
refusal so owed-work can expose the exact remaining reason. New states are
refused instead of importing prototype counts as this run's executed work.

| Jurisdiction | Obligation decision | Arm/candidate | Exact refusal evidence |
|---|---|---|---|
| AL | Refused/Blocked (AccessBlocked) | coach_directories | B:438 |
| AR | Refused/Blocked | coach_directories | B:465 |
| AZ | Refused/Blocked | aia | A:101 |
| CA | Existing Armed | home_campus | A:35 |
| CO | Refused/Blocked | chsaa | A:134 |
| CT | Existing Armed | ciac | B:383 |
| DC | Refused/Blocked | coach_directories | C:348-350 |
| DE | Refused/Blocked | coach_directories | C:323-325 |
| FL | Existing Armed | home_campus | A:46 |
| GA | Refused/Blocked | coach_directories | B:76 |
| IA | Refused/Blocked (AccessBlocked) | bound | B:301 |
| ID | Refused/Blocked | coach_directories | C:120-122 |
| IL | Refused/Blocked | ihsa | A:68 |
| IN | Refused/Blocked | coach_contacts | A:145 |
| KS | Refused/Blocked | ks | C:272-274 |
| KY | Refused/Blocked | arbiter_orgs | A:156 |
| LA | Refused/Blocked | milesplit | A:167 |
| MA | Refused/Blocked | mstca | B:356 |
| MD | Refused/Blocked | coach_directories | B:328 |
| ME | Refused/Blocked | mpa | C:448-450 |
| MI | Refused/Blocked | coach_contacts | A:79 |
| MN | Refused/Blocked | mshsl | B:160 |
| MO | Refused/Blocked (AccessBlocked) | mtccca | B:244 |
| MS | Refused/Blocked (AccessBlocked) | coach_directories | B:494 |
| MT | Refused/Blocked | coach_directories | C:95-97 |
| NC | Refused/Blocked | coach_directories | B:104 |
| ND | Refused/Blocked | plain_names | C:44-46 |
| NE | Refused/Blocked | plain_names | A:178 |
| NH | Refused/Blocked | arbiter_orgs | C:398-400 |
| NJ | Refused/Blocked | home_campus | A:123 |
| NM | Refused/Blocked | coach_directories | C:222-224 |
| NV | Refused/Blocked | niaa | C:196-198 |
| NY | Refused/Blocked | milesplit | A:57 |
| OH | Refused/Blocked | ohsaa | B:49 |
| OK | Refused/Blocked | ossaarankings | B:272 |
| OR | Refused/Blocked | coach_contacts | C:246-248 |
| PA | Refused/Blocked | pa_piaa | B:217 |
| RI | Refused/Blocked | riil | C:373-375 |
| SC | Refused/Blocked | coach_directories | B:411 |
| SD | Existing Armed | bound | C:70-72 |
| TN | Refused/Blocked | tssaa | B:188 |
| TX | Refused/Blocked | coach_contacts | A:24 |
| UT | Existing Armed | uhsaa | C:171-173 |
| VA | Refused/Blocked | milesplit | A:112 |
| VT | Refused/Blocked | vpa | C:423-425 |
| WA | Refused/Blocked | milesplit | A:90 |
| WI | Refused/Blocked | wiaa | B:133 |
| WV | Refused/Blocked | arbiter_orgs | C:298-300 |
| WY | Refused/Blocked | coach_directories | C:145-147 |

Existing armed proof references are unchanged and checked by their retained
per-state suites: CA `var/home-campus-CA-20261004/pass2.log`; FL
`var/home-campus-CA-20261004/fl.log`; CT the retained CIAC FusionPoint adapter
body; SD the four retained Bound staff fixtures; UT the UHSAA directory and
three retained school profiles. A historical executed capture/replay is not
a fresh national run, and an Armed coach-row obligation does not itself prove
eligible recruiting mailboxes/current-tenure publication.

[BLOCKED] HIGH crates/census-service/src/restate_services/jurisdiction/stage_runs.rs:78 — second post-fix acceptance compile encounters concurrent meet API migration — `set -o pipefail && env -u CI tools/moon-local run pipeline:tests -- -E 'test(restate_services::wire::coach_lanes::)' 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-tests-after-round2.txt` exited 1; missing `jobs::meets_stage`, `RecordedSource`, stale tests and non-Display `HandlerError::to_string`, plus stale endpoint `NationalCensus::new(clock, jobs)` constructor call. Reported to MeetStream and PublicationChain/Main; no out-of-scope repair made here.

[OPEN] MEDIUM crates/census-service/src/restate_services/wire/coach_lanes/missing.rs:198 — new registry additions require rustfmt normalization — `set -o pipefail && env -u CI tools/moon-local run pipeline:fmt 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-fmt-before.txt` exited 1 with owned diffs in missing.rs and registry_tests.rs plus unrelated concurrent paths. Applied exactly the owned rustfmt corrections through anchored edits; subsequent formatter evidence is required.

## Formatting ownership transfer and lexical review

Main explicitly transferred final global rustfmt writes to its landing step
and accepted `pipeline:fmt` output containing no coach-lanes diff as this
bead's formatting acceptance (stable-window message, 2026-10-12). This is not
a claim that workspace rustfmt has exited zero.

[FIXED] MEDIUM crates/census-service/src/restate_services/wire/coach_lanes/missing.rs:198 — owned registry formatting corrected without touching sibling files — `set -o pipefail && env -u CI tools/moon-local run pipeline:fmt 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-fmt-after.txt` exited 1 on unrelated paths; searching that retained output for `^Diff in .*coach_lanes` returned no matches. Owned formatting acceptance is met under Main's explicit transfer; global formatting remains Main's landing obligation.

The scoped forbidden-construct search for `unsafe`, unwrap/expect calls,
panic/todo/unimplemented/unreachable/dbg macros, lint/doc attributes and
Rust comments returned no matches in coach_lanes.rs or coach_lanes/**.
It does not stand in for Clippy, formal verification or workspace gates,
none of which are being claimed for this bead.

Registry arm preflight now precedes refusal mutation. The demonstrated CA
failure input is a default SourcePlan with no sweepable `home_campus` arm;
before the repair it returned an arm error while adding a CA refusal.
Afterward the regression requires the same error and byte-identical plan.
Existing acceptance constructors used only by tests are cfg(test), rather
than hiding dead code beneath module-level lint allowances.

[BLOCKED] HIGH crates/census-service/src/restate_services/national.rs:24 — next final acceptance attempt encounters another writer's incomplete test-module save — `set -o pipefail && env -u CI tools/moon-local run pipeline:tests -- -E 'test(restate_services::wire::coach_lanes::)' 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-tests-after-round3.txt` exited 1 with E0583 for missing national/reconciliation_tests.rs after waiting for the shared build lock. No scoped tests executed. PublicationChain confirmed the declaration was Main's intervening change; Main was notified to complete its module.

[FIXED] HIGH crates/census-service/src/restate_services/national.rs:24 — national test-module integration no longer blocks compilation — `set -o pipefail && env -u CI tools/moon-local run pipeline:check 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-check.txt` exited 0, finishing dev profile in 8.81s after waiting for the build lock. This also clears the earlier crawl, review, meet and endpoint compile blockers at this observation.

[BLOCKED] HIGH crates/census-service/src/restate_services/ingest/raw.rs:24 — subsequent focused test compilation encounters another concurrent cutover's untyped Result — `set -o pipefail && env -u CI tools/moon-local run pipeline:tests -- -E 'test(restate_services::wire::coach_lanes::)' 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-tests-after-round4.txt` exited 1 with E0283: `Ok(request)` cannot infer the `Into<JobError>` error type passed to blocking. No coach tests executed; reported to ByteBudget.

[FIXED] MEDIUM crates/census-service/src/restate_services/wire/coach_lanes.rs:1 — final format read remains clean for the owned scope — `set -o pipefail && env -u CI tools/moon-local run pipeline:fmt 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-fmt-final.txt` exited 1 with diffs only in census-crawl recording/spool.rs and census-service jurisdiction/athleticlive/tests.rs (both out of scope); there are no coach-lanes diffs. Main had run global fmt-write before this read; these later sibling saves require their own normalization. Main's scoped fmt transfer still applies.

[FIXED] MEDIUM crates/census-service/src/restate_services/wire/coach_lanes.rs:1 — workspace formatter now also exits zero — `set -o pipefail && env -u CI tools/moon-local run pipeline:fmt 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-fmt-green.txt` exited 0 with one completed task, 5.584s task-run wall time and no diffs after Main's final normalization. Both the scoped and full format reads have now been observed.

[BLOCKED] HIGH crates/census-service/src/restate_services/national/reconciliation.rs:61 — fifth focused acceptance compilation encounters the newly landed reconciliation API cutover — `set -o pipefail && env -u CI tools/moon-local run pipeline:tests -- -E 'test(restate_services::wire::coach_lanes::)' 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-tests-after-round5.txt` exited 1 with E0308 (`ctx.set` requires borrowed key, advice run returns TerminalError where HandlerError is required) and E0061 (endpoint constructor now lacks the newly required Jobs argument). No scoped tests executed; Main notified. The ingest inference blocker is absent from this attempt.

The independent retained-proof read also ran, without fetching or mutating
`var/**`:

```sh
set -o pipefail && sha256sum var/home-campus-CA-20261004/pass2.log var/home-campus-CA-20261004/fl.log research/sources/coach-coverage-bundle-20261004/probes/ciac-fpsports/raw/directory-SchoolLevelID-1.adapter.body crates/census-crawl/tests/fixtures/bound/staff-yankton-sd-girls-xc.html crates/census-crawl/tests/fixtures/bound/staff-yankton-sd-boys-xc.html crates/census-crawl/tests/fixtures/bound/staff-whiteriver-sd-girls-xc.html crates/census-crawl/tests/fixtures/bound/staff-faith-sd-boys-xc-empty-staff.html crates/census-crawl/tests/fixtures/uhsaa/directory.html crates/census-crawl/tests/fixtures/uhsaa/profile-alta.html crates/census-crawl/tests/fixtures/uhsaa/profile-murray.html crates/census-crawl/tests/fixtures/uhsaa/profile-herriman.html | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-retained-armed-digests.txt
```

Observed exit 0 and eleven physical SHA-256 values. The CA log digest was
`b1658ef1f62f9373fca3b4a1c4bd24b6c87a1e46818bea3ead435374c9da5cd4`,
FL `34d59e4eb1529b93a64be617e3d84847838c61d9ae86fe2b4a89bbedba0129b9`,
CT `a872654b3c9450a484c6f68ae75f9bd32c2523af0f7a2c118f8bb823f1cfc634`.
The remaining eight values are retained verbatim in the linked output.
File existence and a digest are not a substitute for the pending behavior
test assertions about parsed coach counts, role rows and durable decisions.

## Final acceptance observations

[FIXED] HIGH crates/census-service/src/restate_services/wire/coach_lanes.rs:213 — all 49 applicable jurisdictions now have a registered Armed or explicit Blocked decision, including the previously absent IL/OH/ND/AL inputs — `set -o pipefail && env -u CI tools/moon-local run pipeline:tests -- -E 'test(restate_services::wire::coach_lanes::registry_tests::)' 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-registry-final.txt` exited 0; Nextest run 9677406b-933a-407e-ad2f-03518de97c05 ran 5 tests, 5 passed, 3697 skipped. Before wiring, the same test namespace's retained round-three execution ran 5 tests and failed all 5 on missing AL/IL/OH/ND decisions and the mutation-on-error regression.

[FIXED] HIGH crates/census-service/src/restate_services/wire/coach_lanes.rs:216 — failed CA bind no longer changes the serialized plan — the same registry-final command passed coach_registry_failed_binding_leaves_serialized_plan_unchanged; the failing-first observation in tests-before-round3 was a returned arm error with newly appended CA refusal bytes.

[FIXED] HIGH crates/census-service/src/restate_services/national/reconciliation.rs:61 — all transient external compilation blockers have cleared for the final behavior run — `set -o pipefail && env -u CI tools/moon-local run pipeline:tests -- -E 'test(restate_services::wire::coach_lanes::)' 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-tests-after-round6.txt` exited 0; Nextest run 76511039-ee4c-4731-b936-9e0095e0bc89 ran 105 tests, 105 passed, 3597 skipped. Existing per-state source-count/digest/arm-wiring suites and every new group/coverage/atomicity test passed. This supersedes the earlier crawl/review/meet/national/ingest/endpoint BLOCKED entries without deleting their observations.

[FIXED] HIGH crates/census-service/src/restate_services/wire/coach_lanes.rs:206 — integrated workspace check meets the required exit-zero acceptance — `set -o pipefail && env -u CI tools/moon-local run pipeline:check 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-check-final.txt` exited 0; dev profile finished in 5.78s, one completed task, 8.555s task-run wall time.

### Binding contract mapped to executed scenarios

| Criterion | Executed witness | Observed result |
|---|---|---|
| Every applicable jurisdiction decided | every_in_scope_jurisdiction_coach_lane_is_armed_or_refused_by_name | Independent CENSUS_SCOPE denominator 49; all named jurisdictions have matching jurisdiction, expected Armed/Blocked status |
| Group A exact decisions | coach_registry_group_a_rebind_and_reopen_preserve_exact_decisions | 15 jurisdictions; exact retained work-order refusal, correct status, three byte-stable rebinds, independent real Fjall reopen/readback |
| Group B exact decisions | coach_registry_group_b_rebind_and_reopen_preserve_exact_decisions | 17 jurisdictions; same assertions, including IA/MO/AL/MS AccessBlocked categories |
| Group C exact decisions | coach_registry_group_c_rebind_and_reopen_preserve_exact_decisions | 17 jurisdictions; same assertions; new multi-line reasons compared in full, legacy five reasons preserve established rendering |
| Idempotent registration | all three group tests | Exactly one state missing-source refusal after each repeated bind; byte-identical serialized JurisdictionState; journal-key count unchanged after rewritten readback |
| Owed-work reason/progress | all three group tests | Exact `refused {state_slug}: {work_order_reason}` exposed for all 49; Blocked stage remains owed for 44; five existing Armed decisions have nonzero retained evidence and wired sweepable engine arms |
| Real retained evidence | existing CA/CT/FL/SD/UT suites in the 105-test run | Source counts, capture SHA-256 values, retained probe metadata and real engine arms pass, without live requests |
| Failure has no partial registration | coach_registry_failed_binding_leaves_serialized_plan_unchanged | CA missing-arm error preserves serialized SourcePlan |
| Compilation | check-final exact command above | Exit 0 |
| Formatting | fmt-green exact command above | Exit 0, no workspace diffs at the observed read |

Final registered decision totals: **49 / 49**, with **5 Armed** (CA, CT, FL,
SD, UT) and **44 Refused/Blocked**. All 28 additions are genuine refusals;
existing 21 slugs and lane semantics are unchanged. Missing sources remain
visible rather than accepted/sealed; the parent wiring bead's completion
does not assert that the 44 states' source-acquisition work is finished.

Final scoped review ledger state: **OPEN 0, BLOCKED 0**; historical failed
rounds are preserved above and superseded by the reproduced green runs.

### Evidence limits

This executes offline registry binding, typed jurisdiction state, actual
Fjall journal persistence with independent reopen/readback and owed-work.
It does not execute the live Restate service, mutate existing national
plans, retrofit an already-bound plan, deploy this code or prove national
population/mailbox completeness. Existing plan-fingerprint short-circuit
behavior and the live run are outside this ownership. No source fetches,
fabricated contacts/mailboxes, acceptance events or writes under var/**;
no docs/** edits, commits, pushes, resets or lifecycle changes to live services.

The no-invented-acceptance statement above concerns runtime business
decisions: all newly refused obligations remain unresolved with no acceptance.
Existing retained per-state unit tests intentionally exercise their
pre-existing owner-acceptance constructors as test values; this bead adds
no acceptance event to a live or national journal.

## Bead closure

```sh
set -o pipefail && bd close athletic-rust-pipeline-zup7 --session 20261012-fleet --reason-file research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-VERIFICATION.md 2>&1 | tee research/sources/coach-coverage-bundle-20261004/ZUP7-20261012-bead-close.txt
set -o pipefail && bd show athletic-rust-pipeline-zup7 --json | jq '.[] | {id,status,closed_at}'
```

Observed both commands exit 0. Close printed `✓ Closed athletic-rust-pipeline-zup7`;
readback reported `status: closed`, `closed_at: 2026-10-12T13:10:08Z`.
`--force` was not needed or used. The complete verification ledger and
per-jurisdiction matrix were supplied as the close reason. No bead push
or source-control commit/push was performed under the caller's shared-tree
handoff constraints.

Reviewer model: `openai-codex/gpt-6.1-sol`.

VERDICT: SATISFIED
