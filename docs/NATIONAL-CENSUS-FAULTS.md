# Fresh national census — all 17 native fault scenarios

Status: required acceptance obligations, not execution evidence. This is the fault catalog for
[the national census master plan](NATIONAL-CENSUS-PLAN.md) and
[ADR-013](adr/ADR-013-fresh-national-source-census.md). No scenario is removed by the source-first
mission correction; faults exercise discovered source units, not private workbook rows.

## Execution contract

Use owned, isolated native non-Docker deployments and local-disk scratch state. Record the actual
injection point, owned processes, exact commands and exit/status, before/after oracle, recovery
result and cleanup. An unreached fault, skipped scenario, simulated error substituted for the real
fault, or unmeasured invariant is not PASS. Preserve terminal gaps as findings, not acquisitions.

Never reboot the shared workstation, kill shared model servers, modify its clock, exhaust its real
filesystems or open a live census store with a second writer. Capture process identities and durable
run/operation IDs so the evidence distinguishes real recovery from starting a replacement job.

## Required scenarios

| # | Injection and exact recovery obligation |
|---|---|
| 1 | SIGKILL the active endpoint; resume the same invocation and reconcile exact accepted observations/receipts without duplicate effects. |
| 2 | SIGKILL native Restate during national fan-out; retain stable parent/child identities, retry counts and exact remaining obligations. |
| 3 | Reboot an isolated native VM/machine with persistent local disks; prove boot identity changed and acknowledged work survives. Do not reboot the shared workstation. |
| 4 | Upgrade actual V1 to different V2 binaries with in-flight work and explicit wire/schema compatibility; prove single-writer store handoff, not merely restart one binary. |
| 5 | Exercise actual browser/HTTP responses for 429 + Retry-After, 500 and a challenge; prove taxonomy, physical admission and the three-attempt ceiling without challenge bypass. |
| 6 | Crash before/after durable effect and receipt boundaries; replay identical operation/content exactly once, reject changed-content reuse, distinguish legitimate new captures. |
| 7 | Concurrent workflows request the same logical unit; one durable effect and consistent progress, not duplicated rows hidden by export deduplication. |
| 8 | Multiple endpoints/workflows share an origin; observe physical requests and prove the aggregate in-flight/rate/burst budget is not multiplied. |
| 9 | Real Fjall ENOSPC on an isolated bounded filesystem; reject unacknowledged writes, retain acknowledged data and recover after space returns. |
| 10 | Real Restate ENOSPC on its own isolated bounded filesystem; verify acknowledged invocation/journal recovery and honest failure reporting. |
| 11 | Stop a parent with active owned workers; prove drain/reap and exact exclusive outcome accounting, with unfinished business durably resumable. |
| 12 | Cross midnight using an isolated native clock environment; run/cohort/season identities remain stable while real acquisition times remain honest. Never change the shared host clock. |
| 13 | Fault real model HTTP through an owned proxy and crash around advice persistence; reuse valid first-model advice, bound second-model retries, never accept on missing/disagreeing advice. Do not kill shared GPU servers. |
| 14 | Attempt sealing with each unmet acceptance item; refuse with exact reasons/counts, retain allowed terminal findings and never substitute zero for an unmeasured item. |
| 15 | Cold-backup and restore a nonempty corpus; reconcile exact evidence, captures, decisions/history and derived state; missing/corrupt components fail. Distinguish store backup from Restate recovery. |
| 16 | Replay frozen real captures and retained advice with fixed run semantics; exact IDs and semantic artifacts match. Separate nondeterministic operational measurements rather than fabricate them. |
| 17 | Kill the actual worker while its batch is demonstrably in flight; prove the injection window, atomic batch visibility and exact remaining-unit recovery, not a guessed sleep. |

## Required phase-boundary subcases

The seventeen scenario IDs also cover the production-path boundaries retained from the superseded
delivery brief; passing a broad restart test does not discharge them:

- **Scenarios 1 and 6:** kill before issuing a source request; after receiving a response but before
  capture commit; after capture commit but before parse completion; before the atomic Fjall
  application; and after commit but before the Restate acknowledgement. Retain honest uncertain
  attempt accounting, reuse committed captures, and expose neither partial rows nor partial receipts.
- **Scenarios 6 and 17:** kill after some deterministic chunks of one source page. Reuse completed
  chunks and finish the same manifest, without resetting ordinals or appending duplicate effects.
- **Scenarios 1 and 2:** restart the endpoint and native Restate independently, preserving each
  durable directory and the same invocation identity; a fresh deployment is not recovery.
