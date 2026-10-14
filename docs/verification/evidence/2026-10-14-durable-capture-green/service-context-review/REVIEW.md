# Private service capture-context boxing: exact delta review

Result: no blocking source defect found in the two-file boxing delta. This is a narrow static delta review, with separately supplied Root test execution; it is not an independent full-service audit, benchmark, or release approval.

Reviewed implementation commit: `3708d6bab0b08c390887c562a89e8c6714255a5d`. The two final hashes are in `reviewed-current-pins.sha256`. The base is the exact pair previously reviewed in `capture-service-review/current-byte-reconciliation/REVIEW.md` and pinned there: `853f0081…` for `capture_scope.rs`, `dad25867…` for its tests. The detached source delta is preserved in `base/`, `current/` and the two `.diff` files. Proposal-to-current production bytes are identical; the added test differs only by rustfmt line breaks and a trailing comma.

## Contract and ownership findings

- `capture_scope.rs:9–20` keeps the private exhaustive `LegacyMetadata`/`DurableV1` sum type. Only the `DurableV1` payload changes to `Box<ResultsCaptureContext>`. `context()` borrows its concrete payload with `as_ref()` and returns the unchanged public borrowed `CaptureContext::DurableV1`. No new public serialized format, optional fallback, downcast, trait-object erasure or compatibility path is introduced.
- `capture_scope.rs:23–63` preserves the same target validation, stable slot derivation, grant-claim order, `continuation_for_owned_meet` call and checked constructor. It moves the already-constructed context into one box after these checks. Existing `owner.clone()` remains the pre-existing copyable owner capability; neither the private wrapper, `ResultsCaptureContext`, nor `RetryOwnerGrant` acquires `Clone`.
- `context/capture.rs:14–24,58–67` retains the private mutex-protected `Available(RetryOwnerGrant)`/`Consumed` state and the destructive `mem::replace` extraction. Boxing does not mint a grant, reset its state or introduce another consumer. This delta adds no lock, await, spawn, thread, retry, network request, persistent write or cancellation branch.
- The sole production caller, `results_arms.rs:182–195`, keeps the admitted wrapper alive while the borrowed adapter context is used by `collect_result_sets().await`. Ownership and drop scope are unchanged. No mutex guard is introduced across that await.
- `capture_scope/tests.rs:59–67` checks the private representation is no larger than two machine words and checks the explicit legacy projection. It does not assert a measured exact byte size or exercise durable grant consumption by itself. Separate exact-named composed tests cover foreign-target/consumed-grant refusal, one-grant multi-result-set collection, service target validation and Results replay.

## Resource and Power-of-Ten boundary

The change adds one `Box<ResultsCaptureContext>` allocation for each successful durable admission, not only on failures. Legacy admission allocates no box from this change. The allocation uses ordinary Box allocator/OOM behavior; this review does not claim graceful allocation failure, no post-initialization allocation, bounded whole-workload memory, reduced peak stack, lower latency, higher throughput, or an optimal storage choice. The passing layout guard establishes only its asserted representation bound on the executed target. Allocation rate, RSS, stack and workload performance remain unmeasured.

For this delta: rules 1/2 retain simple flow and add no loop/recursion; rule 3's strict post-init-allocation ideal is not satisfied and is not waived by this report; rules 4–9 retain short explicit code, typed outcomes, narrow borrows and checked returns, with no unsafe, extraction/panic macro, unchecked arithmetic, ignored fallible result, new code comment or lint suppression. Rule 10's final compile/lint/fmt results belong to the separate Root execution ledger. These are change-scoped observations, not whole-repository certifications.

## Executable evidence kept separate

`raw-logs/composed-third.log` records PASS for `restate_services::results_arms::capture_scope::tests::admitted_capture_storage_stays_within_two_machine_words` and the unchanged target test. Its other selected current tests include `target_bound_linear_grant_refuses_foreign_meet_then_consumed_reuse`, `durable_active_meet_reuses_one_grant_for_multiple_result_sets_and_replays`, and all three actual SDK Results replay tests. Full names, line pointers, declaration paths and source hashes are in `selected-test-results.json`. A fixture pass does not establish native Restate restart, cancellation during real dispatch/archive, whole-source adoption or production recovery.

## Review operations and references

Performed read-only inspection of both final files, the exact base/current/proposal diffs, the sole production call site, `ResultsCaptureContext`, `RetryOwnerGrant`, the current-byte prior review and its pins, and the existing raw selected-test output. Hash comparison verified the two final files against the commit and original frozen source manifest. No build, test, formatter, service, browser, source write or publication was performed by this reviewer.

Read repository `AGENTS.md`, `ARCHITECTURE.md`, and `xtask/README.md`; recovery `rust-contract/SKILL.md`, `holzman-rust/SKILL.md`, `async-rust-reviewer/SKILL.md`, `scott-ddd-refactor/SKILL.md`, `evidence-packaging/SKILL.md`; Holzman `references/nasa-jpl-standards.md`, `references/latency-throughput-playbook.md`, `references/zero-cost-abstractions.md`; async `references/send-sync-ownership.md`. The applicable review is a two-file storage/ownership delta; no new scheduler or async protocol is being introduced or re-audited.
