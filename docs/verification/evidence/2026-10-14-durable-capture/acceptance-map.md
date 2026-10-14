# Capture acceptance mapping

STATUS: UNVERIFIED

## CAP-01 — Separate stable slot/work/attempt/dispatch/content/occurrence identities; equal bytes do not collapse physical attempts.

Authority: spec-v2 §6, ARCHITECTURE §§5,9

Disposition: historical_partial_pass_current_rerun_required

Evidence: core-second.log; composed-second.log: stable_slot_excludes_request_payload_but_work_binds_exact_representation; actual_equal_byte_same_second_dispatches_keep_distinct_occurrences

Remaining boundary: Complete spec content/observation/interpretation/revision identities across all sources and A→B→A accepted revisions are not qualified by this slice.

## CAP-02 — One original retry owner; no hidden transport retry; at most three lifetime attempts per slot.

Authority: spec-v2 §§6,9,10, ADR-002

Disposition: historical_partial_pass_current_rerun_required

Evidence: composed-second.log: actual_retryable_responses_keep_one_original_slot_and_three_attempt_limit; actual_truncated_bodies_reopen_with_observed_failures_and_three_attempt_limit

Remaining boundary: Service unknown-outcome test failed historically; corrected Interrupted/RetainedOutcomeUnknown oracle and separate known 500/BodyIncomplete exhaustion regression remain unexecuted at this checkpoint. Unknown is not known exhaustion.

## CAP-03 — Durable reservation and arm before dispatch; no reconstructed arm after ambiguity.

Authority: spec-v2 §§6,9

Disposition: historical_partial_pass_current_rerun_required

Evidence: core-second.log; composed-second.log: reservation_reuses_ordinal_but_armed_state_never_reconstructs_dispatch; armed_interruption_records_no_local_execute_and_never_reuses_the_arm

Remaining boundary: Native process/power-loss dispatch boundary, coordinator endpoint separation, and actual mid-I/O cancellation remain unqualified.

## CAP-04 — Truthful actual response checkpoints, failure taxonomy, no success from malformed/blocked/partial bodies.

Authority: spec-v2 §§6,9,10

Disposition: historical_partial_pass_current_rerun_required

Evidence: composed-second.log: actual_overbound_response_header_keeps_typed_status_and_url_checkpoint_refusal; actual_malformed_429_preserves_source_cooldown_and_blocks_new_work_before_http; coach capture failures

Remaining boundary: Response-refusal holds actual status/URL and a closed reason, not a fabricated complete header checkpoint. Full adapter and negative-evidence scope remains open.

## CAP-05 — Cache reuse and 304 link exact immutable original bytes/metadata without restamping body freshness.

Authority: spec-v2 §§6,10

Disposition: historical_partial_pass_current_rerun_required

Evidence: composed-second.log: actual_304_keeps_original_occurrence_date_and_eligible_pointer; actual_304_cache_compare_fence_rejects_changed_metadata_without_refetch; cache_replay_and_304_keep_exact_original_body_and_time_without_reference_chains

Remaining boundary: Post-lint boxing changes need rerun. Fixture 304 proof is narrower than full deployed refresh qualification.

## CAP-06 — Retain bytes before committed metadata/receipt; replay acknowledged or Prepared work without duplicate acquisition.

Authority: spec-v2 §9

Disposition: historical_partial_pass_current_rerun_required

Evidence: composed-second.log: prepared_archive_recovers_through_real_owned_parser_with_zero_new_requests; prepared_archive_reopens_and_finishes_original_attempt_without_dispatch; durable_run_request_projects_then_reopens_and_replays_the_same_capture

Remaining boundary: Prepared loss/corruption of backing is fail-closed and remains a recovery blocker; no automatic successor attempt or recoverability guarantee.

## CAP-07 — Preserve receipt and journal authority; conflict checks and semantic effect durability.

Authority: spec-v2 §§9,12

Disposition: historical_partial_pass_current_rerun_required

Evidence: composed-second.log: generic_journal_receipt_and_credit_paths_cannot_overwrite_capture_roots; slot_registration_replays_exact_bytes_and_refuses_changed_request_budget_reset

Remaining boundary: No universal exactly-once claim beyond inspected local boundary; receipt pruning/restore horizons and full projection invalidation/outbox remain separate obligations.

## CAP-08 — Shared bounded root/encoded/backing reads and complete canonical-envelope admission.

Authority: spec-v2 §§12,16

Disposition: historical_partial_pass_current_rerun_required

