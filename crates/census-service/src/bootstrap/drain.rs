use crate::spawn::{SpawnError, TaskReport};

use super::error::BootstrapError;
use super::stop::StopReason;

#[cfg(test)]
use crate::spawn::Spawner;
#[cfg(test)]
use std::time::Duration;
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
    pub timed_out: u64,
    pub remaining: u64,
    pub aborted: u64,
    pub panicked: u64,
    pub stop_reason: StopReason,
    pub endpoint_shutdown: EndpointShutdown,
}

impl DrainReport {
    pub(super) fn from_counted(counted: TaskReport) -> Self {
        Self {
            accepted: counted.accepted,
            completed: counted.completed,
            cancelled: counted.cancelled,
            timed_out: counted.timed_out,
            remaining: counted.remaining,
            aborted: counted.aborted,
            panicked: counted.panicked,
            ..Self::default()
        }
    }
}

pub(super) fn count_error(error: SpawnError) -> BootstrapError {
    BootstrapError::TaskSupervision { source: error }
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
