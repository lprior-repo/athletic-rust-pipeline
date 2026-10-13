use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::Arc;

use tokio::sync::oneshot;

use super::{Counters, Spawner};
use crate::outcome::Outcome;

enum Completion<T, E> {
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
        if !self.push_blocking(run, permit) {
            return Outcome::Cancelled;
        }
        match rx.await {
            Ok(Completion::Returned(Ok(value))) => Outcome::Ok(value),
            Ok(Completion::Returned(Err(error))) => Outcome::Err(error),
            Ok(Completion::Panicked) => Outcome::Panicked,
            Err(_) => Outcome::Cancelled,
        }
    }
}

fn publish<T, E>(tx: oneshot::Sender<Completion<T, E>>, completion: Completion<T, E>) {
    if tx.send(completion).is_err() {
        tracing::debug!("a region job finished with no caller waiting for its value");
    }
}
