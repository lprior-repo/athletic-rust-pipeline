use std::time::Duration;

use census_store::clock::{Clock, SystemClock};

use super::{narrow, DrainState, Ledger, Region, SpawnError, Spawner, TaskReport};

struct Draining<'a> {
    owner: &'a Spawner,
    region: Region,
}

impl Drop for Draining<'_> {
    fn drop(&mut self) {
        *self.owner.lock() = std::mem::take(&mut self.region);
    }
}

impl Spawner {
    #[tracing::instrument(skip_all, fields(timeout = ?timeout))]
    pub async fn drain(&self, timeout: Duration) -> Result<TaskReport, SpawnError> {
        let _exclusive = self.draining.lock().await;
        let mut owned = {
            let mut region = self.lock();
            self.permits.close();
            self.stopping.send_replace(true);
            Draining {
                owner: self,
                region: std::mem::take(&mut *region),
            }
        };
        let region = &mut owned.region;
        let deadline = SystemClock.now().checked_add(timeout);
        if region.aborting {
            abort_and_reap(region).await?;
        }
        while !region.tasks.is_empty() {
            let joined = match deadline {
                Some(deadline) => {
                    match tokio::time::timeout_at(deadline, region.tasks.join_next()).await {
                        Ok(joined) => joined,
                        Err(_) => {
                            abort_and_reap(region).await?;
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
        let report = region.ledger.report();
        region.ledger = Ledger::default();
        region.aborting = false;
        Ok(report)
    }
}

async fn abort_and_reap(region: &mut Region) -> Result<(), SpawnError> {
    if !region.aborting {
        let remaining = narrow(region.tasks.len())?;
        tracing::warn!(
            remaining,
            "drain grace deadline reached; aborting async work and reaping blocking effects"
        );
        region.ledger.note_deadline(remaining);
        region.aborting = true;
        region.tasks.abort_all();
    }
    while let Some(joined) = region.tasks.join_next().await {
        region.ledger.classify_reaped(DrainState::from_join(joined));
        region.ledger.set_remaining(narrow(region.tasks.len())?);
    }
    Ok(())
}
