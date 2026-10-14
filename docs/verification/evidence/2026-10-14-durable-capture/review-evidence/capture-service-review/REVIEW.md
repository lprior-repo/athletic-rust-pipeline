# Frozen capture-service admission review

VERDICT: OPEN 2

This is a static review of the exact original 53-file proposal, not a build or runtime qualification. Root is the sole live writer and build runner. No live source, service, census, security, network or durable store state was changed. The later v2 repair is outside these frozen pins and requires its own review and Root-run evidence.

## Exact subject

- Proposal: `../capture-service-admission-proposal/`
- The exact 53 paths are in `reviewed-files.txt`.
- Their SHA-256 values are in `reviewed-source-pins.sha256` and match all 53 proposed files.
- All 40 original base-file hashes also match. Thirteen files are additions.
- Patch SHA-256: `0b1e1130c7b3f86bcd02819b4bc0372c649fb5532a21899779a20a2985935c33`.
- `proposal-artifact-pins.sha256` pins the proposal handoff, patch and manifests.
- `context/repo/` preserves the relevant unchanged repository ownership/supervision context; `context-pins.sha256` pins those copies.

## Open findings

### CS-01 — High: the Results worker waits for children needing its own bounded Tokio blocking pool

Frozen `crates/census-service/src/restate_services/jurisdiction/stage_runs.rs:61–64` holds a Tokio blocking-pool thread while `Handle::block_on` waits for `jobs::results_stage`. The real stage calls `selection::select` at `results_arms.rs:112`; unchanged `results_arms/selection.rs:23–34` acquires its global one-worker selector semaphore and awaits another `tokio::task::spawn_blocking` job. With a supported caller-owned runtime configured with `max_blocking_threads(1)`, the outer worker occupies the sole thread and its selector never starts. The supervisor cannot abort an already started blocking worker, so drain also remains stuck.

The frozen cancellation test at `work_admission/tests.rs:10–48` uses a controlled closure; it does not invoke the actual Results action and cannot detect this wait graph. This finding is static: no reproducing test was executed under the no-build/no-test fence.

Moving selection before the outer worker removes only this explicit dependency. The current transport proposal's `net/destination_guard.rs:60` and `GuardedResolver::resolve` use `tokio::net::lookup_host`, whose DNS work also depends on Tokio's blocking pool. The transport owner confirmed there are no explicit crawl-side `spawn_blocking` calls, and that synchronous Store/archive phases currently depend on the service's outer blocking execution. Therefore a superficial preselection-only repair is insufficient for pool-independent operation.

Do not claim that CLI `--max-concurrent=1024` by itself saturates a default 512-thread pool: bootstrap still constructs `Spawner::new()` with a 500-permit lane, and each admitted Results stage charges 100,000 of a 1,000,000 Work budget. There can be at most ten such stage reservations, and normally fewer while server work is charged. The one-thread public-runtime case is the precise counterexample; default-pool saturation needs a separate supported-capacity argument.

Required repair: remove the dependency on occupying Tokio's blocking pool while waiting for work that also needs it, retaining the complete original-owner fence, region ownership and drain certificate. Root and the service writer have adopted a bounded dedicated-thread mode in the existing Spawner; the ownership contract is below.

### CS-02 — Medium: two production files newly exceed the hard file-size cap

Frozen `bootstrap/serve.rs` grows from 295 to 314 lines. Frozen `cli/mod.rs` grows from 298 to 305 lines. ARCHITECTURE §10 has a hard 300-line production-file bound. Both exceed it before formatting the compact additions.

Required repair: extract coherent store-opening and CLI dispatch responsibilities, then have Root run formatting and trusted size checks. The service writer reports these extractions in a detached v2 proposal, but those bytes are not part of this frozen review and no resolution is claimed here.

## Sound repair ownership contract

The smallest repair that preserves the current synchronous Store/archive transport shape is a dedicated OS thread for an admitted whole Results job, integrated into the existing Spawner:

1. Acquire the existing bounded blocking-lane permit before spawning. Do not charge another Jobs/Work reservation. The original `WorkAdmission` moves into the actual thread and lives until that thread exits.
2. Use the existing long-lived Tokio `Handle::block_on` on that dedicated thread. Selection and DNS can then use the shared Tokio blocking pool without a parent blocking-pool holder.
3. Retain each fallibly created `std::thread::JoinHandle` and its outcome counters directly in `Region`. An abortable Tokio monitor must never be the sole owner of the handle.
4. Close intake on drain. Preserve thread handles through a cancelled/re-entered drain using the existing owned `Draining` state. Deadline expiry is a recorded overrun; dedicated workers remain owned and must actually exit before their owner fence can be released or success reported.
5. Join only handles whose `is_finished()` is true, so joining cannot starve an async worker or recreate the same pool cycle. Await between bounded scans of outstanding handles. An optional completion Notify needs periodic asynchronous fallback: notification can occur before thread exit, while `is_finished()` is still false, with no subsequent notification.
6. Combine async and dedicated counts using checked arithmetic and record each terminal outcome and deadline classification once. Preserve domain HandlerError, panic, cancellation/refusal, and the existing orthogonal failed count. A thread-spawn error restores admission and creates no accepted-but-unowned work.

This introduces neither an acquisition engine nor a persistent runtime. Its OS thread/stack costs must be explicitly bounded by the admitted lane and measured before making performance claims.

An ordinary supervised async Results task is not a drop-in repair: current drain aborts async tasks, while the transport performs synchronous Store/archive I/O inline. That alternative needs bounded offload for each synchronous phase, ownership of those phase workers after parent cancellation, and explicit cancelled-I/O finalization/recovery before releasing the work fence.

