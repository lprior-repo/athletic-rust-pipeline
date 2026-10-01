use std::sync::Arc;

use restate_sdk::prelude::*;

use crate::outcome::Outcome;
use crate::spawn::Spawner;
use census_report::report::ReportError;
use census_store::StoreError;
#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("{message}")]
    Transient { message: String },
    #[error("{message}")]
    Terminal { message: String },
}

impl From<StoreError> for JobError {
    fn from(error: StoreError) -> Self {
        let message = error.to_string();
        match error {
            StoreError::Open { .. }
            | StoreError::Flush { .. }
            | StoreError::Read { .. }
            | StoreError::Write { .. }
            | StoreError::Io { .. } => Self::Transient { message },
            StoreError::Decode { .. }
            | StoreError::Json { .. }
            | StoreError::SnapshotRow { .. }
            | StoreError::ObservationReplacement { .. }
            | StoreError::TooManyRows { .. }
            | StoreError::JournalTooLarge { .. }
            | StoreError::CounterOverflow
            | StoreError::Refused { .. }
            | StoreError::Identity(_)
            | StoreError::Invariant { .. } => Self::Terminal { message },
        }
    }
}
impl From<ReportError> for JobError {
    fn from(error: ReportError) -> Self {
        let message = error.to_string();
        match error {
            ReportError::Store(source) => Self::from(source),
            ReportError::Invariant { .. } | ReportError::Cleanup { .. } => {
                Self::Terminal { message }
            }
            _ => Self::Transient { message },
        }
    }
}

pub fn job_error(error: JobError) -> HandlerError {
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

#[tracing::instrument(skip_all)]
pub async fn blocking<T, E, F>(spawner: Arc<Spawner>, job: F) -> Result<T, JobError>
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
    }
}
