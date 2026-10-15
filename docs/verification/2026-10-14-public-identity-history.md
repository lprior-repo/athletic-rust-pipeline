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