- **Scenario 16:** quarantine one malformed source object while unrelated objects continue; never
  turn a parse failure into a successful empty source. Retain its bytes and rejection locator.
- **Scenario 13:** malformed and timed-out model responses remain explicit review outcomes; transport
  failure cannot manufacture a match or no-match.
- **Scenarios 14 and 16:** interrupt export before promotion. The previous accepted generation remains
  published, and a newer partial file is not selected by filename or timestamp.
- **Scenario 15:** interrupt backup and restore. The original store and previous accepted backup
  remain recoverable; restore uses a new owned destination.
- **Scenarios 6 and 17:** kill a worker while a derived generation is materializing. The previous
  generation remains current and readable; the incomplete generation is invisible to every reader
  and to the next pass, which either publishes a fresh generation or reclaims the abandoned one;
  no reader observes two passes mixed across tables.

Injection seams must signal the reached boundary and be gated behind explicit test/injection
configuration, unreachable in normal release operation. Record exact failpoint, process IDs,
request counters, seeds, invocations, exit codes and hashes; a guessed sleep is not a reached fault.

## Consolidate export crash boundary

Scenarios 1 and 6 also reach the consolidate export: the `Consolidate` workflow writes its durable
`out/*.jsonl` snapshots before it answers the invocation, so a crash between those two moments must
replay the same operation onto the same snapshots instead of forking a second execution. The
post-effect/pre-ack point is the `consolidate_durable_before_reply` boundary, gated behind the
`native-fault-injection` feature: a normal release build carries no seam, reads no arming variable
and returns no fault.

| Control | Contract |
|---|---|
| `CENSUS_NATIVE_CONSOLIDATE_BOUNDARY` | Arming scope: the exact operation `consolidate:{key}`, where `{key}` is the `Consolidate` workflow key of the run being faulted. Only that operation reaches the boundary; unrelated operations and every operation in a process that is not armed consolidate normally. |
| `CENSUS_NATIVE_CONSOLIDATE_MARKER` | Absolute path of the reached-boundary artifact. It is published without overwrite before the acknowledgement, and records the schema, the phase, the id of the process that reached it, the operation, and for every requested table the row count with the sha256 of the durable snapshot bytes. |
| `CENSUS_NATIVE_CONSOLIDATE_HOLD_MILLIS` | Optional hold window in milliseconds, default and maximum 60000. The reached boundary holds the operation open so the harness can deliver SIGKILL; expiry refuses the operation terminally rather than continuing, because a guessed sleep is not a reached fault. |

The reached-boundary latch is one-shot per endpoint process and is consumed by the armed operation,
so the arming scope cannot leak into a later operation; unrelated operations never inherit it. Reset
is explicit: clear the variables and restart the endpoint. The replay expectation is that the
harness restarts the endpoint without the arming variables and resumes the same invocation
identity, after which the replayed consolidation returns the committed snapshots byte for byte (the
sha256 digests recorded in the marker) exactly once, and the store gains no duplicate rows.

The ordinary injected error `ATHLETIC_FAULT_EXIT_AFTER_CONSOLIDATE` keeps its historical name and
stays separately named and typed (`StoreError::Invariant`): it is acknowledged as a terminal error,
never publishes a reached-boundary artifact, and never crashes a process.

Isolated harness run (owned processes, local-disk scratch state, never an active user census):

1. Seed an owned store, then start an owned native Restate node and an endpoint built with
   `native-fault-injection`.
2. Arm that endpoint with the three controls above, submit `Consolidate/{key}/run`, wait for the
   reached-boundary artifact, and SIGKILL the endpoint before the invocation is acknowledged.
3. Restart the endpoint with the arming variables cleared, resume the same invocation id, and
   compare the replayed reply and the snapshot bytes with the marker inventory and digests.

`c_consolidate_boundary_crash_replays_the_committed_snapshots` in
`crates/census-service/tests/restate_kill_restart.rs` performs that run, and records the reached
process id, phase, operation, per-table digests and the resumed invocation identity as evidence.

## Source, worker and HTTP boundary controls

The remaining injection points named above are armed through four controls, all gated behind the
`native-fault-injection` feature: an unarmed release build carries no seam, reads no arming variable
and returns no fault. Every row names its carrier and the smallest real command already available
for that boundary; a passing in-process ordering test does not discharge the reached-SIGKILL
obligation, and a reached marker without the wrapper's reconciliation oracle is not PASS.

