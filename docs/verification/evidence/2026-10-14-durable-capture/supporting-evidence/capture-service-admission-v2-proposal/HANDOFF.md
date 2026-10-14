# Capture service admission v2 proposal

## TASK

Implement the adopted fresh-qualification capture admission in the existing CLI, serve bootstrap and Jurisdiction Results path. This is a detached source proposal, not integrated implementation or execution qualification.

## OWNERSHIP

Only `capture-service-admission-v2-proposal/` was written for this revision; the v1 proposal is frozen. Root remains the sole live writer/integrator. The contextual patch covers 69 service files. Most leaf changes add an explicit legacy `AdapterContext.capture` or migrate a constructor; service `jobs.rs` preserves the current typed-parser exhaustive arms.

## DO NOT MODIFY

Domain/Store APIs and schema remain the core worker's ownership. Crawl/net/actual whole-meet implementation remains the transport worker's ownership. No legacy request fields, Restate service names, workflow keys, retry annotations, run manifests, profile/enrollment semantics, receipts, source keys or history bytes were changed by this proposal.

## INPUT CONTRACT

Core API is `capture-protocol-core-proposal/API.md`, including the subsequent target-bound `continuation_for_owned_meet` and `ResultsCaptureOwnerV1::slot_for_owned_meet` additions.

Required crawl exports are:

- `CaptureContext<'a>::LegacyMetadata | DurableV1(&'a ResultsCaptureContext)`
- Required `AdapterContext.capture: CaptureContext<'a>`
- `ResultsCaptureContext::new(owner: ResultsCaptureOwnerV1, grant: RetryOwnerGrant, site: UsJurisdiction, meet_id: &str) -> CrawlResult<Self>`
- `CrawlError::Capture(CaptureTransportError)` is a typed nonretryable refusal at the service mapping boundary.

The transport worker agreed these exact names and owns the corresponding implementation, including the private Available/Consumed grant state and lifecycle-4 legacy-role refusal before structured HTTP.

## OUTPUT CONTRACT

- `census-service --store <root> capture-protocol init` and `show` dispatch synchronously before the async runtime. They require explicit roots and call only the dedicated Store APIs. No new JSON flag is introduced; output is the verified declaration.
- `ServeCaptureProtocol::{LegacyFresh, FreshQualificationV1}` is explicit in `ServeOptions`. The only accepted flag value is `fresh-qualification-v1`. Default serving uses ordinary Store open. Selected serving uses existing-root capture open before listener or browser initialization, and passes the actual writer through endpoint composition.
- `CaptureExecution` reaches `JurisdictionCensus`. Lifecycle 4 refuses missing/foreign run manifest, cohort, jurisdiction or run before running stages; legacy missing-manifest behavior is unchanged.
- Results admission compares actual object key and loaded state identity with the existing `WorkflowIdentity`, checks the already bound history window/year, loads the retained manifest, and binds its exact digest. The current actual `history_sources(state)` still derives its selection directly from `plan.sweepable`; no source qualification is manufactured. A selected MileSplit role requires plan membership and the actual MilesplitResults dispatch arm. A selection without MileSplit gets `UnadoptedSources` without minting that role.
- The owner flows through history -> results_work -> jobs re-export -> results_arms -> collection::one -> actual milesplit_results -> crawl collect_result_sets. All discovered result references must match the selected canonical site/meet before a target-bound grant is issued.
- A bounded per-stage `HashSet<AcquisitionSlotId>` permits one grant per canonical slot per continuation. A repeat is an explicit unfinished refusal; it cannot allocate another physical attempt inside the same continuation.
- Existing TeamsSource WorkAdmission/gate ownership is extracted into the shared Jobs object. Lifecycle-4 results move the original `ctx.key()` WorkAdmission into the existing Spawner's bounded dedicated OS worker across the entire stage. Handler cancellation cannot release the worker's guard. The original HandlerError is preserved through worker outcome handling; panic/cancel are explicit terminal outcomes. Nested stage collection does not acquire another work permit.
- Existing HistoryWindow and HistoryError are re-exported from census-domain using Root's exact staged move. HistoricalProgress stays service-owned.

