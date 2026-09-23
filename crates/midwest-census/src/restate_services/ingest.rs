use std::sync::Arc;

use restate_sdk::prelude::*;

use crate::clock::Clock;
use crate::spawn::Spawner;
use crate::store::Store;

use super::jobs::append_observations;
use super::wire::ingest::{IngestReply, IngestRequest, IngestState, WindowRequest};
use super::{blocking, job_error, resolve_table, JobError, KEY_STATE};

/// `Ingest`: durable per-endpoint cursor and window bookkeeping, plus the append itself.
#[derive(Clone)]
pub struct Ingest {
    store: Arc<Store>,
    clock: Arc<dyn Clock>,
    /// The shell's region: the append runs through it, so an aborted invocation cannot leave a
    /// writer behind the drain does not own.
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

    /// Read the endpoint's state. An endpoint that has never recorded anything reads as empty
    /// rather than as an error: absence is the normal first-run state, not a failure.
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

    /// The same read through the read-only (shared) handler context.
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

// Ingest is a state machine keyed by acquisition run: `record` advances it and `complete_window`
// closes the window. It holds no completion of its own, so no workflow-completion retention.
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
        let store = Arc::clone(&self.store);
        let region = Arc::clone(&self.region);
        let rows = request.rows;
        // The date is journaled, not read: `ctx.set` below compares the serialized payload on
        // replay, so a wall-clock read that has moved on to the next day would fail the invocation
        // with a journal mismatch instead of replaying it.
        let today = super::journaled_today(&ctx, &self.clock).await?;
        let appended = ctx
            .run(move || async move {
                blocking(region, move || append_observations(&store, table, &rows))
                    .await
                    // The accepted count reaches the wire as `u64`. A host where it does not fit is
                    // a hard failure: a clamped "appended" figure would be a fabricated total.
                    .and_then(|count| {
                        u64::try_from(count).map_err(|_| JobError::Terminal {
                            message: format!("appended row count {count} does not fit u64"),
                        })
                    })
                    .map_err(job_error)
            })
            .await?;

        let mut state = self.load_object(&ctx).await?;
        state.total_observations = state.total_observations.saturating_add(appended);
        if request.cursor.is_some() {
            state.cursor = request.cursor.clone();
        }
        state.last_appended_at = Some(today);

        // One write, not four: the cursor, the totals, and the timestamp can never disagree.
        ctx.set(KEY_STATE, Json(state.clone()));

        Ok(Json(IngestReply {
            endpoint: state.endpoint,
            appended,
            total_observations: state.total_observations,
            cursor: state.cursor,
            last_appended_at: state.last_appended_at,
        }))
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
