use super::{narrow, Ledger, Region, SpawnError, Spawner, TaskReport};
use census_store::clock::{Clock, SystemClock};
use std::time::Duration;

const THREAD_REAP_INTERVAL: Duration = Duration::from_millis(10);

struct Draining<'a> {
    owner: &'a Spawner,
    region: Region,
}

impl<'a> Draining<'a> {
    fn new(owner: &'a Spawner) -> Self {
        let mut region = owner.lock();
        region.closed = true;
        owner.coordinator.close();
        owner.blocking.close();
        owner.stopping.send_replace(true);
        Self {
            owner,
            region: std::mem::replace(
                &mut *region,
                Region {
                    closed: true,
                    ..Region::default()
                },
            ),
        }
    }
}

impl Drop for Draining<'_> {
    fn drop(&mut self) {
        let remaining = self.region.remaining_hint();
        self.region.ledger.set_remaining(remaining);
        *self.owner.lock() = std::mem::take(&mut self.region);
    }
}

impl Spawner {
    #[tracing::instrument(skip_all, fields(timeout = ?timeout))]
    pub async fn drain(&self, timeout: Duration) -> Result<TaskReport, SpawnError> {
        let _exclusive = self.draining.lock().await;
        let mut owned = Draining::new(self);
        let region = &mut owned.region;
        let deadline = SystemClock.now().checked_add(timeout);
        if region.aborting {
            abort_and_reap(region).await?;
        }
        drain_until(region, deadline).await?;
        region.ledger.set_remaining(narrow(region.pending()?)?);
        drop(region.adopted.take());
        let report = region.ledger.report();
        region.ledger = Ledger::default();
        region.aborting = false;
        Ok(report)
    }
}

async fn drain_until(
    region: &mut Region,
    deadline: Option<tokio::time::Instant>,
) -> Result<(), SpawnError> {
    region.reap_finished();
    while !region.is_empty() {
        match deadline {
            Some(deadline) => {
                if tokio::time::timeout_at(deadline, wait_for_progress(region))
                    .await
                    .is_err()
                {
                    return abort_and_reap(region).await;
                }
            }
            None => wait_for_progress(region).await,
        }
        region.ledger.set_remaining(narrow(region.pending()?)?);
    }
    Ok(())
}

async fn wait_for_progress(region: &mut Region) {
    tokio::select! {
        joined = region.tasks.join_next_with_id(), if !region.tasks.is_empty() => {
            if let Some(joined) = joined { region.ledger.classify_join(joined); }
        }
        () = tokio::time::sleep(THREAD_REAP_INTERVAL), if !region.threads.is_empty() => {}
    }
    region.reap_finished();
}

async fn abort_and_reap(region: &mut Region) -> Result<(), SpawnError> {
    region.reap_finished();
    if !region.aborting {
        let remaining = narrow(region.pending()?)?;
        tracing::warn!(
            remaining,
            "drain grace deadline reached; aborting async work and reaping owned effects"
        );
        region.ledger.note_deadline(remaining);
        region.aborting = true;
        region.tasks.abort_all();
    }
    while !region.is_empty() {
        wait_for_progress(region).await;
        region.ledger.set_remaining(narrow(region.pending()?)?);
    }
    Ok(())
}
