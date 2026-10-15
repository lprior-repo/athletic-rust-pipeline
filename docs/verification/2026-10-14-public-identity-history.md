# Public identity history foundation — 2026-10-14

R3 maps to `census-domain/src/public_identity_history/` and its library export.
Installation/establishment ordinals define stable public IDs. Immutable binding
claims and accepted-edge references support deterministic merges, precise edge
revocation, surviving-component checks, explicit splits and ambiguous predecessor
aliases. This is a pure candidate history; it is not an accepted or committed
store capability, and no existing athlete ID is rewritten.

Independent review repaired redundant-edge loss, component allocation before
admission, and lost correction history across later splits/merges. Bounded replay
checks head, sequence, payload digest, references, graph work and allocations.

Cloud validation, CI unset and MOON_RUST_BUILD_JOBS=1:
`tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-domain --lib -E 'test(public_identity_history::)' --no-capture --no-tests fail`
- Initial compilation FAILED: six errors from ambiguous test HistoryError imports;
  no tests ran. Fixed the explicit test import, without changing production behavior.
- Final: 50 PASS, 0 FAIL, 582 skipped; Nextest
  c72cac83-c58f-4e49-bff0-520603f1c935; 0.600s tests, 31.266s task.
- Moon formatting PASS on combined integration tree. All-target/lint PENDING.
- Full release, migration/recovery qualification, production and national run NOT RUN.

OPEN: owner-authenticated correction/binding closure; atomic application, history,
receipt, head and generation persistence; complete frozen ledger and commit-revision
witness; typed publication ID/representative projection; explicit historical adoption.
Unresolved binding reassignment and recurring A→B→A use of an already-seen candidate
remain refused. Caller references/affected-key lists cannot establish authority.
R3 is not closed by this foundation.

Matched-group extension: `EstablishMatchedGroup` accepts one exact SamePerson member
set with one distinct source binding per candidate, allocates one installation/ordinal
public ID, and retains one precisely revocable connection. Already-bound candidates,
duplicate/reused sources, mismatched membership and duplicate authority are refused.
Later split/revocation replay retains unaffected members; public IDs do not depend on
candidate ordering. This remains a pure candidate operation, not source authority.
The same Moon command above now passes 75 tests, 0 failures, 582 skipped (25 added),
Nextest `c4239d44-ce24-4e6a-9922-3d0951ce5d6d`; 0.614s tests / 40.455s task.
Independent review passed. Mixed multiple-new-candidate plus existing-component adoption
remains unsupported; callers must not synthesize repeated authority or SourceBound claims.


## Isolated source-bound store integration — 2026-10-15 00:12 UTC

This focused integration applies Rust changes from `27d6d03d`, `1b76f1e7`,
`5931cbd7` and `8a77079a`, in that order, to main `b5b7cd07`.
The bounded candidate decoder and shared generic-review partition retain one domain
implementation. Explicit lifecycle 5 creation/opening permits source-bound Establish
only, using retained snapshot evidence, source-only enrichment and the existing
projection rules. A same-store capability, revision/head fences and a SyncData batch
bind decisions, history/head, full witness, receipt and generation/row marks. Exact
replay returns its retained result. Generic identity, reserved journal/receipt,
lineage, credit/prune, migration and enrollment bypasses refuse. Accepted snapshot
reads validate the ledger; both legacy report entrypoints refuse lifecycle 5.
Incoming regions, PR50 resource/boot changes and CLI help behavior are retained.

Prior cloud **source-tree** results, not checks on this rebased tree:

- Candidate decoding/shared classification: 108 PASS, 0 FAIL, 582 skipped;
  Nextest `a6e61888-4bc1-4872-89df-e62af5b56c27`.
- Qualification/history/participation selection: 153 PASS, 0 FAIL, 558 skipped;
  Nextest `ddf56a69-5d2c-43b8-aebd-b61896f8c914`. The participation selection
  is source-tree context, not an added participation-policy change in this bundle.
- Store identity/enrichment/preservation/bounded-reader selection: 67 PASS,
  0 FAIL, 738 skipped; Nextest `acb3553c-e7b5-4633-b5bf-999d39920a19`
  (15.391s tests / 66.683s task). Earlier lineage and counter-corruption
  regressions failed before repair; the final selection passed after those repairs.
