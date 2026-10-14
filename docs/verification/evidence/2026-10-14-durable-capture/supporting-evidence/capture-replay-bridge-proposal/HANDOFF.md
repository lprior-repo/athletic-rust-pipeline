# Results-stage replay and owner-admission bridge proposal

2026-10-14 06:35 UTC. Drafted and statically inspected only. No compilation, test execution, formatting, linting, native Restate process, external source access, live source edit, commit or publication was performed.

## TASK

Add focused regressions at the actual adopted Results workflow boundary. Exercise real Restate SDK replay dropping an unused run closure while that closure owns the original WorkAdmission, and verify same-key re-admission, zero new capture roots/attempts/HTTP activity, exact result/error preservation, and complete resource accounting.

## OWNERSHIP

All writes are confined to this proposal directory. Root owns integration and execution. The service-admission owner confirmed its v2 API is frozen and that this proposal does not overlap its compile-repair work. Three new test files and one test-module registration are proposed. No production function, public API, schema, dependency, or capture policy is changed.

## DO NOT MODIFY

Do not replace live files wholesale from staged. Apply the contextual patch or add the three new test files and the final cfg(test) module registration after checking the base pin. Root may have reformatted or repaired the live tree since this snapshot.

## INPUT CONTRACT

- Live HEAD observed: 5111a603cc1d945e2da3f85c00d149796552e43e. This is not a pin of the dirty integrated tree.
- Exact base SHA256 for results_work.rs: 3ba5f8135f6d81fe78996e5b8dd5e89dab89ee03e09b21436f6be6f62aa7c8dc.
- Per-file base/output pins: pins.json. Dependency observations: READ-PINS.sha256.
- Production composition reached: SDK ObjectContext → JurisdictionCensus::load_object → results_owed → collect_result_year → admit_results_owner → results_stage → ctx.run → dedicated owner worker → jobs::results_stage.
- The test-only ResultsStageBridge object only selects the Results entry point, supplies its deterministic observation date, and returns the resulting year entry. It does not implement a substitute replay engine, worker, admission state machine, or collection algorithm.
- Real lifecycle-4 capture-qualification Store, retained RunManifest, typed request, serialized retained JurisdictionState, original WorkflowIdentity, one selected owned MileSplit meet, shared production Jobs gate and capacity-1 Spawner.
- SDK v6 frames reuse national/publication_tests/codec. GetEagerState consumes completion ID 1, so the following recorded Results Run uses completion ID 2. This was checked against the pinned SDK shared-core source.

## OUTPUT CONTRACT

- capture-replay-bridge.patch: contextual four-path patch.
- staged/: three new Rust test/helper files plus results_work.rs with a test-module declaration only.
- base/: exact original results_work.rs.
- pins.json, OUTPUTS.sha256 and READ-PINS.sha256: content pins.
- test-inventory.md: assertion and coverage inventory.
- commands.md: Root-only Moon commands, not executed.
- static-inspection.txt: limited static inspection statement.

## ACCEPTANCE

Root must compile and execute the actual three tests with nonzero selection before claiming coverage. The first and third test expect exactly zero accepted supervised workers. The real executed-stage test expects exactly one accepted/completed dedicated worker across acquisition and replay. Every report is compared in full, including cancelled, aborted, panicked, failed, still_running, timed_out and remaining; accepted must equal disjoint_total. All coordinator resource fields must equal their pre-call values.

There is no passing result or async approval in this handoff. Formatting and all-target compilation remain unverified.

## HANDOFF

The highest-priority missing journal-skip composition now has proposed executable regressions invoking the actual Results workflow. The executed-stage case reaches selection, cached meet-page parsing, target grant, durable work registration, attempt reservation, real transport offline refusal and terminal persistence. The cached meet page is a synthetic fixture, not external evidence. Source HTTP is disabled; the loopback HTTP/2 endpoint is only the in-process SDK harness.

Important limits:

- No live whole-meet response body, archive commit or successful owned-result projection is acquired by this proposal.
- No caller-drop/old-running-worker ownership test is claimed. The service crate cannot use crawl's private fixture_resolution/TLS-client and capture-interruption fields, which are compiled only into crawl's own test target. Crossing that live response/archive boundary would require an explicitly designed cross-crate test-support seam or a separate native lane. Neither is added here.
- The offline case proves actual pre-dispatch transport finalization and same-key release. It does not prove cancellation during dispatch/archive.
- The outer JurisdictionCensus::run/pass orchestration, earlier stages, full historical multi-year iteration and real Restate persistence/restart are outside this focused bridge.
- No real performance claim, fault qualification, release qualification, national census or publication claim follows from these tests.

Required source skills read: rust-contract/SKILL.md, holzman-rust/SKILL.md, async-rust-reviewer/SKILL.md, scott-ddd-refactor/SKILL.md. Additional test-planner/SKILL.md and test-reviewer/SKILL.md informed coverage. Applicable Holzmann references read: references/nasa-jpl-standards.md and references/runtime-performance-architecture.md. All are under the recovery skills directory. Repository AGENTS.md, ARCHITECTURE.md, stage-composition.md and commands.md were read; Root's no-build/no-test instruction controls over skill execution templates.
