use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::Arc;

use tokio::sync::oneshot;

use super::{Counters, Spawner};
use crate::outcome::Outcome;

pub(super) enum Completion<T, E> {
    Returned(Result<T, E>),
    Panicked,
}

impl Spawner {
    #[tracing::instrument(skip_all)]
    pub async fn blocking<T, E, F>(&self, job: F) -> Outcome<T, E>
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
    {
        {
            let region = self.lock();
            if region.closed {
                tracing::error!(
                    capacity = self.capacity,
                    "the task region is closed; the blocking job was not run"
                );
                return Outcome::Cancelled;
            }
        }
        let Some(permit) = self.acquire_blocking().await else {
            tracing::error!(
                capacity = self.capacity,
                "the blocking lane is closed; the blocking job was not run"
            );
            return Outcome::Cancelled;
        };
        let (tx, rx) = oneshot::channel();
        let run = move |counters: Arc<Counters>| {
            counters.record_started();
            match catch_unwind(AssertUnwindSafe(job)) {
                Ok(Ok(value)) => publish(tx, Completion::Returned(Ok(value))),
                Ok(Err(error)) => {
                    counters.record_failed();
                    publish(tx, Completion::Returned(Err(error)));
                }
                Err(payload) => {
                    publish(tx, Completion::Panicked);
                    resume_unwind(payload);
                }
            }
        };
        if !self.push_blocking(run, permit).await {
            return Outcome::Cancelled;
        }
        match rx.await {
            Ok(Completion::Returned(Ok(value))) => Outcome::Ok(value),
            Ok(Completion::Returned(Err(error))) => Outcome::Err(error),
            Ok(Completion::Panicked) => Outcome::Panicked,
            Err(_) => Outcome::Cancelled,
        }
    }

    pub(super) async fn push_blocking<F>(
        &self,
        job: F,
        permit: tokio::sync::OwnedSemaphorePermit,
    ) -> bool
    where
        F: FnOnce(Arc<Counters>) + Send + 'static,
    {
        loop {
            {
                let mut region = self.lock();
                if region.closed {
                    return false;
                }
                region.reap_finished();
                match region.worker_has_room(self.capacity) {
                    Ok(true) => {
                        let counters = Arc::new(Counters::default());
                        let witness = Arc::clone(&counters);
                        let task = region.tasks.spawn_blocking(move || {
                            let _permit = permit;
                            job(counters);
                        });
                        region.ledger.track_blocking(task.id(), witness);
                        region.ledger.accept();
                        return true;
                    }
                    Ok(false) => {}
                    Err(error) => {
                        tracing::error!(%error, "worker descriptor accounting refused an unstarted effect");
                        return false;
                    }
                }
            }
            self.wait_worker_reap().await;
        }
    }

    pub(super) async fn wait_worker_reap(&self) {
        let mut stopping = self.stopping();
        if *stopping.borrow() {
            return;
        }
        tokio::select! {
            _ = stopping.changed() => {},
            () = tokio::time::sleep(std::time::Duration::from_millis(10)) => {},
        }
    }
}

pub(super) fn publish<T, E>(tx: oneshot::Sender<Completion<T, E>>, completion: Completion<T, E>) {
    if tx.send(completion).is_err() {
        tracing::debug!("a region job finished with no caller waiting for its value");
    }
}
