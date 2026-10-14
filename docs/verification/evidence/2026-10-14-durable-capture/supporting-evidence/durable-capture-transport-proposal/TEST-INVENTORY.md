# Authored regression inventory

Status: every test below is staged only, never executed by this worker. The TLS fixture exercises the existing reqwest execute/body/archive path on loopback. Phase-stop and pre-arm timeout hooks are deterministic fixtures, not native process-crash or actual DNS-timeout qualification.

## crates/census-crawl/src/net/acquisition/tests.rs

- actual_https_success_and_replay_without_eligible_cache_have_one_dispatch
- actual_equal_byte_same_second_dispatches_keep_distinct_occurrences
- actual_retryable_responses_keep_one_original_slot_and_three_attempt_limit

## crates/census-crawl/src/net/acquisition/tests/cache.rs

- historical_or_unsupported_cache_is_a_pre_http_gap_without_fallback
- actual_304_keeps_original_occurrence_date_and_eligible_pointer
- actual_304_cache_compare_fence_rejects_changed_metadata_without_refetch
- eligible_cache_reuse_finishes_new_work_without_new_attempt_or_date
- mismatched_eligible_metadata_cannot_publish_a_work_result

## crates/census-crawl/src/net/acquisition/tests/failures.rs

- actual_overbound_response_header_keeps_typed_status_and_url_checkpoint_refusal
- actual_malformed_429_preserves_source_cooldown_and_blocks_new_work_before_http
- actual_truncated_bodies_reopen_with_observed_failures_and_three_attempt_limit

## crates/census-crawl/src/net/acquisition/tests/grants.rs

- target_bound_linear_grant_refuses_foreign_meet_then_consumed_reuse
- lifecycle_four_legacy_context_refuses_owned_role_before_http
- adopted_foreign_cache_root_refuses_before_grant_or_http

## crates/census-crawl/src/net/acquisition/tests/recovery.rs

- prepared_archive_recovers_through_real_owned_parser_with_zero_new_requests
- prepared_without_archive_refuses_recovery_without_hidden_refetch
- observed_interruption_requires_separate_original_owner_continuation
- armed_interruption_records_no_local_execute_and_never_reuses_the_arm
- injected_prearm_timeout_reopens_reserved_without_consuming_attempt

## crates/census-crawl/src/milesplit/results/run/owned_tests/durable_capture.rs

- durable_active_meet_reuses_one_grant_for_multiple_result_sets_and_replays
- prepared_recovery_enters_acquired_meet_and_real_parser_without_http
- durable_run_request_projects_then_reopens_and_replays_the_same_capture

## Evidence authored and remaining qualification

- Success replay removes eligible cache, reopens Store, and requires unchanged occurrence/body/date with exactly one HTTP request.
- Equal-byte same-second evidence uses two actual dispatches under distinct admitted year scopes with the existing injected Clock; it does not hand-mint occurrences.
- Full Run::request reaches real parsing, raw metadata read and projection; exact three result IDs, date, projection receipt, serialized rows and journal roots are compared after Store reopen.
- Actual truncated Content-Length responses preserve ResponseObserved failures across three original-owner continuations/reopens; the fourth refuses before HTTP.
- Actual malformed429 headers retain the known source cooldown and prevent subsequent work from dispatching.
- The fixture Store produces a legitimate owner capability, but it does not exercise original Restate key/state admission or service WorkAdmission cancellation/drain.
- Compile every migrated production/test/example caller and service exhaustive errors; run existing one-shot retry, redirect, cache/archive, typed parser resource/relocation, owned receipt and projection regressions.
- Native process kill/reopen, capacity/I/O failure, Store backup/restore v2 closure, concurrency, and full84-root joined closure/backing budgets remain separate required lanes.
- Grant poisoning is mapped to a typed refusal but is not actively poisoned by a panic fixture.
- Conditional explicit-validator tests use a new legitimate scope; revalidation does not reset a completed original work.
