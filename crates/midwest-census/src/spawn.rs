//! The region's task spawner: one owned set that every task a region starts joins back through.
//!
//! A `tokio::spawn` (or a `spawn_blocking`) whose handle is dropped is an orphan factory: the task
//! outlives the region that started it, and shutdown cannot tell whether it is still writing. This
//! module is the opposite contract. One [`Spawner`] owns one `JoinSet`, the region starts its tasks
//! only through it, completion classifies through [`Outcome`] and [`DrainState`], and
//! [`Spawner::drain`] reaps the region inside a deadline — counting the natural exits first, then
//! aborting and counting whatever outlived it — so the owner can finalize knowing that nothing of
//! its own is still running.
//!
//! Blocking work rides the same set. A blocking job that is still queued is aborted like any other
//! task; a job that already started runs to completion, because the blocking pool cannot interrupt
//! it. Either way the region waits for it before its drain returns, which is what keeps a store's
//! finalize from racing a writer the region had forgotten it started.
//!
//! Draining is a region-level decision and happens once: the owner drains after nothing can start
//! more work. A task started after that lands in the region's fresh, empty set — the next drain's
//! business, never this one's.

use std::future::Future;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use tokio::sync::oneshot;
use tokio::task::JoinSet;

use crate::clock::{Clock, SystemClock};
use crate::outcome::{DrainState, Outcome};

mod ledger;
use ledger::Ledger;

/// What one region did with the tasks it owned.
///
/// These are the drain report's counters, and they are counted once each: at the end of a drain
/// `accepted == completed + cancelled + panicked + aborted`. `timed_out` and `remaining` carry the
/// same number — how many tasks the deadline found still in flight — and `aborted` counts how many
/// of those the abort then reclaimed. Counts saturate; they never wrap into a smaller, quieter
/// number.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TaskReport {
    pub accepted: u64,
    pub completed: u64,
    pub cancelled: u64,
    pub timed_out: u64,
    pub remaining: u64,
    pub aborted: u64,
    pub panicked: u64,
}

/// A task count that does not fit the report's `u64` field.
///
/// The counting is exact or it is nothing: a count that does not fit is a typed refusal, not a
/// clamped figure that would read as a smaller, quieter region.
#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    /// A set size did not fit the field it was counted into.
    #[error("task count does not fit u64")]
    TaskCountOverflow,
}

/// What a region-owned blocking job published to the caller that started it.
enum Completion<T, E> {
    /// The job returned, with its own result.
    Returned(Result<T, E>),
    /// The job panicked before it could return.
    Panicked,
}

/// The tasks one region owns, plus the ledger of everything it has counted.
///
/// One lock holds both, so the counters can never disagree with the set they describe.
#[derive(Default)]
struct Region {
    tasks: JoinSet<()>,
    ledger: Ledger,
}

impl Region {
    /// Count every task that finished since the last look and drop its entry.
    ///
    /// Reaping here is what keeps a long-lived region's set bounded: a set that is only drained at
    /// shutdown holds one entry for every job the region ever ran.
    fn reap_finished(&mut self) {
        while let Some(joined) = self.tasks.try_join_next() {
            self.ledger.classify(DrainState::from_join(joined));
        }
    }
}

/// The region-owned spawner: start tasks, and reap them inside a deadline.
pub struct Spawner {
    region: Mutex<Region>,
}

impl Default for Spawner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spawner {
    /// A region that owns nothing yet.
    pub fn new() -> Self {
        Self {
            region: Mutex::new(Region::default()),
        }
    }

    /// A region that adopts an already-built set, counting what it already holds.
    ///
    /// The shell's drain tests hand it the set they built by hand, so their report describes
    /// exactly the set they handed in while the counting itself lives here, once.
    pub fn adopting(tasks: JoinSet<()>) -> Result<Self, SpawnError> {
        let held = narrow(tasks.len())?;
        Ok(Self {
            region: Mutex::new(Region {
                tasks,
                ledger: Ledger::holding(held),
            }),
        })
    }

    /// Start a region-owned task. Its completion is counted, and a drain can abort it.
    #[tracing::instrument(skip_all)]
    pub fn spawn<F>(&self, task: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let mut region = self.lock();
        region.tasks.spawn(task);
        region.ledger.accept();
        region.reap_finished();
    }

