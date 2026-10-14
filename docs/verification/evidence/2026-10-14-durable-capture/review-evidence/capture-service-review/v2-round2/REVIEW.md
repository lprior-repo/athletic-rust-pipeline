# V2 corrected freeze: focused ownership review

STATIC RESULT: No remaining confirmed source defect in the reviewed repair delta.

EXECUTION STATUS: Unverified. Root alone owns integration, formatting, builds, tests and runtime qualification. This review does not certify compilation, passing tests, native Restate, physical-request counts or lifecycle-4 acceptance.

## Exact reviewed artifact

- Source: `../../capture-service-admission-v2-proposal/`.
- Source manifest SHA-256: `8adcda448977520fd76fe661f67f104bc277b6e353f4ae105b4770cf056ac7f8`.
- All 69 source hashes and 45 original base hashes verified.
- `changed-files.txt` and `source-pins.sha256` are the exact inventory and source pin manifest.
- `source/` preserves all 69 reviewed source files independently of later writer activity.
- `patch-pins.sha256` pins the full 3,297-line integration patch and 1,818-line v1-to-v2 delta.
- The original v1 OPEN 2 review remains unchanged at `../REVIEW.md`. The first v2 OPEN 2 review and frozen snapshot remain at `../v2-round1/`.

The focused delta is dedicated worker execution, Region ownership, bounded admission, cancellation/drain, tracing, related regression fixtures, and coherent file-size extractions. The original admission/manifest/history/source/grant review is not silently replaced by this narrower review.

## Findings and dispositions

CS-01, original nested Tokio blocking-pool dependency: structurally repaired. `jurisdiction/results_work.rs:40–42` now calls `Spawner::dedicated(admission, job)`. The actual job runs in a bounded OS thread and uses the existing long-lived Tokio Handle. It does not occupy a Tokio blocking-pool worker while awaiting Results selection or DNS. No per-stage Tokio runtime or replacement shared Fetcher was introduced. Runtime confirmation remains required.

CS-02, original hard file-size regressions: structurally repaired. `bootstrap/serve.rs` is 255 lines, its `store.rs` extraction is 65, `cli/mod.rs` is 236 and `cli/run.rs` is 81. Related Jobs/Results helpers were coherently extracted. Root must still run the formatter and trusted file/callable size gates after integration.

CS-03, first-v2 ordinary blocking handoff refusal and potential parent/child collision: structurally repaired. Worker descriptor admission now queues on its independent worker bound, rather than refusing against a combined cap occupied by an async parent.

CS-04, first-v2 builder evaluation before descriptor admission: structurally repaired. `spawn.rs:171–188` checks closed state, reaps, and admits the independent async descriptor before invoking `build()`. The Coordinator reservation is already held.

These are static source dispositions, not `fixed_with_execution_evidence`. Root's required execution remains open. No new confirmed static defect was found in the corrected delta.

## Bounded ownership and wait graph

`spawn/region.rs:31–59` defines:

- A = JoinSet descriptor count minus ledger-tracked Tokio blocking descriptors, using checked subtraction.
- B = ledger-tracked Tokio blocking descriptors plus dedicated-thread descriptors, using checked addition.
- A ≤ N and B ≤ N; combined retained descriptors ≤ checked 2N.

Every insertion checks its correct dimension under the Region mutex. Async insertion checks A. Blocking and dedicated insertion check B. Handles remain counted until their completion is joined; returning a value or releasing the lane does not erase a descriptor.

The existing shared worker-lane semaphore is still acquired before either worker mode. A caller that has acquired a lane permit while B is full implies at least one retained worker has released its permit and is retiring. The caller can wait for worker reaping without depending on an async parent's A descriptor. A capacity-one `spawn_with` parent and its required child can coexist as A=1/B=1. Ordinary `spawn` retains its existing lane charge; this review does not grant a new right to recursively consume the same worker lane.

`spawn/blocking.rs:65–104` and `spawn/dedicated.rs:86–105` retain unstarted closures and permits across worker-room waits, release the Region mutex before awaiting, poll with a 10 ms interval, and observe stop state. Cancellation before successful insertion drops only unstarted work and returns its permit. Region closure prevents an effect from being accepted after drain begins.