A current-thread Tokio runtime per stage with the existing shared Fetcher is also not sound. `jurisdiction/stages.rs:31–49` caches and shares one `Arc<Fetcher>` across jurisdiction clones. It owns a pooled reqwest Client. Local vendor source confirms reqwest 0.13.4 uses hyper-util's TokioExecutor, which spawns connection drivers on the current runtime. A pooled connection made under stage A can be used by stage B; dropping A's short-lived runtime can then invalidate B's transport. Tokio explicitly states runtime-bound I/O stops functioning on runtime drop. Isolating all HTTP and BrowserLane clients while preserving shared pacing and security would be a separate, broader change.

## Reviewed invariants without an additional static defect

- Actual run/pass handlers compare `ctx.key()` to existing `WorkflowIdentity` before stage work. `require_bound_manifest` requires lifecycle-4 retained manifest, exact writer lineage, run, CO2027 and jurisdiction membership before those handlers load and run stages. Legacy missing-manifest behavior remains unchanged.
- The existing `load_object` validates retained state identity. Actual `history_sources(state)` derives SourceSelection from `plan.sweepable`. Admission rechecks actual key/state identity, exact bound HistoryWindow and selected year, loads the retained manifest and hashes its exact value.
- MileSplit selection requires membership in the actual selected sources, `plan.sweepable`, and `MilesplitResults` dispatch. `UnadoptedSources` does not manufacture the durable role. Plan fingerprint is retained as the adopted dispatch witness, not misrepresented as source qualification.
- The required capture enum is threaded through history, stage wrapper, jobs re-export, Results collection and the actual MileSplit call. All discovered result-set references are checked for the same parsed site and meet before target-bound grant issuance. Source site remains the parsed site's jurisdiction rather than being replaced with owner jurisdiction.
- `GrantedTargets` permits one slot claim per per-year stage continuation, bounds the set at 65,536, uses fallible reserve, and returns an explicit duplicate refusal. It is not an automatic retry loop. Store enforces the lifetime three-attempt cap across later continuations.
- The WorkAdmission extraction preserves TeamsSource's work-reservation-before-key-gate order, registration worker ownership, weak-gate cleanup and shared clone behavior. The map now belongs to Jobs; production endpoint composition passes clones of the same Jobs instance.
- In the frozen Results path, the actual blocking worker owns the original `ctx.key()` guard. Dropping the handler's receiver cannot drop that worker's guard. HandlerError is preserved rather than flattened. Panic/cancellation are explicit boundary outcomes. The nested stage does not reacquire a Jobs Work permit; CS-01 concerns the distinct Tokio blocking-pool dependency.
- Bootstrap's explicit lifecycle-4 Store open precedes listener/browser initialization. Default open remains legacy. The CLI routes capture init/show synchronously before runtime creation and requires an explicit store root.
- Existing history wire definitions are replaced by the adopted census-domain re-export, leaving HistoricalProgress service-owned. No new serialized request field, source identity, run manifest or workflow key was found in this patch.

Core and transport owners independently confirmed current service-facing signatures match the frozen proposal: owner construction, target slot, continuation grant, `CaptureContext` variants, `ResultsCaptureContext::new`, and lifecycle-4 legacy-role refusal before structured HTTP. This is API inspection, not compiler evidence.

## Verification and remaining evidence

Executed read-only/static commands:

- `(cd source && sha256sum -c ../source-pins.sha256)`: all 53 OK, saved in `source-pin-verification.txt`.
- `(cd base && sha256sum -c ../base-pins.sha256)`: all 40 OK, saved in `base-pin-verification.txt`.
- `git diff --no-index --check base source`: exit 1 with zero output bytes. No whitespace diagnostic was emitted; exit 1 reflects the differing trees and is not recorded as a successful test.
- `wc -l` for the four base/source size files: 295, 298, 314, 305; `static-scans.txt` records them.
- Added-line lexical scan for unwrap/expect/unsafe/panic/todo/unimplemented/unreachable names: no matches. This is a limited text scan, not the trusted AST/lexical gate.
- Relevant source/contract inspection with `nl`, `sed`, `cat`, `rg` and contextual diffs; no runtime execution.

An initial checksum invocation was made from the proposal directory rather than its `source/` directory and could not resolve its relative paths. `source-pin-wrong-cwd.txt` preserves that failed attempt; the corrected complete verification above is authoritative. This was not a hash mismatch.

Not run: formatter, compiler, tests, lint, benchmarks, native Restate, HTTP or qualification/census. Root owns all of them.

Required Root evidence includes an actual Results action under `max_blocking_threads(1)`, exact original-owner handler cancellation and reacquisition, repeated run/pass/restart cap preservation, success/domain-error/panic accounting, spawn refusal cleanup, dedicated worker notification-before-exit, cancelled drain/re-entry, and deadline overrun drain. Existing TeamsSource cancellation/replay suites must remain unchanged in behavior. Physical-request counts, prepared recovery and publication interruption need the actual integrated consumer, not only synthetic Store tests.

## Standards consulted

AGENTS, ARCHITECTURE, ADR-002, ADR-004, ADR-037, ADR-038; adopted capture admission contract and core API; recovery skills `rust-contract`, `holzman-rust`, `async-rust-reviewer`, `scott-ddd-refactor`, `architectural-drift`.

Applicable references: Holzmann `references/nasa-jpl-standards.md`, `references/runtime-performance-architecture.md`; async reviewer `references/structural-correctness.md`, `references/spawn-discipline.md`, `references/cancellation-safety.md`; Scott DDD `checklist.md`.

Power-of-Ten bounded grants and checked allocation have identifiable static bounds; short-file compliance fails CS-02; warnings/build/proof/runtime evidence remains unexecuted. No throughput, latency, benchmark, or global readiness claim is made.
