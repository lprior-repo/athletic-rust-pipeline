# M1 acceptance mapping

This maps only the adopted current-path lock-order prerequisite. All test references resolve to the exact PASS rows and formatted source pins in `TEST-COVERAGE.json`; they are not proposed tests. The enclosing actual run selected 96 tests, passed all 96, and skipped 3200. A direct selection of the isolated child is a no-op without its environment; only its passing parent establishes that child assertion path.

## 1. One real archive lock, held-admission reuse, and ordered ownership

Contract: adopted prerequisite “Minimal helper and lifetime contract” §§1–5 and lock-order table.

Evidence: `archive_lock_order_legacy_replay_and_writer_do_not_reverse_resources`; `archive_lock_order_current_v1_recovery_publishes_under_one_archive_guard`; `archive_lock_order_publisher_uses_one_guard_and_preserves_busy_precedence`; `archive_lock_order_callback_errors_release_both_actual_guards`.

Acceptance demonstrated: the first acquisition uses actual blocking File::lock on the original pathname; probes through separate descriptors observe the archive lock held. The wrapper holds both real locks before the synchronous callback. Recovery/publisher event order is exactly BeforeAcquire, Acquired, EligibleAcquired. A recursive archive attempt fails the test observer before self-blocking. Legacy actor schedules use bounded channels, release resources, and join every worker. No sleeping race oracle or mock lock substitutes for these checks.

Static bridge: review/REVIEW.md traces all four acquiring production sites, held publisher callers, archive admission and retention scopes. Existing compare callbacks call Store only; inspected Store methods acquire/release appends internally and cannot reacquire Crawl archive. Source files are pinned in SOURCE-PINS.json. No Store appends test hook was introduced, so the exact innermost Store lock edge has static callgraph evidence plus actual guarded callback execution, not independent event instrumentation.

## 2. Exact entry-specific combined-fault precedence

Contract: “Error and persisted-byte compatibility” and corrected per-entrypoint capacity timing.

- Archive lock pathname occupied by a directory + eligible busy: exact Cache/IsADirectory at archive-capacity.lock, for writer, replay and acquiring fence. Test: `archive_lock_order_lock_open_refusal_precedes_busy_eligible`.
- Malformed reservation + eligible busy, ordinary writer: exact Cache/InvalidData at the retained reservation, before eligible acquisition. Test: `archive_lock_order_admission_refusal_precedes_busy_writer_but_replay_defers`.
- Malformed reservation + eligible busy, replay: exact Cache/WouldBlock at eligible .json.lock. After eligible release, the same replay reports exact reservation/InvalidData. Same test.
- Malformed reservation + eligible busy, publisher: exact eligible WouldBlock. With eligible free, exact reservation InvalidData. Test: `archive_lock_order_publisher_uses_one_guard_and_preserves_busy_precedence`.
- Invalid capacity environment + eligible busy, legacy writer/replay: existing configuration InvalidData remains first, including empty/zero/negative/overflow/nonunicode cases exercised by existing isolated environment tests. Those strengthened tests are PASS rows in the 96-test log.
- Missing Prepared + invalid capacity: exact existing Store RequiredBackingMissing, no archive initialization or new request. The isolated parent `archive_lock_order_missing_prepared_defers_invalid_capacity_in_isolated_process` invokes its exact child with the invalid environment and checks successful nonzero child execution. Do not use the standalone child PASS as evidence for this row.
- Missing/corrupt Prepared + malformed reservation: exact Store RequiredBackingMissing/BackingIntegrity, unchanged facts/receipts/reservation. Test: `archive_lock_order_prepared_missing_or_corrupt_backing_defers_admission`.
- Busy eligible on Prepared recovery: exact Cache/WouldBlock at eligible lock before Store terminal completion; original unfinished facts remain; after release, original recovery succeeds without HTTP. Test: `archive_lock_order_busy_prepared_recovery_keeps_original_unfinished_facts`.

These outcomes do not claim every historical combined-fault precedence is unchanged. Archive-first precedence at the earlier acquiring boundaries is the explicitly adopted compatibility change; deferred full admission remains intentional elsewhere.

## 3. Legacy hit/miss and archive preservation

Contract: current legacy replay/write rows and unchanged successful bytes/meaning.