- Moon formatting and all-target/all-feature check PASS on the source tree
  (9.511s and 286.664s respectively). Source lint subsequently PASS, exit 0
  (349.219s), recorded by `1c2ed1da`: two existing nine-argument advisories in
  acquisition cache/inventory; no new lifecycle 5 warning.

All source commands used repository-root `tools/moon-local`, CI unset,
`CARGO_INCREMENTAL=0` and `MOON_RUST_BUILD_JOBS=1`. Rebased compilation, tests,
formatting, lint and full release checks are **PENDING**; patch-application and tree
roundtrip checks are not execution evidence. Review preserves domain/shell ownership
and historical identity semantics; it does not establish production acceptance.

Observation admission bounds application-level loading/decoding. Fjall/lsm-tree
inline iterator guards and size queries may already own/load values, so no global
peak-RSS or production-scale latency claim follows. Process interruption at identity
SyncData boundaries, frozen public-ID archive/readback parity, reusable production,
persisted correction operations and representative RSS remain **NOT RUN**.
Typed publication cutover and historical adoption remain open; lifecycle 5 neither
enrolls production nor turns candidate IDs into accepted public IDs. R3 remains OPEN.

## Identity interruption and authority regressions — October 15, 00:24 UTC

R3 follow-up files: `public_identity_tests/{source_isolation,projection_guards,
revision_races,enrichment_refusal}.rs` cover competing source keys in both orders,
actual legacy projection refusals, derived-only revision movement, two barrier-start
writer races, and fail-closed 8,193-row enrichment with exact replay after reopening.
`native_worker_boundary.rs`, `public_identity/commit.rs` and
`tests/public_identity_crash{.rs,/}` add feature-gated pre/post-SyncData interruption
hooks and owned SIGTERM subprocess tests. The post-commit hook follows in-memory
entity-generation publication. `format/migrate.rs` extracts the unchanged enrollment
refusal to keep the existing migration function within its size gate. Two new source
doc comments were removed from `read/view/enrichment.rs` per repository policy.

PASSED: the preceding combined store selector now runs 75 passed, 0 failed, 738
skipped; Nextest `a9e8ec9b-67a9-49cd-bd34-8b1a3eeeb23a`, 10.756s tests / 56.194s Moon.
PASSED: identity/migration after helper extraction: 60 passed, 0 failed, 753 skipped;
Nextest `c3365132-e6f5-4ca3-934f-654241cc24ca`, 9.148s tests / 53.337s Moon.
PASSED: two real SIGTERM controllers, 0 failed, one standalone child fixture skipped
by default and explicitly invoked by both controllers; Nextest
`d4838bd8-0af5-4b5a-9833-9303eb4eda4d`, 0.159s tests / 20.789s Moon.

```sh
env -u CI CARGO_INCREMENTAL=0 MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-store --lib -E 'test(public_identity_tests::) | test(format::migrate::tests::)' --no-capture --no-tests fail
env -u CI CARGO_INCREMENTAL=0 MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-store --features native-fault-injection --test public_identity_crash --no-capture --no-tests fail
```

The crash oracle compares raw retained keyspace deltas and exact replay/no-rewrite.
Review found that a positive-only sequence assertion could accept a wrong constant.
The follow-up independently captures the child precommit snapshot sequence (synced
eight-byte sidecar) or the parent recovery snapshot sequence, and compares it exactly.
Two barrier-start races are exercised schedules, not exhaustive concurrency proof.
PASSED: all-target/all-feature check (212.319s), format (7.963s) and source lint
(67.356s), three tasks completed in 288.261s. Existing forced-warning arity advisories
remain in two acquisition functions and browser rankings capture::note_request.
After the test-only exact-sequence change, both SIGTERM controllers passed again
(0 failed, one explicitly invoked child fixture skipped by default), Moon 20.124s.
PENDING: native Mac checks.
NOT RUN: power-loss durability, frozen public-ID/archive parity, production reuse,
correction operations and representative peak RSS. R3 remains OPEN.

## Selected integration checkpoint — October 15, 15:35 UTC

