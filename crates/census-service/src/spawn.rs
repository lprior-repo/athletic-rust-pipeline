use std::future::Future;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::sync::{oneshot, watch, OwnedSemaphorePermit, Semaphore, TryAcquireError};
use tokio::task::JoinSet;

use crate::outcome::{DrainState, Outcome};

mod drain;
mod ledger;
use ledger::Ledger;

pub const DEFAULT_CAPACITY: usize = 500;

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
    #[error("the task region already supervises its {capacity} tasks")]
    RegionFull { capacity: usize },
    #[error("the task region's capacity is closed")]
    RegionClosed,
}

enum Completion<T, E> {
    Returned(Result<T, E>),
    Panicked,
}

#[derive(Default)]
struct Region {
    tasks: JoinSet<()>,
    ledger: Ledger,
    blocking: Arc<AtomicUsize>,
    closed: bool,
    aborting: bool,
}

impl Region {
    fn reap_finished(&mut self) {
        while let Some(joined) = self.tasks.try_join_next() {
            self.ledger.classify(DrainState::from_join(joined));
        }
    }
}

pub struct Spawner {
    capacity: usize,
    permits: Arc<Semaphore>,
    region: Mutex<Region>,
    draining: tokio::sync::Mutex<()>,
    stopping: watch::Sender<bool>,
}

impl Default for Spawner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spawner {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let bound = capacity.clamp(1, Semaphore::MAX_PERMITS);
        Self {
            capacity: bound,
            permits: Arc::new(Semaphore::new(bound)),
            region: Mutex::new(Region::default()),
            draining: tokio::sync::Mutex::new(()),
            stopping: watch::Sender::new(false),
        }
    }

    pub fn adopting(tasks: JoinSet<()>) -> Result<Self, SpawnError> {
        let held = narrow(tasks.len())?;
        let capacity = DEFAULT_CAPACITY.max(tasks.len());
        Ok(Self {
            capacity,
            permits: Arc::new(Semaphore::new(capacity.saturating_sub(tasks.len()))),
            region: Mutex::new(Region {
                tasks,
                ledger: Ledger::holding(held),
                blocking: Arc::new(AtomicUsize::new(0)),
                closed: false,
                aborting: false,
            }),
            draining: tokio::sync::Mutex::new(()),
            stopping: watch::Sender::new(false),
        })
    }

    pub fn stopping(&self) -> watch::Receiver<bool> {
        self.stopping.subscribe()
    }

    #[tracing::instrument(skip_all)]
    pub fn spawn<F>(&self, task: F) -> Result<(), SpawnError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let slot = self.admit()?;
        let mut region = self.lock();
        if region.closed {
            return Err(SpawnError::RegionClosed);
        }
        region.reap_finished();
        region.tasks.spawn(holding_slot(slot, task));
        region.ledger.accept();
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn blocking<T, E, F>(&self, job: F) -> Outcome<T, E>
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
    {
        let Ok(slot) = Arc::clone(&self.permits).acquire_owned().await else {
            tracing::error!(
                capacity = self.capacity,
                "the task region's capacity is closed; the blocking job was not run"
            );
            return Outcome::Cancelled;
        };
        let (tx, rx) = oneshot::channel();
        if !self.push_blocking(
            move || match catch_unwind(AssertUnwindSafe(job)) {
                Ok(result) => publish(tx, Completion::Returned(result)),
                Err(payload) => {
                    publish(tx, Completion::Panicked);
                    resume_unwind(payload);
                }
            },
            slot,
        ) {
            return Outcome::Cancelled;
        }
        match rx.await {
            Ok(Completion::Returned(Ok(value))) => Outcome::Ok(value),
            Ok(Completion::Returned(Err(error))) => Outcome::Err(error),
            Ok(Completion::Panicked) => Outcome::Panicked,
            Err(_) => Outcome::Cancelled,
        }
    }

    fn admit(&self) -> Result<OwnedSemaphorePermit, SpawnError> {
        let acquired = Arc::clone(&self.permits).try_acquire_owned();
        match acquired {
            Ok(slot) => Ok(slot),
            Err(TryAcquireError::NoPermits) => {
                tracing::warn!(
                    capacity = self.capacity,
                    "the task region is at capacity; refusing to supervise more work"
                );
                Err(SpawnError::RegionFull {
                    capacity: self.capacity,
                })
            }
            Err(TryAcquireError::Closed) => Err(SpawnError::RegionClosed),
        }
    }

    fn push_blocking<F>(&self, job: F, slot: OwnedSemaphorePermit) -> bool
    where
        F: FnOnce() + Send + 'static,
    {
        let mut region = self.lock();
        if region.closed {
            tracing::warn!("the task region is closed; the blocking job was not run");
            return false;
        }
        region.reap_finished();
        let blocking = Arc::clone(&region.blocking);
        blocking.fetch_add(1, Ordering::AcqRel);
        region.tasks.spawn_blocking(move || {
            let _slot = slot;
            let _owing = BlockingGuard(blocking);
            job();
        });
        region.ledger.accept();
        true
    }

    fn lock(&self) -> MutexGuard<'_, Region> {
        match self.region.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

struct BlockingGuard(Arc<AtomicUsize>);

impl Drop for BlockingGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

async fn holding_slot<F>(slot: OwnedSemaphorePermit, task: F)
where
    F: Future<Output = ()>,
{
    let _slot = slot;
    task.await;
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
mod shutdown_tests;
#[cfg(test)]
mod tests;
