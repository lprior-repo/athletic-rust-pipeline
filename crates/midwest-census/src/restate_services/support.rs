//! The job layer every handler shares: how a store or report failure is classified for retry, and
//! how the blocking pool runs the work.
//!
//! Nothing here touches the store or the journal. A handler converts its job's error into
//! [`JobError`] and hands the function to [`blocking`]; the split between retryable and terminal is
//! made in one place so no handler can decide it differently.
use std::sync::Arc;

use restate_sdk::prelude::*;

use crate::outcome::Outcome;
use crate::report::ReportError;
use crate::spawn::Spawner;
use crate::store::StoreError;
/// A job outcome Restate can act on: retrying a transient failure is worth it, retrying a terminal
/// one is not.
#[derive(Debug, thiserror::Error)]
pub enum JobError {
    /// Retry with backoff — the input may still be there next time.
    #[error("{message}")]
    Transient { message: String },
    /// Do not retry — the request itself is wrong or the job panicked.
    #[error("{message}")]
    Terminal { message: String },
}

/// Store work that failed, classified for retry.
///
/// Transient is the default and the deliberate one: a lock, a full volume, an in-flight compaction —
/// each is exactly what a journaled retry repairs, and every job here is safe to repeat. An
/// `Invariant` violation is the exception: the store's own writer maintains those, so replaying the
/// same journal value cannot restore one.
impl From<StoreError> for JobError {
    fn from(error: StoreError) -> Self {
        let message = error.to_string();
        match error {
            StoreError::Invariant { .. } => Self::Terminal { message },
            _ => Self::Transient { message },
        }
    }
}

/// Report, bests and workbook work that failed: the same rule one layer up. A store failure
/// delegates so its own classification survives, and a violated invariant is terminal here for the
/// same reason it is in the store: the report's invariants are its own, so a replay cannot restore
/// one.
impl From<ReportError> for JobError {
    fn from(error: ReportError) -> Self {
        let message = error.to_string();
        match error {
            ReportError::Store(source) => Self::from(source),
            ReportError::Invariant { .. } => Self::Terminal { message },
            _ => Self::Transient { message },
        }
    }
}

/// Map a job outcome onto Restate's terminal/retryable split. Deliberately a plain function rather
/// than a `From` impl: `HandlerError` already has a blanket `From<E: StdError>`, and letting
/// `JobError` take that path would make every terminal failure silently retryable.
pub(super) fn job_error(error: JobError) -> HandlerError {
    match error {
        JobError::Transient { message } => HandlerError::from(TransientFailure { message }),
        JobError::Terminal { message } => HandlerError::from(TerminalError::new(message)),
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
struct TransientFailure {
    message: String,
}

/// Run a blocking job as a region task, classifying the outcome for retry.
///
/// `E` is whatever the job reports: the store's and the report's typed errors convert through the
/// `From` impls above, and a job that already knows its own outcome — an input bound it refused —
/// hands back a [`JobError`] unchanged. A panicked or cancelled task is always terminal: replaying
/// the journal value that panicked would panic again.
///
/// The job goes through the region the shell handed in, so an invocation that is aborted mid-await
/// leaves the work owned (and reaped) by the region rather than running unattached.
#[tracing::instrument(skip_all)]
pub(super) async fn blocking<T, E, F>(spawner: Arc<Spawner>, job: F) -> Result<T, JobError>
where
    F: FnOnce() -> Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: Into<JobError> + Send + 'static,
{
    match spawner.blocking(job).await {
        Outcome::Ok(value) => Ok(value),
        Outcome::Err(error) => Err(error.into()),
        Outcome::Panicked => Err(JobError::Terminal {
            message: "job panicked".to_string(),
        }),
        Outcome::Cancelled => Err(JobError::Terminal {
            message: "job cancelled".to_string(),
        }),
        // A region job cannot report a timeout — the abort path reports a cancellation — but the
        // lattice is total and a job that never returned is terminal either way.
        Outcome::Timeout => Err(JobError::Terminal {
            message: "job timed out".to_string(),
        }),
    }
}