Evidence: composed-second.log: full_three_attempt_five_dispatch_closure_fits_exactly_forty_two_roots; maximal_original_and_revalidation_join_uses_eighty_three_roots_with_default_eighty_four; maximum_canonical_envelope_and_physical_key_are_both_debited_before_materialization; oversized_canonical_envelope_refuses_before_any_journal_mutation

Remaining boundary: 42-root single closure and legal 83-root original/current join under default 84 are not total-I/O bounds. Clock-race test correction requires rerun. 2 GiB inventory ceiling is unresolved against larger legal writer output.

## CAP-09 — Historical lifecycle preservation and explicit fresh qualification only.

Authority: spec-v2 §§12,14,17,18, ADR-038

Disposition: historical_partial_pass_current_rerun_required

Evidence: composed-second.log: capture_init_never_adopts_legacy_existing_or_artifact_roots; capture_control_preserves_uppercase_existing_identity_and_never_remints_missing_identity; historical_non_capture_backup_keeps_raw_schema_preservation_policy

Remaining boundary: Lifecycle 1/2 meaning and IDs retained; lifecycle 3 enrollment-only retained; lifecycle 4 is fresh capture qualification only. No legacy adoption, annual production upgrade or migration certified. Lifecycle-5 proposal is excluded.

## CAP-10 — One Fjall owner; ownership-safe stopped backup/relocation/restore and invalid-restore refusal.

Authority: spec-v2 §§2,9,12

Disposition: historical_partial_pass_current_rerun_required

Evidence: composed-second.log: capture_init_has_one_real_fjall_owner_and_missing_open_never_creates; stopped_relocation_backup_restore_preserves_identity_attempts_receipts_and_cache_v2; a_hash_consistent_invalid_restore_cannot_promote_over_an_empty_destination

Remaining boundary: Stopped fixture backup is not cross-system native Restate/Fjall barrier, production backup scale or measured RPO/RTO; all referenced live/archived byte availability remains required.

## CAP-11 — Bind service original WorkflowIdentity, retained manifest/history/source plan and canonical target before grant.

Authority: spec-v2 §§2,9,10,16

Disposition: historical_partial_pass_current_rerun_required

Evidence: composed-second.log: capture_owner_requires_the_actual_retained_manifest_key_and_window; capture_manifest_rejects_foreign_lineage_cohort_scope_and_run; grant_target_matches_all_discovered_result_sets; same_target_can_receive_only_one_grant_per_continuation

Remaining boundary: MileSplit whole-meet lane is scoped qualification; source-plan membership is not external-source qualification or all-provider adoption.

## CAP-12 — Original owner guard survives caller cancellation; region owns dedicated threads and exact drain accounting.

Authority: spec-v2 §§14,16, ARCHITECTURE §10, ADR-037

Disposition: historical_partial_pass_current_rerun_required

Evidence: composed-second.log: cancelled_results_caller_keeps_original_owner_fenced_until_worker_drains; spawn::dedicated::*; spawn::descriptor_tests::*

Remaining boundary: Controlled worker fixtures do not establish cancellation during real dispatch/archive. Dedicated effect may overrun grace and must remain owned until it actually exits; performance/stack/resource envelope is unmeasured.

## CAP-13 — No nested blocking-pool deadlock in actual Results selector plus DNS under one blocking thread.

Authority: spec-v2 §§9,16

Disposition: not_run

Evidence: No executed evidence for this obligation in this batch.

Remaining boundary: This test exists but was not selected by the historical composed --lib filter. It needs its own nonzero exact test-name result.

## CAP-14 — Actual SDK completed/error Results Run replay drops unused owner admission and preserves state/effects.

Authority: spec-v2 §§9,10,16

Disposition: authored_reviewed_not_run

Evidence: capture-service-review/replay-bridge/REVIEW.md

Remaining boundary: Three SDK protocol tests are integrated and formatted, but not executed at this checkpoint. Offline refusal case is not successful live-body acquisition or native journal restart.

## CAP-15 — Explicit inspectable CLI init/show and serve opt-in with useful failures.

Authority: spec-v2 §14

Disposition: partial_not_run

Evidence: composed-second.log: bootstrap::serve::capture_tests::*

Remaining boundary: Serve library tests passed historically. Two CLI unit tests belong to --bin census-service, not --lib; three integration CLI tests need --test capture_protocol_cli. Neither target was executed in composed-second.

## CAP-16 — Final composed compiler, all-target, lint, format, trusted source/AST/size and safety gates.

