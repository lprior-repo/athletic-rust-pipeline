use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use restate_sdk::prelude::*;

use crate::spawn::Spawner;
use census_store::clock::Clock;
use census_store::{Application, Credit as StoreCredit, Store, Table};

use super::ingest_validation::validate_rows;
use super::jobs::apply_observations;
use super::wire::ingest::{
    IngestReply, IngestRequest, IngestState, WindowRequest, MAX_OPERATION_ID_BYTES,
    MAX_WINDOW_LABEL_BYTES, WINDOW_LABEL_RING,
};
use super::{blocking, job_error, JobError, KEY_STATE};

mod raw;
mod recorded;

struct Credit {
    total: u64,
    cursor: Option<String>,
    today: String,
}

pub(super) fn payload_digest(table: Table, rows: &[Value]) -> Result<String, JobError> {
    let mut hasher = Sha256::new();
    hasher.update(table.file().as_bytes());
    for row in rows {
        hasher.update(
            serde_json::to_vec(row).map_err(|source| JobError::Terminal {
                message: format!("serializing observation row for the payload digest: {source}"),
            })?,
        );
    }
    Ok(format!("{:x}", hasher.finalize()))
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

pub(super) fn check_identifier(kind: &str, value: &str, limit: usize) -> Result<(), HandlerError> {
    if value.len() > limit {
        return Err(TerminalError::new(format!("{kind} exceeds {limit} bytes")).into());
    }
    Ok(())
}

struct Posted {
    table: Table,
    rows: Vec<Value>,
    operation: String,
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
    #[tracing::instrument(skip_all)]
    async fn recorded(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<super::wire::RecordedIngestRequest>,
    ) -> Result<Json<IngestReply>, HandlerError> {
        self.recording(&ctx, request).await
    }

    #[handler]
    #[tracing::instrument(skip_all, fields(rows = request.rows.len()))]
    async fn record(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<IngestRequest>,
    ) -> Result<Json<IngestReply>, HandlerError> {
        self.record_raw(&ctx, request).await
    }

    async fn credit(
        &self,
        ctx: &ObjectContext<'_>,
        endpoint: String,
        operation: String,
    ) -> Result<StoreCredit, HandlerError> {
        let store = Arc::clone(&self.store);
        let region = Arc::clone(&self.region);
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
        } = posted;
        let Json(application) = ctx
            .run(move || async move {
                blocking(region, move || {
                    validate_rows(table, &rows)?;
                    let digest = payload_digest(table, &rows)?;
                    apply_observations(&store, table, &rows, &operation, &digest)
                        .map_err(JobError::from)
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
        credit: Credit,
        ctx: &ObjectContext<'_>,
    ) -> Result<(), HandlerError> {
        state.total_observations = credit.total;
        if credit.cursor.is_some() {
            state.cursor = credit.cursor;
        }
        state.last_appended_at = Some(credit.today);
        Self::persist_state(ctx, state)
    }

    fn persist_state(ctx: &ObjectContext<'_>, state: &IngestState) -> Result<(), HandlerError> {
        let encoded = restate_sdk::serde::Serialize::serialize(&Json(state))
            .map_err(|error| TerminalError::new(format!("encoding ingest state: {error}")))?;
        ctx.set(KEY_STATE, encoded);
        Ok(())
    }

    #[handler]
    async fn complete_window(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<WindowRequest>,
    ) -> Result<Json<IngestState>, HandlerError> {
        raw::admit_window(&request.window)?;
        let mut state = self.load_object(&ctx).await?;
        if !state.windows.contains(&request.window) {
            state
                .windows
                .try_reserve(1)
                .map_err(|_| TerminalError::new("allocating bounded ingest windows"))?;
            if record_window(&mut state, request.window) {
                Self::persist_state(&ctx, &state)?;
            }
        }
        Ok(Json(state))
    }
}
