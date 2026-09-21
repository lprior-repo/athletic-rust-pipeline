//! Drain accounting for the owned task region: the report it returns and the bounded reap.

use std::time::Duration;

use tokio::task::JoinSet;

use crate::outcome::DrainState;

use super::error::BootstrapError;
use super::stop::StopReason;

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

fn bump(counter: &mut u64, by: u64) -> u64 {
    *counter = counter.saturating_add(by);
    *counter
}

/// Bounded drain of an owned task region: reap natural exits, then abort and count the rest.

pub(super) async fn drain(
    mut tasks: JoinSet<()>,
    timeout: Duration,
) -> Result<DrainReport, BootstrapError> {
    let mut report = DrainReport::default();
    let accepted = u64::try_from(tasks.len()).map_err(|_| BootstrapError::TaskCountOverflow)?;
    bump(&mut report.accepted, accepted);
    let deadline = tokio::time::Instant::now() + timeout;

    while !tasks.is_empty() {
        match tokio::time::timeout_at(deadline, tasks.join_next()).await {
            Ok(Some(result)) => match DrainState::from_join(result) {
                DrainState::Completed => {
                    bump(&mut report.completed, 1);
                }
                DrainState::Panicked => {
                    tracing::error!("task panicked during shutdown");
                    bump(&mut report.panicked, 1);
                }
                DrainState::Cancelled => {
                    bump(&mut report.cancelled, 1);
                }
            },
            Ok(None) => break,
            Err(_) => {
                tracing::warn!(?timeout, "drain deadline reached; aborting remaining tasks");
                report.remaining =
                    u64::try_from(tasks.len()).map_err(|_| BootstrapError::TaskCountOverflow)?;
                bump(&mut report.timed_out, report.remaining);
                tasks.abort_all();
                // Reap the aborted tasks so their resources (sockets, file handles) are released
                // before this returns.
                while tasks.join_next().await.is_some() {
                    bump(&mut report.aborted, 1);
                }
                break;
            }
        }
    }
    Ok(report)
}
