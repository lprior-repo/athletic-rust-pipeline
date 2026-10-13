# ADR-037 — Blocking work has its own lane

Status: accepted 2026-10-12; repairs the national run at `var/national-fresh-20261012-04`.

## Context

`Spawner::blocking` charged `WORK_CLASS_BLOCKING` from the same `Coordinator` pool that handler
work charges through `WORK_CLASS_STAGE`, `WORK_CLASS_CONTROL` and `WORK_CLASS_NORMAL`. A pool
dimension charged by a parent *and* by the nested work that parent needs cannot be scheduled:
the parent holds the dimension until it completes, the nested request needs the same dimension to
complete the parent, and neither can release. Waiting callers park; once the per-origin waiter
table fills, every further request is refused instead of queued, so the caller's job is cancelled
and retried into the same wall.

The evidence is the stalled national census: after the last collected team index at 19:15:39Z the
endpoint produced only `blocking admission refused; work remains owed` for OR, VA, OK and NY, 82
refusals in total, with the endpoint holding 37 threads all parked on futexes — no running job
could release anything. The refusal is `try_enqueue` overflow (`MAX_ORIGIN_WAITERS = 16`), not a
data error, so no retry policy could clear it.

## Decision

Blocking jobs take a dedicated `Semaphore` lane on the `Spawner`, bounded by the spawner's
capacity. The lane never charges an admission-pool dimension, so a reservation held by a caller
can no longer collide with the reservations its nested work needs.

- `Spawner::acquire_blocking` waits for a lane permit; saturation queues the caller instead of
  refusing it. A closed lane (drain) wakes waiters immediately and they return
  `Outcome::Cancelled` without running their effect.
- `Spawner::spawn` holds one lane permit for the task's lifetime, so a region whose spawned work
  fills the lane is also a region where a blocking job waits: the lane, not a pool dimension, is
  what bounds the whole region, and `push_async` re-checks the closed region before it accepts the
  task.
- `Spawner::push_blocking` takes the permit and holds it for the job's duration, releasing it when
  the effect finishes; the ledger's accounting (`accepted`, `started`, `completed`, `timed_out`,
  `still_running`) is unchanged.
- `WORK_CLASS_BLOCKING` is removed: the pool no longer models blocking work at all.
- Observation reads reserve from the Control class: the `TeamsSource` `state`, `progress` and
  `inspection` handlers read the ledger through `ledger_status_observed` /
  `ledger_inspection_observed`, which take control permits, so a saturated work plane cannot block
  census observation. The attempt-path read in `execution.rs::journaled_status` stays on the work
  class, where it is outermost (it runs before `admit`).
- Ingest steps (`Ingest::prepare_recording`) take `WORK_CLASS_INGEST`: one stage slot and no byte or
  work charge. The step is a child of the jurisdiction or attempt stage that already prices that
  work, so charging the work dimension again made a holder wait on a child that could not be
  admitted once ten stage charges (100,000 work units each against the 1,000,000 unit budget) were
  spent — the distributed form of the same parent/child collision. `WorkClass::validate` now
  refuses only a completely empty envelope, because a slot-only or byte-only envelope is still
  fully accounted.
- `MAX_ORIGIN_WAITERS` rises from 16 to 64. Waiters are counted per `(Class, origin)`, and the
  whole census arrives through one administrative origin, so the 49-jurisdiction fan-out shared a
  single bucket: a saturated work pool turned queued jurisdictions into refusals
  (`OriginWaitersFull`) — observed as `stage admission: class/origin waiter queue is full` from
  `census-service open-work` on 2026-10-12. The bucket the cap must cover is the concurrent
  Work-class waiter count, which the national workflow structures: jurisdiction passes are
  sequenced (one pass chain at a time), each pass awaits one source object before the next, and the
  remaining Work-class issuers are the browser-launch job and the publish/join lanes — an order of
  magnitude below 64. Control and drain waiters are counted in their own classes and add nothing to
  this bucket. A 49-pass plus 16-normal Work mixture would still reach the cap; the sequenced
  workflow does not produce that mixture, and reaching the cap refuses the caller rather than
  deadlocking it, so the margin over the structural bound is what matters, not the exact figure.
  The global `MAX_WAITERS` bound is unchanged.

## Consequences

The pool's dimensions (`bytes`, `work`, `slots`) now bound only work that nests into itself through
its own reservations, and blocking concurrency is bounded by the lane (capacity, 500 by default)
and by tokio's blocking pool. The pool no longer counts the 4 KiB blocking envelope: at lane
saturation that is 500 × 4096 = 2 MiB, negligible against the pool's byte budget, and thread
stacks are bounded by the blocking pool rather than by the envelope. A caller that finds the lane
saturated observes delay, never a cancellation, so a retry loop can no longer amplify saturation.

## Evidence

- `crates/census-service/src/spawn/admission_tests.rs::a_saturated_blocking_lane_defers_the_next_effect_until_the_holder_releases`
  (a held lane offers no permits, the queued effect runs only after release, effects run in order).
- `crates/census-service/src/spawn/shutdown_tests.rs::shutdown_wakes_waiting_admission_without_running_its_effect`
  (drain wakes a queued blocking caller as `Cancelled` without running its effect).
- `crates/census-service/src/admission/tests.rs::a_blocking_job_runs_while_its_caller_holds_the_only_task_slot`
  (a blocking job completes while every task slot is held).
- The live stall and the restarted run are recorded in
  `var/evidence/admission-lane-20261012.md`.