| Boundary | Control | Carrier | Smallest real command |
|---|---|---|---|
| Source request, before issuance | `CENSUS_NATIVE_SOURCE_BOUNDARY` schema 2, `kind: before_source_request` | `crates/census-crawl/src/milesplit/boundary.rs` through the service hook at `crates/census-service/src/restate_services/jurisdiction/team_source/native_boundary.rs` | `env -u CI tools/moon-local run pipeline:tests -- -E 'test(fetch_roster_fires_all_boundary_points_in_order)'` |
| Response received, before capture commit | same config, `kind: response_received` | as above | as above |
| Capture committed, before parse completion | same config, `kind: capture_committed` | as above | as above |
| Page chunk, after a deterministic chunk | same config, `kind: page_chunk` with `chunk_index` | as above | as above |
| Before the atomic Fjall application | same config, `kind: before_apply` | `crates/census-service/src/census/sweep/roster/commit.rs` | `SERVE_BINARY=<native-fault-injection endpoint> env -u CI tools/moon-local run pipeline:durability -- scenario-06-no-duplicate-evidence` |
| After commit, before the Restate acknowledgement | same config, `kind: after_commit_before_ack` | as above | as above |
| Teams source reservation, pre-acquisition | `CENSUS_NATIVE_SOURCE_BOUNDARY` schema 1 (`schema`, `operation`, `attempt`, `timeout_seconds`) | `crates/census-service/src/restate_services/jurisdiction/team_source/native_boundary/` | `… pipeline:durability -- scenario-02-restate-kill-during-fanout` |
| Store worker batch and derived generation | `CENSUS_NATIVE_WORKER_BOUNDARY` = `source_batch_staged_before_commit`, `source_chunk_committed_before_next` or `derived_batch_staged_before_publish`, with `CENSUS_NATIVE_WORKER_MARKER` | `crates/census-store/src/native_worker_boundary.rs`, spawned by `crates/census-service/tests/recovery_mod/native_worker.rs` | `env -u CI tools/moon-local run pipeline:durability -- scenario-17-recovery-tests` |
| HTTP serve exit | `ATHLETIC_FAULT_HTTP_EXIT`: empty or `1` exits at startup, any other value names the trigger file whose appearance exits the server | `crates/census-service/src/bootstrap/serve.rs` | `… pipeline:durability -- scenario-11-parent-exit` |
| Consolidate export, post-effect and pre-ack | `CENSUS_NATIVE_CONSOLIDATE_BOUNDARY` with its marker and hold controls | own section above | `c_consolidate_boundary_crash_replays_the_committed_snapshots` in `crates/census-service/tests/restate_kill_restart.rs` |

The scenario-01 wrapper runs the service `restate_kill_restart` suite for the source/capture/apply/ack
points (`… pipeline:durability -- scenario-01-endpoint-kill`), and scenario 17's derived-generation
subcase carries the atomic-visibility obligation. Each armed lane keeps its configuration and
reached-boundary marker in one process-owned 0700 directory; the marker is
`teams-source-reservation-reached.json` and is published without overwrite. Its schema-2 payload
records the phase, the real operation, the attempt, an observation instant, the request digest and
the acknowledged effects, and placeholder identity (`fault_injection`) is rejected by the wrappers
and by the receipt oracle instead of counting as a reached boundary. Configuration is bounded to
4 KiB, marker readback to 32 MiB and the hold to at most 60 seconds; expiry refuses the injection
rather than continuing normally. The per-boundary reached/recovered certificates of scenario 17 and
the marker inventory of scenario 06 are the evidence objects; a marker alone proves reachability,
not recovery.

The controls assume verified resources. `tools/durability/native-resources.sh`
(`env -u CI tools/moon-local run pipeline:native-resources`) verifies the pinned Restate 1.7.10
server, the QEMU 11.1.1 prefix with its staged package checksums, the pinned Arch guest image and
its published digest, the live guest capture manifest, Chromium and the scenario prerequisite
tools, and prepares the owned `var/durability-scratch` disks plus the persistent guest disk parent.
The same verifier fails closed when the unprivileged user+mount namespace or its bounded tmpfs
media is unavailable, because scenarios 9 and 10 depend on that media; it records the observed host
clock instant and never sets the host clock. See
[the harness README](../tools/durability/README.md#owned-native-resources) for the pinned
identities and the `--certificate` evidence object.

## Required reconciliation

For each fault, compare stable source-unit/capture/effect/decision IDs and content, not just counts.
Every acknowledged effect remains readable and evidence-resolvable; unfinished effects are resumed
or explicitly terminal with retained reasons. No double commit may be hidden by a later merge or
workbook deduplication. Publication retains the previous valid generation until a complete new one
passes independent verification. Scenario results belong with the tested build, run specification,
store and Restate versions, fixture/capture digests and actual resource measurements.
