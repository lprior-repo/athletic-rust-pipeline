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

Injection seams must signal the reached boundary and be gated behind explicit test/injection
configuration, unreachable in normal release operation. Record exact failpoint, process IDs,
request counters, seeds, invocations, exit codes and hashes; a guessed sleep is not a reached fault.

## Required reconciliation

For each fault, compare stable source-unit/capture/effect/decision IDs and content, not just counts.
Every acknowledged effect remains readable and evidence-resolvable; unfinished effects are resumed
or explicitly terminal with retained reasons. No double commit may be hidden by a later merge or
workbook deduplication. Publication retains the previous valid generation until a complete new one
passes independent verification. Scenario results belong with the tested build, run specification,
store and Restate versions, fixture/capture digests and actual resource measurements.
