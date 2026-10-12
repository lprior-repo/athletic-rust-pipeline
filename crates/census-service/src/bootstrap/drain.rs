use std::time::Duration;

use census_store::Store;

use crate::outcome::AttemptState;
use crate::restate_services::jurisdiction::team_source::durable_attempts;
use crate::spawn::{SpawnError, TaskReport};

use super::error::BootstrapError;
use super::stop::StopReason;

#[cfg(test)]
use crate::spawn::Spawner;

#[cfg(test)]
use tokio::task::JoinSet;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EndpointShutdown {
    #[default]
    Completed,
    TimedOut,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DrainReport {
    pub accepted: u64,
    pub completed: u64,
    pub cancelled: u64,
    pub aborted: u64,
    pub panicked: u64,
    pub failed: u64,
    pub still_running: u64,
    pub timed_out: u64,
    pub remaining: u64,
    pub attempts: AttemptState,
    pub stop_reason: StopReason,
    pub endpoint_shutdown: EndpointShutdown,
}

impl DrainReport {
    pub(super) fn from_counted(counted: TaskReport) -> Self {
        Self {
            accepted: counted.accepted,
            completed: counted.completed,
            cancelled: counted.cancelled,
            aborted: counted.aborted,
            panicked: counted.panicked,
            failed: counted.failed,
            still_running: counted.still_running,
            timed_out: counted.timed_out,
            remaining: counted.remaining,
            ..Self::default()
        }
    }

    pub fn disjoint_total(&self) -> u64 {
        self.completed
            .saturating_add(self.cancelled)
            .saturating_add(self.aborted)
            .saturating_add(self.panicked)
            .saturating_add(self.still_running)
    }
}

pub(super) fn count_error(error: SpawnError) -> BootstrapError {
    BootstrapError::TaskSupervision {
        source: Box::new(error),
    }
}

pub(super) fn attempt_state(store: &Store) -> Result<AttemptState, BootstrapError> {
    durable_attempts(store).map_err(|source| BootstrapError::SourceAttempts {
        source: Box::new(source),
    })
}

#[cfg(test)]
pub(super) async fn drain(
    tasks: JoinSet<()>,
    timeout: Duration,
) -> Result<DrainReport, BootstrapError> {
    let region = Spawner::adopting(tasks).map_err(count_error)?;
    let counted = region.drain(timeout).await.map_err(count_error)?;
    Ok(DrainReport::from_counted(counted))
}

pub(super) const ENDPOINT_SHUTDOWN_GRACE: Duration = Duration::from_secs(10);

pub(super) async fn await_endpoint_shutdown(
    endpoint_done: tokio::sync::oneshot::Receiver<()>,
    grace: Duration,
) -> EndpointShutdown {
    match tokio::time::timeout(grace, endpoint_done).await {
        Ok(Ok(())) | Ok(Err(_)) => EndpointShutdown::Completed,
        Err(_) => {
            tracing::warn!(
                grace_seconds = grace.as_secs_f64(),
                "the endpoint did not finish within the shutdown grace; closing effect admission"
            );
            EndpointShutdown::TimedOut
        }
    }
}
