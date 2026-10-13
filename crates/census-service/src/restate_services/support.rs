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

impl JobError {
    fn from_store_error(error: StoreError, message: String) -> Self {
        if is_transient_store_error(&error) {
            Self::Transient { message }
        } else {
            Self::Terminal { message }
        }
    }
}

fn is_transient_store_error(error: &StoreError) -> bool {
    matches!(
        error,
        StoreError::RecordRevision { .. }
            | StoreError::RecordMissing { .. }
            | StoreError::LeaseMismatch { .. }
            | StoreError::LeaseExpired { .. }
            | StoreError::LeaseLost { .. }
            | StoreError::CurrentRootChanged { .. }
            | StoreError::PublicationDurabilityUncertain { .. }
            | StoreError::JournalNotReady { .. }
            | StoreError::Open { .. }
            | StoreError::Flush { .. }
            | StoreError::Read { .. }
            | StoreError::Write { .. }
            | StoreError::Io { .. }
            | StoreError::PublicationRefused { .. }
            | StoreError::EvidenceMoved { .. }
            | StoreError::RevisionMoved { .. }
    )
}

impl From<StoreError> for JobError {
    fn from(error: StoreError) -> Self {
        let message = error.to_string();
        Self::from_store_error(error, message)
    }
}
impl From<ReportError> for JobError {
    fn from(error: ReportError) -> Self {
        let message = error.to_string();
        match error {
            ReportError::Store(source) => Self::from(source),
            ReportError::Catalog { source, .. } => match source {
                census_domain::model::CatalogError::LabelCountExceeded
                | census_domain::model::CatalogError::LabelBytesExceeded
                | census_domain::model::CatalogError::AggregateLabelBytesExceeded
                | census_domain::model::CatalogError::AggregateLabelBytesOverflow
                | census_domain::model::CatalogError::InvalidLabel => Self::Terminal { message },
                census_domain::model::CatalogError::ResourceExhausted => {
                    Self::Transient { message }
                }
            },
            ReportError::Invariant { .. } | ReportError::Cleanup { .. } => {
                Self::Terminal { message }
            }
            _ => Self::Transient { message },
        }
    }
}

#[cfg(test)]
mod catalog_tests;

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
