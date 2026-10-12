use super::{blocking, job_error, Credit, Ingest};
use crate::admission::{Charged, WORK_CLASS_STAGE};
use crate::restate_services::{
    jobs::collect_error,
    wire::{IngestReply, RecordedIngestRequest},
};
use census_store::Application;
use restate_sdk::prelude::*;
use std::sync::Arc;

impl Ingest {
    #[tracing::instrument(skip_all)]
    pub(super) async fn recording(
        &self,
        ctx: &ObjectContext<'_>,
        request: RecordedIngestRequest,
    ) -> Result<Json<IngestReply>, HandlerError> {
        let mut state = self.load_object(ctx).await?;
        let prepared = self
            .prepare_recording(
                request.spool_identity,
                request.fragment,
                request.spool,
                request.operation_id,
            )
            .await?;
        admit(prepared.operation(), request.cursor.as_deref())?;
        let today = super::super::journaled_today(ctx, &self.clock).await?;
        let application = self.apply_recording(ctx, prepared).await?;
        let receipt = application.receipt();
        let credit = self
            .credit(ctx, state.endpoint.clone(), receipt.operation.clone())
            .await?;
        let written = application.appended();
        Self::update_ingest_state(
            &mut state,
            Credit {
                total: credit.total,
                cursor: request.cursor,
                today,
            },
            ctx,
        )?;
        Ok(Json(IngestReply {
            endpoint: state.endpoint,
            appended: credit.credited,
            written,
            total_observations: state.total_observations,
            cursor: state.cursor,
            last_appended_at: state.last_appended_at,
        }))
    }

    #[tracing::instrument(skip_all)]
    async fn apply_recording(
        &self,
        ctx: &ObjectContext<'_>,
        prepared: Charged<census_crawl::PreparedRecorded>,
    ) -> Result<Application, HandlerError> {
        let store = Arc::clone(&self.store);
        let region = Arc::clone(&self.region);
        let Json(application) = ctx
            .run(move || async move {
                blocking(region, move || {
                    let (prepared, _permit) = prepared.into_parts();
                    prepared.apply(&store).map_err(collect_error)
                })
                .await
                .map(Json)
                .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(application)
    }

    #[tracing::instrument(skip_all)]
    async fn prepare_recording(
        &self,
        identity: String,
        index: usize,
        spool: census_crawl::recording::spool::FragmentSummary,
        operation: String,
    ) -> Result<Charged<census_crawl::PreparedRecorded>, HandlerError> {
        admit_reference(&identity, index, &spool, &operation)?;
        let store = Arc::clone(&self.store);
        let mut profile = WORK_CLASS_STAGE;
        profile.resources.work = profile
            .resources
            .work
            .checked_add(
                u64::try_from(spool.fragments)
                    .map_err(|_| TerminalError::new("ingest work exceeds u64"))?,
            )
            .ok_or_else(|| {
                TerminalError::new("ingest work accounting overflow; acquisition remains owed")
            })?;
        let permit = self
            .region
            .coordinator()
            .reserve(profile)
            .await
            .map_err(|error| TerminalError::new(format!("ingest admission: {error}")))?;
        blocking(Arc::clone(&self.region), move || {
            census_crawl::recording::spool::verify_reference(&store, &identity, &spool)
                .map_err(collect_error)?;
            let recorded = census_crawl::recording::spool::read_fragment(&store, &identity, index)
                .map_err(collect_error)?;
            let manifest = super::super::ingest_post::admit_recorded(&recorded)
                .map_err(crate::restate_services::JobError::from)?;
            manifest.iter().try_for_each(|entry| {
                let batch = recorded.rows.get(entry.batch).ok_or_else(|| {
                    crate::restate_services::JobError::from(census_store::StoreError::Invariant {
                        detail: "recorded manifest batch is missing".to_string(),
                    })
                })?;
                let rows = batch.rows.get(entry.chunk.rows.clone()).ok_or_else(|| {
                    crate::restate_services::JobError::from(census_store::StoreError::Invariant {
                        detail: "recorded manifest rows are missing".to_string(),
                    })
                })?;
                super::validate_rows(batch.table, rows)
            })?;
            let prepared = recorded.prepare(&operation).map_err(collect_error)?;
            Ok::<_, crate::restate_services::JobError>(permit.into_charged(prepared))
        })
        .await
        .map_err(job_error)
    }
}

pub(super) fn admit(operation: &str, cursor: Option<&str>) -> Result<(), HandlerError> {
    if operation.trim().is_empty() {
        return Err(TerminalError::new("ingest operation must not be empty").into());
    }
    super::check_identifier("operation id", operation, super::MAX_OPERATION_ID_BYTES)?;
    if cursor.is_some_and(|value| value.len() > 4096) {
        return Err(TerminalError::new(
            "ingest cursor exceeds 4096 bytes; acquisition remains owed",
        )
        .into());
    }
    Ok(())
}

fn admit_reference(
    identity: &str,
    index: usize,
    spool: &census_crawl::recording::spool::FragmentSummary,
    operation: &str,
) -> Result<(), HandlerError> {
    admit(operation, None)?;
    if identity.is_empty()
        || identity.len() > super::MAX_OPERATION_ID_BYTES
        || spool.fragments > census_crawl::recording::MAX_RECORDED_WORK
        || index >= spool.fragments
        || spool.digest.len() != 64
        || !spool.digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        || operation != census_crawl::recording::spool::fragment_key(identity, index)
    {
        return Err(TerminalError::new(
            "invalid durable ingest reference; acquisition remains owed",
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod admission_tests;
