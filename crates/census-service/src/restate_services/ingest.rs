use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use restate_sdk::prelude::*;

use crate::spawn::Spawner;
use census_store::clock::Clock;
use census_store::{Application, Store, Table};

use super::ingest_validation::validate_rows;
use super::jobs::apply_observations;
use super::wire::ingest::{IngestReply, IngestRequest, IngestState, WindowRequest};
use super::{blocking, job_error, resolve_table, JobError, KEY_STATE};

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

pub(super) fn credited_appended(
    state: &IngestState,
    operation: &str,
    receipt_appended: u64,
) -> u64 {
    if state.seen_operations.iter().any(|seen| seen == operation) {
        0
    } else {
        receipt_appended
    }
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
        Ok(
            match ctx
                .get::<Json<IngestState>>(KEY_STATE)
                .await?
                .map(|state| state.0)
            {
                Some(value) => {
                    drop(endpoint);
                    value
                }
                None => IngestState {
                    endpoint,
                    ..IngestState::default()
                },
            },
        )
    }

    async fn load_shared(
        &self,
        ctx: &SharedObjectContext<'_>,
    ) -> Result<IngestState, HandlerError> {
        let endpoint = ctx.key().to_string();
        Ok(
            match ctx
                .get::<Json<IngestState>>(KEY_STATE)
                .await?
                .map(|state| state.0)
            {
                Some(value) => {
                    drop(endpoint);
                    value
                }
                None => IngestState {
                    endpoint,
                    ..IngestState::default()
                },
            },
        )
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
        let appended = credited_appended(&state, &operation, application.receipt().appended);
        let written = application.appended();
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
            written,
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
