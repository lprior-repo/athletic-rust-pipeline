
use std::future::Future;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use tokio::sync::oneshot;
use tokio::task::JoinSet;

use crate::outcome::{DrainState, Outcome};
use census_store::clock::{Clock, SystemClock};

mod ledger;
use ledger::Ledger;

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

#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    #[error("task count does not fit u64")]
    TaskCountOverflow,
}

enum Completion<T, E> {
    Returned(Result<T, E>),
    Panicked,
}

#[derive(Default)]
struct Region {
    tasks: JoinSet<()>,
    ledger: Ledger,
}

impl Region {
    fn reap_finished(&mut self) {
        while let Some(joined) = self.tasks.try_join_next() {
            self.ledger.classify(DrainState::from_join(joined));
        }
    }
}

pub struct Spawner {
    region: Mutex<Region>,
}

impl Default for Spawner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spawner {
    pub fn new() -> Self {
        Self {
            region: Mutex::new(Region::default()),
        }
    }

    pub fn adopting(tasks: JoinSet<()>) -> Result<Self, SpawnError> {
        let held = narrow(tasks.len())?;
        Ok(Self {
            region: Mutex::new(Region {
                tasks,
                ledger: Ledger::holding(held),
            }),
        })
    }

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

    #[tracing::instrument(skip_all)]
    pub async fn blocking<T, E, F>(&self, job: F) -> Outcome<T, E>
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        self.push_blocking(move || {
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
            Err(_) => Outcome::Cancelled,
        }
    }

    #[tracing::instrument(skip_all, fields(timeout_secs = timeout.as_secs()))]
    pub async fn drain(&self, timeout: Duration) -> Result<TaskReport, SpawnError> {
        let mut region = self.take();
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
                            const REAP_TURNS: usize = 8;
                            for _ in 0..REAP_TURNS {
                                while let Some(joined) = region.tasks.try_join_next() {
                                    region.ledger.classify_reaped(DrainState::from_join(joined));
                                }
                                if region.tasks.is_empty() {
                                    break;
                                }
                                tokio::task::yield_now().await;
                            }
                            region.ledger.set_remaining(narrow(region.tasks.len())?);
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

    fn push_blocking<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let mut region = self.lock();
        region.tasks.spawn_blocking(job);
        region.ledger.accept();
        region.reap_finished();
    }

    fn take(&self) -> Region {
        let mut region = self.lock();
        Region {
            tasks: std::mem::take(&mut region.tasks),
            ledger: std::mem::take(&mut region.ledger),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Region> {
        match self.region.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

fn narrow(count: usize) -> Result<u64, SpawnError> {
    u64::try_from(count).map_err(|_| SpawnError::TaskCountOverflow)
}

fn publish<T, E>(tx: oneshot::Sender<Completion<T, E>>, completion: Completion<T, E>) {
    if tx.send(completion).is_err() {
        tracing::debug!("a region job finished with no caller waiting for its value");
    }
}

#[cfg(all(feature = "loom", test))]
mod loom_tests;
#[cfg(test)]
mod tests;
