use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use restate_sdk::prelude::*;

use crate::spawn::Spawner;
use census_domain::model::{
    AppliedAthleteIdentity, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, CollectionSnapshot, CoverageRow,
    RetainedConflict, ReviewCase, ReviewVerdictRecord, SourceAccessCondition, SourceMeetRef,
    SourceObjectIdentity, SourceObservation,
};
use census_store::clock::Clock;
use census_store::{Application, Store, Table};

use super::jobs::apply_observations;
use super::wire::ingest::{IngestReply, IngestRequest, IngestState, WindowRequest};
use super::{blocking, job_error, resolve_table, JobError, KEY_STATE};

fn validate_rows(table: Table, rows: &[Value]) -> Result<(), JobError> {
    for (i, row) in rows.iter().enumerate() {
        let name = table.file();
        match table {
            Table::Schools => {
                let _: CanonicalSchool =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::Teams => {
                let _: CanonicalTeam =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::Coaches => {
                let _: CanonicalCoach =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::Athletes => {
                let _: CanonicalAthlete =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::Meets => {
                let _: CanonicalMeet =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::Events => {
                let _: CanonicalEvent =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::Performances => {
                let _: CanonicalPerformance =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::SourceIdentities => {
                let _: SourceObjectIdentity =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::Conflicts => {
                let _: RetainedConflict =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::ReviewCases => {
                let _: ReviewCase =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::Coverage => {
                let _: CoverageRow =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::Snapshots => {
                let _: CollectionSnapshot =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::SourceAccess => {
                let _: SourceAccessCondition =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::IdentityVerdicts => {
                let _: ReviewVerdictRecord =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::SourceMeets => {
                let _: SourceMeetRef =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::SourceObservations => {
                let _: SourceObservation =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
            Table::AthleteIdentityDecisions => {
                let _: AppliedAthleteIdentity =
                    serde_json::from_value(row.clone()).map_err(|source| JobError::Terminal {
                        message: format!("row {i} for table {name} failed validation: {source}"),
                    })?;
            }
        }
    }
    Ok(())
}

pub(super) fn payload_digest(table: Table, rows: &[Value]) -> Result<String, HandlerError> {
    let mut hasher = Sha256::new();
    hasher.update(table.file().as_bytes());
    for row in rows {
        hasher.update(
            serde_json::to_vec(row).map_err(|source| JobError::Terminal {
                message: format!("serializing observation row for the payload digest: {source}"),
            })?,
        );
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

struct Posted {
    table: Table,
    rows: Vec<Value>,
    operation: String,
    digest: String,
}

#[derive(Clone)]
pub struct Ingest {
    store: Arc<Store>,
    clock: Arc<dyn Clock>,
    region: Arc<Spawner>,
}

impl Ingest {
    pub fn new(store: Arc<Store>, clock: Arc<dyn Clock>, region: Arc<Spawner>) -> Self {
        Self {
            store,
            clock,
            region,
        }
    }

    async fn load_object(&self, ctx: &ObjectContext<'_>) -> Result<IngestState, HandlerError> {
        let endpoint = ctx.key().to_string();
        Ok(ctx
            .get::<Json<IngestState>>(KEY_STATE)
            .await?
            .map(|state| state.0)
            .unwrap_or_else(|| IngestState {
                endpoint,
                ..IngestState::default()
            }))
    }

    async fn load_shared(
        &self,
        ctx: &SharedObjectContext<'_>,
    ) -> Result<IngestState, HandlerError> {
        let endpoint = ctx.key().to_string();
        Ok(ctx
            .get::<Json<IngestState>>(KEY_STATE)
            .await?
            .map(|state| state.0)
            .unwrap_or_else(|| IngestState {
                endpoint,
                ..IngestState::default()
            }))
    }
}

#[object(
    journal_retention = "90 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(
        initial_interval = "500ms",
        max_interval = "1m",
        max_attempts = 3,
        on_max_attempts = "pause"
    )
)]
impl Ingest {
    #[handler]
    async fn state(&self, ctx: SharedObjectContext<'_>) -> Result<Json<IngestState>, HandlerError> {
        Ok(Json(self.load_shared(&ctx).await?))
    }

    #[handler]
    #[tracing::instrument(skip_all, fields(rows = request.rows.len()))]
    async fn record(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<IngestRequest>,
    ) -> Result<Json<IngestReply>, HandlerError> {
        let table = resolve_table(&request.table)?;
        validate_rows(table, &request.rows)?;
        let store = Arc::clone(&self.store);
        let region = Arc::clone(&self.region);
        let operation = request.operation_id;
        let rows = request.rows;
        let today = super::journaled_today(&ctx, &self.clock).await?;
        let mut state = self.load_object(&ctx).await?;
        let digest = payload_digest(table, &rows)?;
        let application = self
            .apply(
                &ctx,
                store,
                region,
                Posted {
                    table,
                    rows,
                    operation: operation.clone(),
                    digest,
                },
            )
            .await?;
        let appended = application.appended();
        Ingest::update_ingest_state(
            &mut state,
            appended,
            operation,
            request.cursor.clone(),
            today,
            &ctx,
        );
        Ok(Json(IngestReply {
            endpoint: state.endpoint,
            appended,
            total_observations: state.total_observations,
            cursor: state.cursor,
            last_appended_at: state.last_appended_at,
        }))
    }

    async fn apply(
        &self,
        ctx: &ObjectContext<'_>,
        store: Arc<Store>,
        region: Arc<Spawner>,
        posted: Posted,
    ) -> Result<Application, HandlerError> {
        let Posted {
            table,
            rows,
            operation,
            digest,
        } = posted;
        let Json(application) = ctx
            .run(move || async move {
                blocking(region, move || {
                    apply_observations(&store, table, &rows, &operation, &digest)
                })
                .await
                .map(Json)
                .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(application)
    }

    fn update_ingest_state(
        state: &mut IngestState,
        appended: u64,
        operation: String,
        cursor: Option<String>,
        today: String,
        ctx: &ObjectContext<'_>,
    ) {
        state.total_observations = state.total_observations.saturating_add(appended);
        if !state.seen_operations.contains(&operation) {
            state.seen_operations.push(operation);
        }
        if cursor.is_some() {
            state.cursor = cursor;
        }
        state.last_appended_at = Some(today);
        ctx.set(KEY_STATE, Json(state.clone()));
    }

    #[handler]
    async fn complete_window(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<WindowRequest>,
    ) -> Result<Json<IngestState>, HandlerError> {
        if request.window.trim().is_empty() {
            return Err(TerminalError::new("window label must not be empty").into());
        }
        let mut state = self.load_object(&ctx).await?;
        if !state.windows.contains(&request.window) {
            state.windows.push(request.window);
            state.windows.sort();
            ctx.set(KEY_STATE, Json(state.clone()));
        }
        Ok(Json(state))
    }
}