R1, R2 and R3 remain **OPEN**. Candidate checkpoint `f814138e4b39ae49a06272e6602ef8cf658e0070`
is based on main `450ca90765e08d963597b283a1c15fc9541d594a`; it is a local integration
checkpoint, not a pushed or runtime-qualified release. Subsequent fixture, coverage,
manual-crash and public-API repairs are still being integrated. Earlier passing results
above do not validate these later changes.

- R1 implementation track: `export/qualification/participation/` and selected capture
  bind source-backed outcomes to PR selection. The combined roster/second-meet context
  repair preserves per-performance eligibility and retained unknown/conflicting evidence.
  Acceptance still needs compiled composed regressions, real-source policy coverage,
  and actual workbook/normalized winner and exclusion verification.
- R2 implementation track: `export/qualification/current_affiliation/` and selected
  derivation use accepted dated roster evidence. Cross-school transfer is not closed:
  school-minted candidates sharing a provider subject cannot currently acquire the
  required accepted public membership through the store writer. Four transfer/conflict
  fixtures expose this domain/admission dependency; no fabricated identities qualify it.
- R3 implementation track: `public_identity/local_adjudication/`, retained/frozen replay,
  and report public-identity consumers. Manual V2/witness5 binds exact affected records;
  its native crash replacement and consumer regressions are authored, not executed.
  Public `Derivation::of` still permits historical projection from selected input at this
  checkpoint; a fallible canonical dispatch repair is in progress. Candidate membership
  extension/correction semantics and repeated-prepare staging capacity remain open.

Forward patch/whitespace checks passed for the integrated composition, consumer,
preflight, manual-authority and crash slices. Candidate Moon formatting passed.
All-target compilation, focused tests, native crash execution, source lint and release
qualification for the latest combined tree are **NOT RUN**. Mac access is connected and
authorized, but task admission still fails; cloud code/review continues.

Five physical tables now receive existing bounded admission before aggregate loading.
Other materialized tables, source-observation enrichment, decoded amplification and
lineage hashing still require bounded-memory acceptance. No representative peak-RSS
measurement exists for this checkpoint; the historical 64-GB problem is not closed.

### 2026-10-15 selected integration: school-index source comment scope

Read-only inspection of `moon.yml`'s `pipeline:xtask`, `xtask/src/main.rs`,
`xtask/src/comments.rs`, `xtask/src/comments/lexical.rs` and `tools/gate.sh`
confirms that the zero-comment lane includes both school-index source and tests.
The isolated current-main candidate removes only 25 documentation-comment lines
from `school_index.rs` and three comment lines from its tests. No executable
logic or assertions change. This is source inspection, not an executed trusted
comment-gate result.

This historical capacity repair was superseded by main `e68312bf`, which withdraws
the unused capacity API and its tests. The reconciled candidate preserves that
withdrawal; this earlier comment cleanup is retained only as provenance.
The original four compile-fix SHA256s were verified before this explicit
comment-only delta, and the two changed source hashes are recorded separately in
the integration packet. Compiler and trusted source-gate validation remain open.

## Focused identity verification — October 15, 16:58 UTC

Omarchy verified commit `8750a2160bde030f35e356fe9b49f0e56819d523`, tree
`de1b0f18bf637d52dc1336767ee84be6cefd0759`, based on main `450ca907`.
This is the focused identity repair, not the subsequent selected integration.
Moon 2.2.4, pinned nightly, task-local Cargo home, CI unset, incremental disabled,
one build job: format, workspace all-target/all-feature check and source lint PASS.
Three nonfatal forced arity warnings remain. Store selections PASS 81 and 60;
native crash controllers PASS 4 (2 child fixtures invoked by controllers, skipped
as standalone cases); CLI local-adjudication PASS 22, 202 unrelated tests skipped.

Reproduction from the repository root with the pinned toolchain and Moon on PATH:

```sh
export CARGO_INCREMENTAL=0 MOON_RUST_BUILD_JOBS=1
env -u CI tools/moon-local --concurrency 1 run pipeline:fmt
env -u CI tools/moon-local --concurrency 1 run pipeline:check
env -u CI tools/moon-local --concurrency 1 run pipeline:lint-src
env -u CI tools/moon-local --concurrency 1 run pipeline:tests-scoped -- -p census-store --lib -E 'test(public_identity::)' --no-tests fail
env -u CI tools/moon-local --concurrency 1 run pipeline:tests-scoped -- -p census-store --lib -E 'test(public_identity_tests::)' --no-tests fail
env -u CI tools/moon-local --concurrency 1 run pipeline:tests-scoped -- -p census-store --test public_identity_crash --no-tests fail
env -u CI tools/moon-local --concurrency 1 run pipeline:tests-scoped -- -p census-service --bin census-service -E 'test(cli::local_adjudication::)' --no-tests fail
```

