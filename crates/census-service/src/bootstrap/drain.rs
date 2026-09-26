//! Drain accounting for the owned task region: the report the shell returns and the bounded reap.
//!
//! The region itself — one [`Spawner`](crate::spawn::Spawner) over one `JoinSet` — lives in
//! [`crate::spawn`]. This module is the shell's view of it: the report an operator reads and the
//! integration test asserts on, plus — for tests that build a set by hand — the drain entry point
//! that takes one.

use crate::spawn::{SpawnError, TaskReport};

use super::error::BootstrapError;
use super::stop::StopReason;

#[cfg(test)]
use crate::spawn::Spawner;
#[cfg(test)]
use std::time::Duration;
#[cfg(test)]
use tokio::task::JoinSet;

/// What the region did before it stopped.
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
}

impl DrainReport {
    /// The shell's report from the region's count.
    ///
    /// The seven task counters are the region's, verbatim. The stop reason stays the shell's,
    /// because the shell is what saw how the stop arrived: it defaults to
    /// [`StopReason::ServerExit`] and the supervisor overwrites it with what it observed.
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

/// A region count that does not fit a report field is the shell's own overflow: the refusal is
/// reported as a failure rather than as a smaller, quieter number.
pub(super) fn count_error(error: SpawnError) -> BootstrapError {
    match error {
        SpawnError::TaskCountOverflow => BootstrapError::TaskCountOverflow,
    }
}

/// Bounded drain of an owned task region: reap natural exits, then abort and count the rest.
///
/// The supervisor's own drain is [`Spawner::drain`] on the region it owns. This entry point serves a
/// caller that already holds a set — the supervisor's drain tests build one directly — and it keeps
/// their report meaning exactly: the tasks handed in, counted by the one implementation in
/// [`crate::spawn`].
#[cfg(test)]
pub(super) async fn drain(
    tasks: JoinSet<()>,
    timeout: Duration,
) -> Result<DrainReport, BootstrapError> {
    let region = Spawner::adopting(tasks).map_err(count_error)?;
    let counted = region.drain(timeout).await.map_err(count_error)?;
    Ok(DrainReport::from_counted(counted))
}