Evidence: `archive_lock_order_replay_miss_and_mismatch_defer_archive_initialization` proves None before full initialization on miss and expected-metadata mismatch, retaining an occupied reservation object. Acquiring the lock file is recorded separately. `archive_lock_order_busy_cache_preserves_incoming_archive_and_exact_error` checks exact WouldBlock/path, original eligible body+metadata and old immutable archive bytes, and preserved incoming capture.

Existing PASS coverage includes refresh-retains-both, identical replay preserving modified time/audit files, unknown metadata migration, compact metadata migration, shared read-only inode, distinct metadata bindings for identical bodies, corrupt old body/metadata refusal, quarantine interruption boundaries, capacity reservations and crash-adjacent archive replay. These are actual existing-path regressions within the 96-test run, not new broad crash qualifications.

## 4. Current v1 Prepared success and refusal semantics

Contract: recover_prepared row, successful 200-only publication and “Why lock-only must be distinct from admission.”

Evidence: `archive_lock_order_current_v1_recovery_publishes_under_one_archive_guard` drives real fixture HTTPS acquisition, stops after archive, reopens Store, and uses the actual owner/parser recovery path with and without a prior valid eligible pair. It asserts original ordinal/time/provenance, exact body and retained metadata bytes, canonical eligible encoding, old body+metadata preservation, no new request, and unchanged replay journal payloads and all receipt fields after another reopen. Existing actual304, cache reuse, interrupted owner continuation and grant-fencing tests also pass.

Missing/corrupt backing keeps exact old Store refusals and unchanged facts/receipts. Non200 recovery beside a malformed reservation produces the existing Http500 and a single terminal without eligible publication or new requests (`archive_lock_order_non-200_prepared_recovery_never_runs_archive_admission`). No restoration, loss closure, successor reservation or new retry authority is added. Raw engine receipt bytes were not read by this test; no Store serialization code changed, and all receipt fields are compared using the supported reader.

## 5. Errors, unwind and cancellation ownership

Contract: synchronous guards held until callback return/error; unchanged dedicated worker/WorkAdmission ownership.

Evidence: callback error test probes both real locks inside the callback; corrected unwind test probes both locks before deliberately unwinding, matches the exact owned payload, and verifies subsequent lock availability. Publisher encoding and missing-link errors preserve eligible bytes and release both guards. Existing archive I/O, capacity, reservation and quarantine error tests pass.

Service PASS rows include dedicated_worker_survives_caller_cancellation_and_releases_lane_after_completion; dedicated_domain_error_and_panic_have_distinct_region_accounting; published_result_does_not_release_region_owned_thread_before_actual_exit; thread_start_failure_restores_lane_and_creates_no_accepted_unowned_work; mixed_async_and_dedicated_deadline_is_counted_once_across_drain_reentry; cancelled_results_caller_keeps_original_owner_fenced_until_worker_drains; and three real results-work replay tests. They rerun the existing service ownership boundary; this patch changes none of it.

## 6. Scope, compatibility and root acceptance

Contract: no new Store/domain/service/schema/lifecycle/wire/cache envelope or second lock engine; preserve ordinary root behavior.

Evidence: SOURCE-PINS.json lists exactly 16 changed Crawl files, four new; proposal/context.patch and independent review establish the constrained cut. `archive_lock_order_existing_symlink_root_alias_keeps_legacy_behavior` verifies real write/replay through the existing supported root alias. AL01's rejected new root-type gate is absent from the corrected source. New guard/admission/capacity fields are private and uncloneable; checked zero capacity remains pre-file refusal. Four review findings AL01–AL04 are all fixed_with_evidence in the corrected patch/re-review.

## Gate boundary

The 96 focused tests and corrected independent static review support this narrow acceptance mapping. All basic gates now have terminal exit-0 receipts in GATE-STATUS.json. Exact M1 commit `0cde4e29ffbb95baa4d427b460a31dc0925dfd71` and tree `73bb89461cf332eca30849c351c982431d1bbdf4` are independently matched to all 16 formatted source snapshots in COMMIT-RECEIPT.json. Full release, native fault/restart, production, missing-backing recovery v2 and archive-scale qualification are outside the demonstrated evidence; see LIMITS.md.