Dedicated allocation and thread start are fallible. Metadata reservation, closed-state checking, successful start, handle insertion and acceptance are ordered under one Region lock. A failed start returns a typed error and leaves no accepted-but-unowned job. Standard threads are owned directly by Region rather than an abortable monitor task.

The dedicated OS thread installs the submitting tracing dispatcher and span, then retains both its permit and explicit WorkAdmission owner around the job, result publication and unwind. A disappearing handler only drops caller interest. The actual worker retains the original keyed source-work fence until it finishes its owned action. The existing HandlerError is preserved at the Results boundary.

## Drain and accounting

`spawn/drain.rs` closes region, Coordinator and worker-lane intake before draining. `Draining` owns the actual JoinSet and thread handles and restores them on cancellation, preserving a closed region and the accumulated ledger.

Dedicated threads are not aborted at grace expiry. They remain owned, are polled asynchronously and are joined after `is_finished()`. The std API expects that join to complete quickly after this signal; it is not a substitute for proving arbitrary thread-local destructors cannot block. No such application-owned destructor dependency was found in the production Results path.

Grace expiry records outstanding async/thread descriptors once, guarded by `aborting`, then aborts async work while retaining non-abortable worker effects. Re-entering a cancelled drain cannot double-count that deadline. The final checked `set_remaining(pending)` at line 43 clears stale remaining counts even if every thread finished and was reaped before either wait loop runs.

Successful joins, domain failures, panic, cancelled/aborted async work and outstanding counts remain separately classified. The existing `failed` count remains an orthogonal domain-error diagnostic, not an extra disjoint terminal bucket.

## Regression fixtures inspected, not executed

- `spawn/descriptor_tests.rs`: actual capacity-one async parents awaiting blocking and dedicated children; simultaneous A=1/B=1 and exact accepted/completed counts; untouched builder witness on descriptor refusal; controlled lane-release-before-worker-completion handoff for both follower modes; cancellation while either unstarted worker waits returns its lane with no accepted effect.
- `spawn/dedicated/tests.rs`: result publication before owner-tail exit; handler cancellation; cancelled deadline drain and re-entry; completion before re-entry; mixed async/thread deadline counting once; distinct domain-error and panic accounting; deterministic thread-start failure cleanup; span/dispatcher propagation.
- `work_admission/tests.rs`: real dedicated supervisor retains the original keyed WorkAdmission after handler cancellation and blocks a second claim until the first worker finishes.
- `results_arms/tests/dedicated_worker.rs`: worker_threads=1/max_blocking_threads=1, actual Results selector, then actual shared Fetcher/GuardedResolver and a localhost HTTP body in the same dedicated action. It asserts exact returned bytes and zero remaining work.
- Existing preadmitted blocking drain-race fixtures are adapted to the now-async private acceptance helper. The original saturated-lane follower-success assertion was not weakened.

The capacity-one selector/DNS fixture uses an empty source selection and an unadopted Results mode. It is appropriately scoped to the worker/runtime dependency; it does not establish lifecycle-4 MileSplit transport, Restate replay or attempt-cap qualification.

## Required Root verification

Run the canonical Moon check and format/lint/size gates after integration. Execute the original Spawner admission/accounting/shutdown suites plus the new dedicated/descriptor tests, Results capacity-one selector/DNS test, keyed WorkAdmission cancellation test and existing TeamsSource cancellation/replay suites. Inspect exact disjoint counts, not only task completion.

Then run integrated source-capture tests for repeated original-owner continuations, physical request counts, successful replay/prepared recovery, interrupted finalization and native restart. These are separate obligations from the static ownership review. No source qualification or production readiness claim follows from accepting this source delta for integration.

Executed here: source inspection; checksum verification of all 69 proposed and 45 base files; line counts; contextual diffs; `git diff --no-index --check base source`, which exited 1 with zero diagnostic bytes because the trees differ. Artifacts retain these observations. No compiler, formatter, test, lint, benchmark, HTTP or live service command ran.

VERDICT: BLOCKED 1 — Root-owned execution and qualification remain unrun; no remaining confirmed static source blocker in this focused delta.