    /// Run `job` on the blocking pool as a region-owned task, and classify what came back.
    ///
    /// The job belongs to the region, not to the caller: a caller cancelled mid-await costs the
    /// region nothing, because the job stays the region's to reap.
    #[tracing::instrument(skip_all)]
    pub async fn blocking<T, E, F>(&self, job: F) -> Outcome<T, E>
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        self.push_blocking(move || {
            // A panicking job publishes that truth *before* its unwind resumes: the caller must not
            // read "no value came back" as a cancellation when the region counted a panic, and the
            // resumed unwind keeps the join error a panic, so both counts stay honest.
            match catch_unwind(AssertUnwindSafe(job)) {
                Ok(result) => publish(tx, Completion::Returned(result)),
                Err(payload) => {
                    publish(tx, Completion::Panicked);
                    resume_unwind(payload);
                }
            }
        });
        match rx.await {
            Ok(Completion::Returned(Ok(value))) => Outcome::Ok(value),
            Ok(Completion::Returned(Err(error))) => Outcome::Err(error),
            Ok(Completion::Panicked) => Outcome::Panicked,
            // No value and no panic: the job was aborted before it ran (a region shutdown), so the
            // caller learns the region stopped instead of a value it must not trust.
            Err(_) => Outcome::Cancelled,
        }
    }

    /// Reap the region inside `timeout`: the natural exits first, then abort and count the rest.
    ///
    /// Blocking jobs already running are waited for rather than abandoned, and the report says so:
    /// what the deadline found still in flight is `timed_out` and `remaining`, and what the abort
    /// actually reclaimed is `aborted`.
    #[tracing::instrument(skip_all, fields(timeout_secs = timeout.as_secs()))]
    pub async fn drain(&self, timeout: Duration) -> Result<TaskReport, SpawnError> {
        let mut region = self.take();
        // `checked_add` keeps the deadline arithmetic panic-free. A timeout the clock cannot
        // represent is a timeout there is no deadline to reach — the region is waited for — rather
        // than a number clamped into something quieter than what the caller asked for.
        let deadline = SystemClock.now().checked_add(timeout);
        while !region.tasks.is_empty() {
            let joined = match deadline {
                Some(deadline) => {
                    match tokio::time::timeout_at(deadline, region.tasks.join_next()).await {
                        Ok(joined) => joined,
                        Err(_) => {
                            let remaining = narrow(region.tasks.len())?;
                            tracing::warn!(remaining, "drain deadline reached; aborting");
                            region.ledger.note_deadline(remaining);
                            region.tasks.abort_all();
                            while let Some(joined) = region.tasks.join_next().await {
                                region.ledger.classify_reaped(DrainState::from_join(joined));
                            }
                            break;
                        }
                    }
                }
                None => region.tasks.join_next().await,
            };
            match joined {
                Some(joined) => region.ledger.classify(DrainState::from_join(joined)),
                None => break,
            }
        }
        Ok(region.ledger.report())
    }

    /// Register `job` as a region task and count it as accepted.
    fn push_blocking<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let mut region = self.lock();
        region.tasks.spawn_blocking(job);
        region.ledger.accept();
        region.reap_finished();
    }

    /// Take the set and its counters, leaving the region empty for whatever comes next.
    fn take(&self) -> Region {
        let mut region = self.lock();
        Region {
            tasks: std::mem::take(&mut region.tasks),
            ledger: std::mem::take(&mut region.ledger),
        }
    }

    /// The lock is held only across bookkeeping — register, count, release — so no guard ever
    /// crosses an `await`. A poisoned lock is recovered instead of propagated: the panic happened
    /// inside the region's own bookkeeping (spawning outside a runtime is the one such path), and
    /// the counters and the set are still structurally sound.
    fn lock(&self) -> MutexGuard<'_, Region> {
        match self.region.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

/// A set size as a report field.
fn narrow(count: usize) -> Result<u64, SpawnError> {
    u64::try_from(count).map_err(|_| SpawnError::TaskCountOverflow)
}

/// Hand a result to the caller that is waiting for it. A caller that is gone — cancelled, or
/// aborted with the region — makes the send fail, and the value is simply dropped.
fn publish<T, E>(tx: oneshot::Sender<Completion<T, E>>, completion: Completion<T, E>) {
    if tx.send(completion).is_err() {
        tracing::debug!("a region job finished with no caller waiting for its value");
    }
}

#[cfg(all(feature = "loom", test))]
mod loom_tests;
#[cfg(test)]
mod tests;
