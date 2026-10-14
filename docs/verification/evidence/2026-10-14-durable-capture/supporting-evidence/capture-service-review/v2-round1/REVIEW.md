# V2 first freeze: focused static review

Status: OPEN 2 additional findings. The original v1 `../REVIEW.md` and its OPEN 2 history remain unchanged.

Reviewed source-pins SHA-256: `16ad072003aa9db25638a61e02187d3b36a7d2e12362dec3c1d0914e959f91b4`.
All 67 source hashes verified. This directory preserves that exact source snapshot and inventory, even while the writer prepares the next repair. No builds/tests/live source edits were performed.

## CS-03 — High: combined descriptor rejection changes ordinary blocking handoff to cancellation

`spawn.rs:188–192` calls `Region::require_room` after acquiring a worker-lane permit. A retiring blocking worker releases its lane permit before Tokio publishes its JoinSet completion. A queued successor can acquire that permit, find the predecessor still counted and not yet reapable, then be refused as `Outcome::Cancelled`. ADR-037 and the existing `a_saturated_blocking_lane_defers_the_next_effect_until_the_holder_releases` regression require the queued successor to execute successfully after the holder releases.

The first v2 dedicated tests avoid this race by explicitly waiting for reaping before trying another worker; that does not fix production admission. Waiting blindly for the same combined numeric cap is also insufficient: a `spawn_with` async parent at capacity one can await a required blocking child that is forbidden by its parent's descriptor, recreating the parent/child resource dependency.

Root adopted the following finite model for the next freeze:

- A = JoinSet descriptor count minus tracked Tokio-blocking descriptors, using checked subtraction.
- B = tracked Tokio-blocking descriptors plus dedicated-thread descriptors, using checked addition.
- A is independently bounded by N and B independently bounded by N; total retained metadata is bounded by checked 2N.
- Async insertion checks only A. Blocking/dedicated insertion checks only B and may asynchronously wait for retiring worker descriptors while retaining its unstarted job and already admitted lane permit.
- Acquiring a lane permit while B=N implies at least one retained worker has already released its lane and is retiring. Worker admission never waits on the async parent's A descriptor.
- Closed-region checks, no accepted entry before successful start, cancellation cleanup of unstarted jobs, and all existing drain ownership requirements remain.

Required regression: capacity-one `spawn_with` parent actually awaits a blocking child and a dedicated child; separately exercise a controlled lane-release-before-join-completion handoff. Do not weaken the original ordinary blocking queue-success assertion.

## CS-04 — Medium: spawn_with constructs its future before descriptor admission

`spawn.rs:156` evaluates `build()` before `push_async` checks `require_room` at lines 172–173. When Coordinator admission succeeds but the new descriptor bound refuses, the builder has already run. This breaks the existing builder-admission contract expressed by `closure_is_not_constructed_before_admission` for the new rejection path and permits constructor side effects without task admission.

Required repair: evaluate the builder only after Coordinator admission and the independent async descriptor check both succeed, with a race-safe reservation or insertion boundary. Add a descriptor-saturation regression whose builder toggles a witness; rejection must leave it untouched.

## Other focused observations

- Region directly owns each std JoinHandle, which removes the original shared Tokio blocking-pool dependency without a short-lived Tokio runtime or shared-Fetcher affinity loss.
- WorkAdmission is the explicit owner argument in `dedicated`; result publication does not release that owner. Join occurs only after `is_finished()`; polling is asynchronous and periodic.
- Cancelled drain restores owned Region state. The new explicit final `set_remaining(pending)` repairs the finished-before-reentry stale-count edge. Mixed deadline classification uses the existing `aborting` fence.
- Span plus dispatcher are installed in the dedicated OS thread. The accompanying fixture checks the instrumented dedicated span.
- The proposed one-blocking-thread fixture runs actual Results selection and real local Fetcher/DNS under the existing long-lived runtime. It is a proposal, not execution evidence, and does not certify native Restate or lifecycle-4 capture behavior.
- The v1 size regressions are structurally repaired in this freeze: bootstrap/serve.rs is 255 lines and CLI dispatch moved into cli/run.rs. Other coherent extractions provide formatting margin. Root still owns formatting and trusted size verification.

All findings and the adopted repair model were sent immediately to Root and the original writer. A second freeze, fresh pins and another focused review are required before Root integration.
