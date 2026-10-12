use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::sync::{watch, Semaphore};
use tokio::task::JoinSet;

use crate::admission::{
    Coordinator, DepartureReason, Reservation, TryResult, WorkClass, WORK_CLASS_NORMAL,
};
#[cfg(test)]
use crate::outcome::Outcome;
#[cfg(test)]
use tokio::sync::oneshot;

mod blocking;
mod drain;
mod ledger;
use ledger::{Counters, Ledger};

pub const DEFAULT_CAPACITY: usize = 500;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TaskReport {
    pub accepted: u64,
    pub completed: u64,
    pub cancelled: u64,
    pub aborted: u64,
    pub panicked: u64,
    pub failed: u64,
    pub still_running: u64,
    pub timed_out: u64,
    pub remaining: u64,
}

impl TaskReport {
    pub fn disjoint_total(&self) -> u64 {
        self.completed
            .saturating_add(self.cancelled)
            .saturating_add(self.aborted)
            .saturating_add(self.panicked)
            .saturating_add(self.still_running)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    #[error("task count does not fit u64")]
    TaskCountOverflow,
    #[error("the task region already supervises its {capacity} tasks")]
    RegionFull { capacity: usize },
    #[error("the task region's capacity is closed")]
    RegionClosed,
    #[error("admission refused: {0}")]
    Admission(DepartureReason),
}

impl From<DepartureReason> for SpawnError {
    fn from(reason: DepartureReason) -> Self {
        match reason {
            DepartureReason::RegionClosed => Self::RegionClosed,
            reason => Self::Admission(reason),
        }
    }
}

#[derive(Default)]
struct Region {
    tasks: JoinSet<()>,
    ledger: Ledger,
    closed: bool,
    aborting: bool,
    adopted: Option<Reservation>,
}

impl Region {
    fn reap_finished(&mut self) {
        while let Some(joined) = self.tasks.try_join_next_with_id() {
            self.ledger.classify_join(joined);
        }
    }
}

pub struct Spawner {
    capacity: usize,
    coordinator: Coordinator,
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
        let permits = capacity.clamp(1, Semaphore::MAX_PERMITS);
        Self {
            capacity: permits,
            coordinator: Coordinator::runtime(narrow_bound(permits)),
            region: Mutex::new(Region::default()),
            draining: tokio::sync::Mutex::new(()),
            stopping: watch::Sender::new(false),
        }
    }

    pub fn adopting(tasks: JoinSet<()>) -> Result<Self, SpawnError> {
        let held = narrow(tasks.len())?;
        let capacity = DEFAULT_CAPACITY.max(tasks.len());
        let coordinator = Coordinator::runtime(narrow(capacity)?);
        let adopted = if held == 0 {
            None
        } else {
            let mut profile = WORK_CLASS_NORMAL;
            profile.resources.bytes = profile
                .resources
                .bytes
                .checked_mul(held)
                .ok_or(SpawnError::TaskCountOverflow)?;
            profile.resources.work = held;
            profile.resources.slots.tasks = held;
            match coordinator.try_reserve_profile(profile) {
                TryResult::Granted(reservation) => Some(reservation),
                TryResult::Deferred(reason) | TryResult::Refused(reason) => {
                    return Err(reason.into())
                }
            }
        };
        Ok(Self {
            capacity,
            coordinator,
            region: Mutex::new(Region {
                tasks,
                ledger: Ledger::holding(held),
                closed: false,
                aborting: false,
                adopted,
            }),
            draining: tokio::sync::Mutex::new(()),
            stopping: watch::Sender::new(false),
        })
    }

    pub fn coordinator(&self) -> &Coordinator {
        &self.coordinator
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
        self.push_async(task, slot)
    }

    fn admit(&self) -> Result<Reservation, SpawnError> {
        match self.coordinator.try_reserve_profile(WORK_CLASS_NORMAL) {
            TryResult::Granted(reservation) => Ok(reservation),
            TryResult::Deferred(_) => Err(SpawnError::RegionFull {
                capacity: self.capacity,
            }),
            TryResult::Refused(reason) => Err(reason.into()),
        }
    }

    pub fn spawn_with<F, B>(&self, profile: WorkClass, build: B) -> Result<(), SpawnError>
    where
        F: Future<Output = ()> + Send + 'static,
        B: FnOnce() -> F,
    {
        let reservation = match self.coordinator.try_reserve_profile(profile) {
            TryResult::Granted(reservation) => reservation,
            TryResult::Deferred(_) => {
                return Err(SpawnError::RegionFull {
                    capacity: self.capacity,
                })
            }
            TryResult::Refused(reason) => return Err(reason.into()),
        };
        self.push_async(build(), reservation)
    }

    fn push_async<F>(&self, task: F, slot: Reservation) -> Result<(), SpawnError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let mut region = self.lock();
        if region.closed {
            return Err(SpawnError::RegionClosed);
        }
        region.reap_finished();
        region.tasks.spawn(supervised(slot, task));
        region.ledger.accept();
        Ok(())
    }

    fn push_blocking<F>(&self, job: F, slot: Reservation) -> bool
    where
        F: FnOnce(Arc<Counters>) + Send + 'static,
    {
        let mut region = self.lock();
        if region.closed {
            tracing::warn!("the task region is closed; the blocking job was not run");
            return false;
        }
        region.reap_finished();
        let counters = Arc::new(Counters::default());
        let witness = Arc::clone(&counters);
        let task = region.tasks.spawn_blocking(move || {
            let _slot = slot;
            job(counters);
        });
        region.ledger.track_blocking(task.id(), witness);
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

async fn supervised<F>(slot: Reservation, task: F)
where
    F: Future<Output = ()>,
{
    let _slot = slot;
    task.await;
}

fn narrow(count: usize) -> Result<u64, SpawnError> {
    u64::try_from(count).map_err(|_| SpawnError::TaskCountOverflow)
}

fn narrow_bound(permits: usize) -> u64 {
    u64::try_from(permits).map_or(u64::MAX, core::convert::identity)
}

#[cfg(test)]
mod accounting_tests;
#[cfg(test)]
mod admission_tests;
#[cfg(all(feature = "loom", test))]
mod loom_tests;
#[cfg(test)]
mod shutdown_tests;
#[cfg(test)]
mod tests;
