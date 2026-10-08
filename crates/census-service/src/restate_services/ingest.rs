use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use restate_sdk::prelude::*;

use crate::spawn::Spawner;
use census_store::clock::Clock;
use census_store::{Application, Credit, Store, Table};

use super::ingest_validation::validate_rows;
use super::jobs::apply_observations;
use super::wire::ingest::{
    IngestReply, IngestRequest, IngestState, WindowRequest, MAX_OPERATION_ID_BYTES,
    MAX_WINDOW_LABEL_BYTES, WINDOW_LABEL_RING,
};
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

pub(super) fn record_window(state: &mut IngestState, window: String) -> bool {
    if state.windows.contains(&window) {
        return false;
    }
    state.windows_completed = state.completed_windows().saturating_add(1);
    state.windows.push(window);
    state.windows.sort();
    if state.windows.len() > WINDOW_LABEL_RING {
        state.windows.remove(0);
    }
    true
}

pub(super) fn check_identifier(
    kind: &str,
    value: &str,
    limit: usize,
) -> Result<(), HandlerError> {
    if value.len() > limit {
        return Err(TerminalError::new(format!("{kind} exceeds {limit} bytes")).into());
    }
    Ok(())
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
        let credit_store = Arc::clone(&self.store);
        let credit_region = Arc::clone(&self.region);
        let operation = request.operation_id;
        check_identifier("operation id", &operation, MAX_OPERATION_ID_BYTES)?;
        let rows = request.rows;
        let today = super::journaled_today(&ctx, &self.clock).await?;
        let mut state = self.load_object(&ctx).await?;
        let digest = payload_digest(table, &rows)?;
        let applied = self
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
        let written = applied.appended();
        let credit = self
            .credit(
                &ctx,
                credit_store,
                credit_region,
                state.endpoint.clone(),
                operation,
            )
            .await?;
        Ingest::update_ingest_state(
            &mut state,
            credit.total,
            request.cursor.clone(),
            today,
            &ctx,
        );
        Ok(Json(IngestReply {
            endpoint: state.endpoint,
            appended: credit.credited,
            written,
            total_observations: state.total_observations,
            cursor: state.cursor,
            last_appended_at: state.last_appended_at,
        }))
    }

    async fn credit(
        &self,
        ctx: &ObjectContext<'_>,
        store: Arc<Store>,
        region: Arc<Spawner>,
        endpoint: String,
        operation: String,
    ) -> Result<Credit, HandlerError> {
        let Json(credit) = ctx
            .run(move || async move {
                blocking(region, move || {
                    store.credit_observations(&endpoint, &operation)
                })
                .await
                .map(Json)
                .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(credit)
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
        total: u64,
        cursor: Option<String>,
        today: String,
        ctx: &ObjectContext<'_>,
    ) {
        state.total_observations = total;
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
        check_identifier("window label", &request.window, MAX_WINDOW_LABEL_BYTES)?;
        let mut state = self.load_object(&ctx).await?;
        if record_window(&mut state, request.window) {
            ctx.set(KEY_STATE, Json(state.clone()));
        }
        Ok(Json(state))
    }
}
