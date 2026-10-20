use super::blocking::{publish, Completion};
use super::{Counters, Ledger, SpawnError, Spawner};
use crate::outcome::Outcome;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::Arc;
use tokio::sync::oneshot;

struct OwnedThread {
    handle: std::thread::JoinHandle<()>,
    counters: Arc<Counters>,
}

#[derive(Default)]
pub(super) struct DedicatedThreads(Vec<OwnedThread>);

impl DedicatedThreads {
    pub(super) fn len(&self) -> usize {
        self.0.len()
    }
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn reserve(&mut self) -> Result<(), SpawnError> {
        self.0
            .try_reserve(1)
            .map_err(|_| SpawnError::ThreadAllocation)
    }

    pub(super) fn reap_finished(&mut self, ledger: &mut Ledger) {
        self.0
            .extract_if(.., |thread| thread.handle.is_finished())
            .for_each(|thread| {
                ledger.classify_thread(thread.handle.join(), &thread.counters);
            });
    }
}

impl Spawner {
    pub(crate) fn reserve_browser(&self) -> Result<super::Reservation, SpawnError> {
        match self
            .coordinator
            .try_reserve_profile(crate::admission::WORK_CLASS_BROWSER)
        {
            super::TryResult::Granted(reservation) => Ok(reservation),
            super::TryResult::Deferred(reason) => Err(SpawnError::AdmissionDeferred(reason)),
            super::TryResult::Refused(reason) => Err(SpawnError::Admission(reason)),
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn dedicated<T, E, F, G>(&self, owner: G, job: F) -> Result<Outcome<T, E>, SpawnError>
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
        G: Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
    {
        if self.lock().closed {
            return Ok(Outcome::Cancelled);
        }
        let Some(permit) = self.acquire_blocking().await else {
            return Ok(Outcome::Cancelled);
        };
        let (sent, received) = oneshot::channel();
        let counters = Arc::new(Counters::default());
        let witness = Arc::clone(&counters);
        let span = tracing::Span::current();
        let dispatcher = tracing::dispatcher::get_default(Clone::clone);
        let run = move || {
            let _dispatch = tracing::dispatcher::set_default(&dispatcher);
            let _span = span.enter();
            let _permit = permit;
            let _owner = owner;
            counters.record_started();
            match catch_unwind(AssertUnwindSafe(job)) {
                Ok(Ok(value)) => publish(sent, Completion::Returned(Ok(value))),
                Ok(Err(error)) => {
                    counters.record_failed();
                    publish(sent, Completion::Returned(Err(error)));
                }
                Err(payload) => {
                    publish(sent, Completion::Panicked);
                    resume_unwind(payload);
                }
            }
        };
        match self.push_dedicated(run, witness).await {
            Ok(()) => {}
            Err(SpawnError::RegionClosed) => return Ok(Outcome::Cancelled),
            Err(error) => return Err(error),
        }
        Ok(match received.await {
            Ok(Completion::Returned(Ok(value))) => Outcome::Ok(value),
            Ok(Completion::Returned(Err(error))) => Outcome::Err(error),
            Ok(Completion::Panicked) => Outcome::Panicked,
            Err(_) => Outcome::Cancelled,
        })
    }

    async fn push_dedicated<F>(&self, run: F, counters: Arc<Counters>) -> Result<(), SpawnError>
    where
        F: FnOnce() + Send + 'static,
    {
        self.push_dedicated_with(run, counters, |run| {
            std::thread::Builder::new()
                .name("census-owned-work".into())
                .spawn(run)
        })
        .await
    }

    async fn push_dedicated_with<F, S>(
        &self,
        run: F,
        counters: Arc<Counters>,
        start: S,
    ) -> Result<(), SpawnError>
    where
        F: FnOnce() + Send + 'static,
        S: FnOnce(F) -> std::io::Result<std::thread::JoinHandle<()>>,
    {
        loop {
            {
                let mut region = self.lock();
                if region.closed {
                    return Err(SpawnError::RegionClosed);
                }
                region.reap_finished();
                if region.worker_has_room(self.capacity)? {
                    region.threads.reserve()?;
                    let handle = start(run).map_err(|source| SpawnError::ThreadSpawn { source })?;
                    region.threads.0.push(OwnedThread { handle, counters });
                    region.ledger.accept();
                    return Ok(());
                }
            }
            self.wait_worker_reap().await;
        }
    }
}

#[cfg(test)]
mod tests;
