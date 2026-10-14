# Focused archive-lock repair acceptance

Status: static review/test planning only, before the implementation freeze. Root owns integration and all Moon execution. No build, test, network, browser, live repository write, or external publication was performed by this review.

## Scope and source contract

The adopted input is `prepared-recovery-minimal-review/M1-PREREQUISITE.md`, SHA256 `fe9b1f711f4383cdc03456dca9b4be40547ee2d6c8fb594327d46cbd483eba67`. Review covers only existing Crawl archive/eligible/Prepared recovery lock ordering and its tests. No schema/domain/Store/service change is in scope. Existing `EligibleCacheV2` is an existing eligible envelope name, not adoption of the deferred Prepared recovery v2 proposal.

## Required observable behaviors

1. The archive guard uses the existing `archive-capacity.lock` descriptor, real blocking `File::lock`, and existing error mapping. Eligible uses real `File::try_lock` with exact `WouldBlock` path/message.
2. Every nested path has archive before eligible before Store appends. Existing eligible-only compare callbacks stay eligible-only and never acquire archive. No held helper opens or acquires a second archive guard.
3. `ArchiveAdmission<'lock>` cannot outlive the owning guard. An unchecked integer cannot reach admission. Test migration must release the separate guard whenever an old test released admission before reacquisition.
4. Writer/capture/replay capacity parsing remains before lock/file creation. Replay miss/mismatch avoids archive initialization, inventory, recovery, and capture publication; the lock file is the admitted new mechanical effect.
5. Recovery does lock-only acquisition before the eligible fence and Store finishing. Capacity parsing/init/recovery occurs only on successful200 publication. Missing/corrupt Prepared retains exact Store failure and immutable facts; non200 never performs publication admission.
6. Busy ordinary cache write preserves the incoming response and existing archives, leaves eligible body AND metadata bytes unchanged, then returns exact error.
7. Callback success, typed failure, encode failure, staging/sync failure, capacity refusal, and panic unwind release both guards. Existing stage/reservation preservation rules remain exact. No new cleanup or corruption repair is introduced.
8. Current v1 recovery preserves the original occurrence/receipt/provenance/time and attempt ordinal, makes zero new HTTP requests, and publishes the right eligible body/envelope without mutating old archives.

## Error-precedence matrix

| Entry and combined inputs | Required result |
|---|---|
| Legacy writer/capture/replay + invalid capacity | Exact existing InvalidData configuration error before archive or eligible lock-file creation |
| Any newly archive-first entry + archive-lock open/lock refusal + eligible busy | Exact archive lock error, because archive lock is acquired first |
| Legacy writer + malformed archive admission + eligible busy | Exact archive admission failure (Root-approved ordering) |
| Eligible publisher + eligible busy + malformed capacity/reservation | Exact eligible WouldBlock if admission remains after the fence; do not assert archive precedence at an entry that has not begun admission |
| Legacy replay + valid config + cache miss or expected mismatch + malformed archive contents | None; no admission recovery or archive object creation |
| Legacy replay + invalid capacity + cache miss | Existing configuration error; parse is intentionally before read |
| Prepared missing/corrupt + malformed reservation/invalid config, eligible available | Exact existing RequiredBackingMissing/BackingIntegrity from Store; no terminal, receipt, new ordinal, restore, or reservation change |
| Prepared non200 + invalid config | Existing retained outcome/status semantics with no capacity parsing or publication |
| Successful200 recovery + malformed reservation/config, eligible available | Exact existing publication-time admission failure after the same Store finish/read sequence |

## Deterministic test design requirements

Use real owned temporary filesystem roots and real OS locks. Hooks may observe before/after actual acquisitions and coordinate actors; they may not replace lock acquisition, silently skip it, or prove safety using a separate fake lock. The observer should verify that a second descriptor cannot acquire archive while the callback is running, and likewise the eligible descriptor. Acquisition counts alone do not detect premature guard drop.

The important interleaving is actor A holding archive, actor B entering replay/publish, and A subsequently obtaining eligible. A before-archive signal from B must establish that B has not already acquired eligible. Release all barriers on each error path, inspect each worker result, and join every worker. Do not drop a JoinHandle to escape timeout. A mutant capable of self-deadlock or actor-cycle deadlock must be supervised by an owned child process or equivalent bounded harness that can terminate/reap only that child and report held/waited lock diagnostics. A test-only pre-acquisition rejection can detect repeated acquisition but is not itself proof that the real lock primitive ran.

Environment-dependent cases run in isolated child processes; never mutate process-global environment in parallel tests. The child must positively report the intended case ran, because a zero-test process or a no-op child selector is not evidence. Timeouts are failure evidence, never success. Sleeps are not scheduling proof.

## Existing fixtures and checks to retain

- `archive_tests/races.rs::an_in_progress_cache_writer_retains_the_response_without_retry_or_partial_replacement`: strengthen exact path/kind/message and old metadata bytes; retain old/new immutable archive assertions.
- `acquisition/tests/recovery.rs::prepared_archive_recovers_through_real_owned_parser_with_zero_new_requests`: preserve real parser/Store fixture; add prior eligible pair and no-prior-pair cases with exact publication/occurrence and lock witnesses.
- `acquisition/tests/recovery.rs::prepared_without_archive_refuses_recovery_without_hidden_refetch`: replace broad CaptureStore(_) assertion in added tests with exact Store failure, and compare journal/receipts/files.
- Existing admission explicit-zero and isolated invalid/non-Unicode environment tests preserve before-files error timing.
- All capacity/reservation tests remain on real held guard/admission, especially sequential `drop(admission)`/rebegin scopes and exact used/requested/capacity counters.
- Existing refresh/replay/shared inode/unknown metadata/quarantine/crash tests are the persisted-byte compatibility oracle; preserve their exact fixture assertions and add before/after manifests to new failure cases as needed.
- Existing dedicated-worker/WorkAdmission cancellation tests are rerun unchanged by Root; this patch must not change their ownership machinery.

## Mutations the suite must detect

- Held publication reacquires archive.
- Replay acquires eligible before archive.
- Archive guard drops before callback or publication.
- Recovery admission runs before Store finishing or for non200.
- Replay admission runs before miss/mismatch check.
- Busy write returns success, drops incoming preservation, or replaces eligible bytes.
- A previous archive is replaced or rewritten.
- A constructor silently admits zero capacity.

No broad mutation or fuzz campaign is requested: architecture explicitly retires mandatory mutation execution. These are review thought experiments and targeted regression oracles for unchanged serialization/parsing; no new parser is introduced beyond the existing capacity conversion into a checked token.

## Root-only verification commands

From the final integrated repository, first use nonzero test selection and bounded harness supervision:

```
tools/moon-local run pipeline:tests -- -E 'package(census-crawl) & test(archive_lock_order_)'
tools/moon-local run pipeline:tests -- -E 'package(census-crawl) & (test(net::cache::archive_tests::) | test(net::cache::archive::admission::tests::) | test(net::acquisition::tests::))'
tools/moon-local run pipeline:tests -- -E 'package(census-service) & (test(jurisdiction::results_work::replay_tests::) | test(spawn::dedicated::tests::) | test(work_admission::tests::))'
```

Then the required Moon format/check/source-lint/aggregate gates. This review will not call any of those commands or claim they passed. Passing focused checks cannot certify release, v2 adoption, national census, production readiness, or unrelated recovery exclusions.
