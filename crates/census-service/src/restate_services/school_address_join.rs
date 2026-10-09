use std::path::{Path, PathBuf};
use std::sync::Arc;

use restate_sdk::prelude::*;

use crate::school_address::{join_generation_pinned, preflight_generation, JoinError, Overrides};
use census_store::Store;

use super::publish::Jobs;
use super::wire::{SchoolAddressJoinReply, SchoolAddressJoinRequest};
use super::{blocking, job_error, JobError};

const DEFAULT_DIRECTORY: &str = "school-address";

#[derive(Clone)]
pub struct SchoolAddressJoin {
    jobs: Jobs,
}

impl SchoolAddressJoin {
    pub fn new(jobs: Jobs) -> Self {
        Self { jobs }
    }

    pub(super) fn generation_path(root: &Path, requested: Option<&str>) -> PathBuf {
        match requested {
            Some(value) => PathBuf::from(value),
            None => root.join(DEFAULT_DIRECTORY),
        }
    }
}

impl From<JoinError> for JobError {
    fn from(error: JoinError) -> Self {
        let message = error.to_string();
        match error {
            JoinError::Store(source) => Self::from(source),
            JoinError::Io { .. } => Self::Transient { message },
            JoinError::Generation(_)
            | JoinError::Artifact { .. }
            | JoinError::EvidenceSource { .. }
            | JoinError::EvidencePair { .. }
            | JoinError::EvidenceValue { .. }
            | JoinError::Invariant { .. } => Self::Terminal { message },
        }
    }
}

#[workflow(
    journal_retention = "90 days",
    workflow_completion_retention = "180 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(
        initial_interval = "500ms",
        max_interval = "1m",
        max_attempts = 3,
        on_max_attempts = "pause"
    )
)]
impl SchoolAddressJoin {
    #[handler]
    #[tracing::instrument(skip_all, fields(generation = ?request.generation))]
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        Json(request): Json<SchoolAddressJoinRequest>,
    ) -> Result<Json<SchoolAddressJoinReply>, HandlerError> {
        let store: Arc<Store> = Arc::clone(self.jobs.store());
        let region = Arc::clone(self.jobs.region());
        let generation = Self::generation_path(store.root(), request.generation.as_deref());
        let overrides = Overrides {
            urls: request.urls,
            dates: request.dates,
        };
        let permit = self.jobs.permit().await?;
        let reply = ctx
            .run(move || async move {
                let _permit = permit;
                blocking(region, move || {
                    let expected = match request.expected_digest {
                        Some(digest) => digest,
                        None => preflight_generation(&generation, overrides.clone())?,
                    };
                    join_generation_pinned(&store, &generation, overrides, &expected)
                })
                .await
                .map(|report| Json(SchoolAddressJoinReply::from(report)))
                .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(reply)
    }
}
