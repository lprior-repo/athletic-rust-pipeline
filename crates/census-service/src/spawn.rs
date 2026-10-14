use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::sync::{watch, Semaphore};
use tokio::task::JoinSet;

pub use crate::admission::{
    Charged, Reservation, WORK_CLASS_CONTROL, WORK_CLASS_INGEST, WORK_CLASS_STAGE,
};
use crate::admission::{Coordinator, TryResult, WorkClass, WORK_CLASS_NORMAL};
#[cfg(test)]
use crate::outcome::Outcome;
#[cfg(test)]
use tokio::sync::oneshot;

mod blocking;
mod dedicated;
mod drain;
mod region;
mod types;
use region::Region;
pub use types::{SpawnError, TaskReport};
mod ledger;
use ledger::{Counters, Ledger};

pub const DEFAULT_CAPACITY: usize = 500;

pub struct Spawner {
    capacity: usize,
    coordinator: Coordinator,
    blocking: Arc<Semaphore>,
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
            blocking: Arc::new(Semaphore::new(permits)),
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
            blocking: Arc::new(Semaphore::new(capacity)),
            region: Mutex::new(Region {
                tasks,
                threads: dedicated::DedicatedThreads::default(),
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

    pub(crate) async fn acquire_blocking(&self) -> Option<tokio::sync::OwnedSemaphorePermit> {
        Arc::clone(&self.blocking).acquire_owned().await.ok()
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
        let permit = self.try_acquire_blocking()?;
        self.push_async(task, slot, Some(permit))
    }

    fn try_acquire_blocking(&self) -> Result<tokio::sync::OwnedSemaphorePermit, SpawnError> {
        match Arc::clone(&self.blocking).try_acquire_owned() {
            Ok(permit) => Ok(permit),
            Err(_) if self.lock().closed => Err(SpawnError::RegionClosed),
            Err(_) => Err(SpawnError::RegionFull {
                capacity: self.capacity,
            }),
        }
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
        self.push_async_with(build, reservation, None)
    }

    fn push_async<F>(
        &self,
        task: F,
        slot: Reservation,
        permit: Option<tokio::sync::OwnedSemaphorePermit>,
    ) -> Result<(), SpawnError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.push_async_with(|| task, slot, permit)
    }

    fn push_async_with<F, B>(
        &self,
        build: B,
        slot: Reservation,
        permit: Option<tokio::sync::OwnedSemaphorePermit>,
    ) -> Result<(), SpawnError>
    where
        F: Future<Output = ()> + Send + 'static,
        B: FnOnce() -> F,
    {
        let mut region = self.lock();
        if region.closed {
            return Err(SpawnError::RegionClosed);
        }
        region.reap_finished();
        region.require_async_room(self.capacity)?;
        region.tasks.spawn(supervised(slot, build(), permit));
        region.ledger.accept();
        Ok(())
    }

    fn lock(&self) -> MutexGuard<'_, Region> {
        match self.region.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

async fn supervised<F>(
    slot: Reservation,
    task: F,
    permit: Option<tokio::sync::OwnedSemaphorePermit>,
) where
    F: Future<Output = ()>,
{
    let _slot = slot;
    let _permit = permit;
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
#[cfg(test)]
mod descriptor_tests;
#[cfg(all(feature = "loom", test))]
mod loom_tests;
#[cfg(test)]
mod shutdown_tests;
#[cfg(test)]
mod tests;