Authority: spec-v2 §16, ARCHITECTURE §§10,11

Disposition: blocked

Evidence: core-first.log; transport-first.log; composed-first.log; basic-first.log; all-target-first.log; basic-second.log; format-first.log; format-second.log; format-third.log; all-target-serial.log

Remaining boundary: Red diagnostics are retained; source repair does not imply pass. Serial check passed before subsequent boxing repairs; final reruns remain pending unless later raw terminal output says otherwise. Full gate and exact-build binary identity remain unestablished.

## CAP-17 — Exact-build traceability and current-byte independent review.

Authority: spec-v2 §§16,18

Disposition: blocked

Evidence: raw-log-ledger.json; verification-ledger.jsonl

Remaining boundary: Source ledger is current snapshot only; historical run snapshots were not reconstructed. Current-byte gaps, active compile, missing binary/corpus/toolchain-at-run receipts and active-context truth-serum prevent blanket acceptance.

## CAP-19 — Keep new capture error/context variants bounded while preserving exact typed errors and linear grant authority.

Authority: spec-v2 §16, ARCHITECTURE §10

Disposition: source_repaired_pending_rerun_and_review

Evidence: lint-format-final.log; transport-error-size-repair/README.md; capture-service-context-box-delta/HANDOFF.md

Remaining boundary: Seven-file FetchError::CaptureStore boxing delta and two-file private AdmittedCapture boxing delta applied; final fmt-write passed, but exact delta review and final lint/check/test outcomes are pending. No broad error boxing, warning suppression or 85 caller rewrites are claimed.

## CAP-18 — Native crash/I/O faults, quota reservations, writer-reader/GC races and backup restore across systems.

Authority: spec-v2 §§12,16

Disposition: not_run_in_this_batch

Evidence: No executed evidence for this obligation in this batch.

Remaining boundary: All 17 native fault scenarios, ENOSPC/short write/fsync failure and actual interrupted dispatch/archive/ack/frontier/publication boundaries need exact witnesses; loopback fixtures and modeled faults do not substitute.

## FULL-01 — Five-state Class-of-2027 product with UP-first scope, accurate cohort/event/contact/affiliation and publication filtering.

Authority: spec-v2 §§1,3,4,7,8,13,18,19

Disposition: outside_this_slice_not_accepted

Evidence: No executed evidence for this obligation in this batch.

Remaining boundary: No five-state/UP live census, complete source qualification, finalized school/program denominator or recruiting publication is delivered by durable-capture integration.

## FULL-02 — Reversible identities, accepted current affiliation, corrected candidate fallback and all affected projection semantics.

Authority: spec-v2 §§5,7,8,11

Disposition: outside_this_slice_not_accepted

Evidence: No executed evidence for this obligation in this batch.

Remaining boundary: Do not turn a detached accepted-affiliation design or prior best-mark work into this batch’s acceptance evidence.

## FULL-03 — Complete interpretation dependency identity, offline reparse, due/negative evidence indexing and annual retained-store rollover.

Authority: spec-v2 §§6,10,11,14

Disposition: outside_this_slice_not_accepted

Evidence: No executed evidence for this obligation in this batch.

Remaining boundary: This fresh-qualification capture slice does not prove reusable lifecycle-3 execution, annual 2027→2028 operation or all-source A/B/A semantics.

## FULL-04 — Measured whole-workload capacity, memory/disk ceilings, retention, GC, RPO/RTO and growth/skew correctness.

Authority: spec-v2 §§12,16

Disposition: blocked

Evidence: No executed evidence for this obligation in this batch.

Remaining boundary: No performance/throughput/scale claim; default archive 100 MiB is distinct from 2 GiB inventory ceiling and independent backup scans. Deployment resource limits remain to be qualified.

## FULL-05 — One immutable workbook/normalized generation and any separately selected query database.

Authority: spec-v2 §§13,15

Disposition: outside_this_slice_not_accepted

Evidence: No executed evidence for this obligation in this batch.

Remaining boundary: No new publication or query-database engine decision, migration, import, or readiness claim is made.

## FULL-06 — Complete applicable release/fuzz/security/dependency qualification.

Authority: spec-v2 §§16,18, ARCHITECTURE §11

Disposition: not_run_in_this_batch

Evidence: No executed evidence for this obligation in this batch.

Remaining boundary: Full release wrapper, separate native fault and parser-fuzz campaigns, security/dependency and resource/performance evidence are not replaced by focused tests. Retired mutation and excluded license/cargo-vet requirements are not reintroduced.