Incoming main `e68312bf524ff2baf5986818246d2e309a8765cf` withdraws the unused
school-index capacity API and supersedes the browser boundary contract. Those user
changes are preserved in the next candidate; the passing results above do not
validate that rebase or the later membership, selected exports, participation,
transfer, paging and historical graph-budget corrections. Their runtime results
are PENDING. R1/R2/R3 remain OPEN; independent public population comparison and
representative peak-memory evidence are NOT RUN. No live census was started.

Required Rust/domain/functional/drift/test skills are available in the connected
skill repository. The black-hat response template is unavailable. Current Beads
storage is unavailable (incomplete Dolt database/read-only home configuration);
historical exports are not treated as current tracking authority.

### Remaining independent scale limits — October 15, 17:45 UTC

Current constants still bound each whole authority-table read to 8,192 rows / 16 MiB
(1 MiB per record, Store `public_identity/context.rs`). History decoding uses
`ReplayLimits::SMALL`: 4,096 records, 4,096 identities and 8,192 bindings
(Domain `public_identity_history/types.rs`, Store `public_identity/codec.rs`),
with a separate 16 MiB encoded-history cap (`public_identity/mod.rs`). The complete
frozen identity bundle remains capped at 128 MiB (`public_identity/frozen.rs`).
Manual lifetime admission independently caps charged bytes at 128 MiB, roots at 4,096
and reservations at 4,096 (`public_identity/local_adjudication/admission.rs`).
Raw physical paging does not remove these limits or make these whole-ledger and
whole-bundle consumers streaming. National-scale readiness and representative
peak-memory qualification remain unproved.

### Strict current-profile acceptance — October 15, 19:14 UTC

Current capture now binds checked qualification execution references, strict
participation v3 and selected artifact 8. Strict participation requires independent
high-school status and an explicit applicable versioned window, including summer;
a policy reference or scholastic division label does not supply that evidence.
The current test migration preserves retained source dates, identities, marks,
transfer history, frozen replay and crash recovery, but asserts withholding where
those inputs are absent. It does not establish the following positive obligations:

- OPEN: composed eligible best selection after retained-source deletion.
- OPEN: an eligible PR across workbook, normalized artifacts and sidecars.
- OPEN: transfer to the current school while retaining the old-school eligible PR.
- OPEN: a stronger ineligible mark loses to an independently eligible fallback.
- OPEN: the configured performance-window winner remains eligible across artifacts.

These five migrated cases currently prove strict withholding and replay only.
Positive acceptance still needs actual source-backed status/window projection and
execution of the positive cases. R2 additionally needs the reviewed source reader
for an explicit capture-day Current Roster statement; generic roster presence is
insufficient. That parser is not included while adversarial review remains open.
Historical v2 replay evidence remains bound to its historical inputs and cannot
qualify current v3 capture. Current CLI, full runtime and scale acceptance remain
OPEN; this checkpoint records formatting and static integration, not test passes.

### Strict checkpoint execution — October 15, 19:40 UTC

On exact `53bf6e74762ec07c441e673e5ee79e76bf593f9e`, Omarchy reported format and
full check PASS; selected tests 31/32 (one stale Eligible expectation), bridge 9/9,
historical-v2 replay 1/1 and qualification CLI 9/9. A separate external historical
artifact-7 probe passed current-consumer replay, preserved all 42 files and refused
new publication without mutation. The external probe is distinct from the one
in-repository historical replay test; neither proves current eligible-mark inputs.
The next source checkpoint corrects that remaining oracle to exact strict unresolved
reasons while preserving all three source-free views and witness equality, and adds
the independently reviewed capture-day Current Roster reader. Execution of that
new reader and combined checkpoint remains pending. No summer policy is relaxed;
the five positive eligibility obligations above remain OPEN.
