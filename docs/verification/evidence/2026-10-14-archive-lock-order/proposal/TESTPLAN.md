# Lock-order candidate verification

Status: all executable cases below are written but NOT RUN. Source checking is not a substitute for compilation or runtime verification. Root owns Moon commands and final integration.

## Instrumentation and containment

The cfg(test) observer is thread-local, installed by an RAII Observation that is deliberately not Send. Production still executes real blocking File::lock and real eligible try_lock. BeforeAcquire occurs before opening/acquiring the archive File. Acquired occurs only after successful real File::lock. EligibleAcquired occurs after successful eligible try_lock, immediately before the synchronous recovery/publication callback. Separate File descriptors perform actual try_lock probes, requiring precise WouldBlock while a resource must remain held.

The two-actor test holds the archive File in actor A, pauses actor B at BeforeAcquire using bounded channels, probes eligible availability, releases A, releases B and joins B before propagating any barrier/probe/action failure. Observer errors and barrier deadlines are test failures. No sleeps, scheduler guesses, retry-to-success oracle or detached workers are used. A second attempted archive acquisition is rejected by the test observer before it can self-block; this is explicit test-only mutant detection. The actual first File lock and its continued hold are separately probed. The harness does not claim process-kill or unrestricted schedule-exploration evidence.

## Named coverage

Cache/archive cases:

1. `archive_lock_order_legacy_replay_and_writer_do_not_reverse_resources`: real archive-holder actor versus both legacy replay and writer; before-acquire eligible availability, actual held archive lock, exact Some/body or replacement result, original archived bytes, explicit join and release.
2. `archive_lock_order_busy_cache_preserves_incoming_archive_and_exact_error`: precise eligible-lock path and WouldBlock, byte-identical old eligible/metadata/archive, both old and incoming captures retained, archive guard released.
3. `archive_lock_order_lock_open_refusal_precedes_busy_eligible`: lock pathname occupied by directory plus held eligible; exact IsADirectory/path for writer, replay and archive+eligible wrapper; occupied object unchanged.
4. `archive_lock_order_admission_refusal_precedes_busy_writer_but_replay_defers`: malformed reservation plus held eligible; writer exact record/InvalidData, replay exact eligible/WouldBlock; after eligible release replay exact record/InvalidData; all existing bytes retained.
5. `archive_lock_order_replay_miss_and_mismatch_defer_archive_initialization`: miss produces None and only a lock-file effect; mismatch beside occupied reservation path remains None and preserves occupied bytes without publishing archive captures.
6. `archive_lock_order_callback_errors_release_both_actual_guards`: callback directly probes both real locks, returns typed refusal, then both locks are available.
7. `archive_lock_order_callback_unwind_releases_both_actual_guards`: owned test unwind caught outside the fence, then both lock paths can be acquired. No production panic or catch boundary was added.
8. `archive_lock_order_existing_symlink_root_alias_keeps_legacy_behavior`: existing root aliases still write/replay through the real shared lock and preserve the alias plus target archive bytes. No new root-type admission gate is introduced.

Current acquisition cases:

9. `archive_lock_order_current_v1_recovery_publishes_under_one_archive_guard`: real HTTPS fixture interrupted after archive, reopened, then actual owner/parser recovery with and without previous valid eligible capture. Event sequence is exactly BeforeAcquire→Acquired→EligibleAcquired with actual held-lock probes; a second acquisition refuses immediately. Original ordinal/date, body/parser outcome, canonical eligible bytes, exact retained metadata bytes, prior body+metadata and occurrence binding are checked. Reopen/replay retains all journal payloads and exact decoded receipt records; no new requests.
10. `archive_lock_order_prepared_missing_or_corrupt_backing_defers_admission`: genuine interrupted Prepared plus malformed reservation; exact RequiredBackingMissing or BackingIntegrity, unchanged journal/receipts, no new requests or recovery-created bodies/captures, reservation bytes retained.
11. `archive_lock_order_non200_prepared_recovery_never_runs_archive_admission`: retained-ready500 plus malformed reservation still finishes once and returns existing Http500, creates no eligible pair and makes no new request.
12. `archive_lock_order_busy_prepared_recovery_keeps_original_unfinished_facts`: actual held eligible file gives precise WouldBlock/path before Store finishing; facts unchanged and archive guard released; release then exact recovery succeeds without request.
13. `archive_lock_order_publisher_uses_one_guard_and_preserves_busy_precedence`: actual publication records one archive acquisition and both held resources, byte-identical pointer; malformed reservation plus eligible busy returns WouldBlock, then exact reservation InvalidData after release, bytes unchanged.
14. `archive_lock_order_publisher_encoding_and_link_errors_release_guards`: malformed envelope and missing archived hard-link source produce exact typed error/location; both locks release and eligible bytes remain unchanged.
15. `archive_lock_order_missing_prepared_invalid_capacity_child`: isolated child body; genuine Prepared with no archive plus invalid configuration retains exact RequiredBackingMissing and original facts, no request/archive creation.
16. `archive_lock_order_missing_prepared_defers_invalid_capacity_in_isolated_process`: invokes exact child with invalid process-local capacity and verifies nonzero test execution and child success.

Existing `archive_capacity_environment_is_checked_in_isolated_processes` and `nonunicode_capacity_environment_is_refused_in_an_isolated_process` are strengthened: each invalid environment is also exercised with eligible held, proving configuration wins for existing writer/replay entries. Positive/zero/parser boundary tests migrate through checked capacity; zero still creates no file. All direct admission callers migrate to explicit guards and release those guards before reacquisition; archive capacity/reservation algorithms and fixtures remain unchanged.

## Required Root commands (not run)

From the final integrated repository root:

- tools/moon-local run pipeline:fmt
- tools/moon-local run pipeline:check
- tools/moon-local run pipeline:tests -- -E 'package(census-crawl) & test(archive_lock_order_)'
- tools/moon-local run pipeline:tests -- -E 'package(census-crawl) & (test(net::cache::archive_tests::) | test(net::cache::archive::admission::tests::) | test(net::acquisition::tests::))'
- tools/moon-local run pipeline:tests -- -E 'package(census-service) & (test(jurisdiction::results_work::replay_tests::) | test(spawn::dedicated::tests::) | test(work_admission::tests::))'
- tools/moon-local run pipeline:lint-src

Intentional source formatting, if needed, must use Root-owned `env -u CI tools/moon-local run pipeline:fmt-write`, followed by renewed pins and verification. Check exact named test selection and counts; the child test's direct no-environment pass alone is not its evidence. Run remaining required aggregate/release lanes per owning contracts before any broader acceptance claim. Current service cancellation/drain regression reruns are required without changing service machinery. No native restart/production/census lane was requested or executed by this worker.

## Limits

This candidate does not add a Store appends observer: Store append ordering follows the real synchronous fence callback and existing Store APIs, and must also be checked in independent callgraph review. Exact raw on-disk receipt encoding is unchanged by construction (no Store edit); tests compare all receipt fields through the existing reader, not private physical engine bytes. Tests exercise ordinary encode, link, callback, unwind, configuration, reservation, busy and backing errors; existing archive publication/quarantine/reservation tests cover the remaining supported storage failure paths. No new broad crash/fault or performance campaign is claimed.
