use super::{blocking, job_error, Credit, Ingest};
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
            .prepare_recording(request.recorded, request.operation_id)
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
        prepared: census_crawl::PreparedRecorded,
    ) -> Result<Application, HandlerError> {
        let store = Arc::clone(&self.store);
        let region = Arc::clone(&self.region);
        let Json(application) = ctx
            .run(move || async move {
                blocking(region, move || {
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
        recorded: census_crawl::Recorded,
        operation: String,
    ) -> Result<census_crawl::PreparedRecorded, HandlerError> {
        blocking(Arc::clone(&self.region), move || {
            recorded
                .rows
                .iter()
                .try_for_each(|batch| super::validate_rows(batch.table, &batch.rows))?;
            recorded.prepare(&operation).map_err(collect_error)
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