## REVISION: SUPERVISION AND FILE SIZES

V1's outer Tokio blocking worker could deadlock with a single Tokio blocking thread: the Results selector uses `spawn_blocking`, and real HTTP hostname resolution uses `lookup_host` on the same pool. V2 uses one fallibly started dedicated OS thread per admitted Results stage, retaining the existing long-lived Tokio Handle and shared Fetcher. There is no stage runtime, alternate client, independent retry engine or extra gate registry.

The existing Region now owns both its JoinSet and each standard-thread JoinHandle directly. Async descriptors A are checked as JoinSet length minus the existing ledger's tracked blocking count. Worker descriptors B are checked as that blocking count plus the dedicated-thread count. A and B each have capacity N, and their checked combined bound is 2N. A worker's lane permit is independent of async descriptor admission, preserving the existing capacity-one async parent/blocking child contract. Ordinary spawn retains its existing lane charge. All handles, including completion-publication tails, remain counted until joined.

A newly acquired worker lane permit can race with publication of a retiring worker's join completion. Blocking/dedicated acceptance therefore waits only for B room, reaping under the Region lock and sleeping without it; async parents cannot fill this worker-only dimension. Saturation remains queued, never mapped to cancellation. Cancelling this wait drops an unstarted job and its permit without any accepted ledger entry. Region closure wakes queued admission and returns cancellation. Async spawn_with checks both Coordinator and descriptor admission before evaluating its build closure.

Dedicated metadata allocation and thread start have typed failures before acceptance. Span and subscriber dispatch are propagated into the dedicated thread. There is no abortable completion monitor or second admission registry.

The actual thread owns the source-work guard until job completion/unwind. Its result oneshot only communicates caller interest. There is no abortable monitor holding the JoinHandle. Drain polls unfinished standard threads every 10 ms and joins only `is_finished` handles; it never waits for a pending thread in the Tokio blocking pool. Cancellation of drain restores the entire Region. The grace deadline aborts async tasks, records the combined remaining count once, and keeps draining non-abortable effects. Completion/domain error/panic accounting uses the existing ledger. A finished-before-drain-reentry edge explicitly clears stale remaining/still_running counts.

Production modules were cohesively extracted to leave formatting margin:

- `bootstrap/store.rs` owns Store/open-mode composition; `bootstrap/serve.rs` is 255 lines.
- `cli/run.rs` owns synchronous/offline routing and runtime command dispatch; `cli/mod.rs` is 236 lines.
- `publish/runtime.rs` owns shared Jobs/admission composition; `publish.rs` is 256 lines.
- `jurisdiction/results_work.rs` owns the retained Results work boundary.
- `spawn/{types,region,dedicated}.rs` own typed outcomes, combined Region accounting and dedicated-thread dispatch; `spawn.rs` is 231 lines.
- Existing HistoryWindow inline tests move to `pipeline/history/tests.rs` without moving HistoricalProgress out of service.

These are unformatted source counts, not evidence from the repository formatting/size gate.

## ACCEPTANCE AND EVIDENCE

Executed: source inspection, contextual diff generation, SHA-256 base comparison (all original files match current live), and `git diff --no-index --check base source` (no whitespace findings; exit 1 indicates file differences).

Not executed: builds, tests, formatter, lint, native Restate, HTTP requests, production or qualification services. No network, live source edit or commit was performed. The no-build/no-test ownership fence remains binding. Formatting and compiler correctness must be verified after Root integrates the core and crawl prerequisites.

New tests cover the real CLI parser/binaries and public serve function, original owner/key/window/manifest/source-plan admission, selected-target equality, per-continuation duplicate-grant refusal, source-work cancellation/drain ownership, and service-admission-to-Store lifetime attempt cap across repeated original-owner continuations. Historical direct-call tests and every service AdapterContext literal were migrated explicitly.

Additional v2 proposed regressions:

- `spawn/dedicated/tests.rs`: publication before actual exit; caller cancellation; cancelled drain and finished-before-reentry; one-time combined deadline accounting; distinct domain error/panic outcomes; deterministic thread-start failure with returned permits and no accepted job; submitting span/dispatcher propagation.
- `spawn/descriptor_tests.rs`: actual capacity-one async parents waiting for blocking and dedicated children, with simultaneous A=1/B=1 assertions; async descriptor refusal leaves the builder unevaluated; controlled permit-release-before-worker-join tail queues both worker modes; cancellation of either unstarted descriptor wait restores the lane without accepting an effect.
- `work_admission/tests.rs::cancelled_results_caller_keeps_original_owner_fenced_until_worker_drains` now uses the actual dedicated supervisor and retains the original keyed WorkAdmission.
- `results_arms/tests/dedicated_worker.rs::dedicated_results_selector_and_tokio_dns_finish_with_one_blocking_thread` uses worker_threads=1/max_blocking_threads=1, the actual Results selector, and the real shared Fetcher/GuardedResolver against a loopback localhost HTTP fixture within the same dedicated action. It asserts exact body, completed count and zero remaining work. It does not claim adopted MileSplit request/replay coverage.

These tests are proposals, not passing evidence. The cancellation test exercises the shared actual WorkAdmission and supervisor with a controlled worker; it does not execute a MileSplit HTTP capture. The cap tests exercise the actual service admission and Store protocol without transport, using a synthetic request representation. They are not native Restate run/pass/restart or physical-request-count evidence. Actual production-caller replay/prepared recovery, interrupted finalization, network counts and native restart still require integrated qualification.

Suggested Root Moon checks after all three source slices are integrated:

- `tools/moon-local run pipeline:check`
- `tools/moon-local run pipeline:tests -- -E 'binary(capture_protocol_cli)'`
- `tools/moon-local run pipeline:tests -- -E 'test(capture_) or test(original_owner_pass_continuations) or test(same_original_owner_cannot_reset) or test(grant_target) or test(same_target_can_receive) or test(cancelled_results_caller)'`
- Existing TeamsSource cancellation/replay tests and the prior typed-parser/result-status suites
- Format/lint and the full required release/fault/resource qualification through Moon

## HANDOFF

- `service-admission.patch`: full 3297-line contextual integration patch with original relative paths
- `v1-to-v2.patch`: 1818-line revision delta from frozen v1, including newly touched live-base files
- `base/`: exact original file bytes
- `source/`: proposed complete files
- `base-pins.sha256`: input source hashes
- `source-pins.sha256`: proposed changed-file hashes
- `changed-files.txt`: 69-file integration inventory

Frozen source-pins.sha256 digest: `8adcda448977520fd76fe661f67f104bc277b6e353f4ae105b4770cf056ac7f8`. V1 source-pins.sha256 remains `fa1b7a3fa0328ee712419229663905c6da1db1cdac453e908f9764dc70475a8b`.

The focused v2 reviewer verified this exact source-pins digest, all 69 source files and all 45 base files. No remaining confirmed static source blocker was found in the focused delta; the v1 nesting/file-size and v2 capacity/handoff findings are structurally addressed. The separate review record is under `capture-service-review/v2-round2`. This is static review, not execution acceptance. Compiler/lint, real loopback execution, native replay/restart and source-capture qualification remain Root integration obligations. The dedicated worker can exceed the drain grace deadline while completing an already owned effect; it is intentionally retained and reaped, never abandoned.

Integrate alongside core and transport APIs. Do not describe this as source qualification or lifecycle-4 runtime acceptance until the actual consumer and required evidence have passed.
