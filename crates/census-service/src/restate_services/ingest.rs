use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use restate_sdk::prelude::*;

use crate::spawn::Spawner;
use census_store::clock::Clock;
use census_store::{Application, Store, Table};

use super::jobs::apply_observations;
use super::wire::ingest::{IngestReply, IngestRequest, IngestState, WindowRequest};
use super::{blocking, job_error, resolve_table, JobError, KEY_STATE};

/// The digest one request's payload hashes to: the target table's name and every row, canonically.
///
/// The digest is the payload half of a receipt; the operation id the caller sends is the identity
/// half. They are deliberately not derived from one another: an id derived from the payload cannot
/// notice that the payload changed under it, which is the case a re-used operation id has to fail
/// on rather than append.
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

/// One posted operation, as the handler hands it to the store: what to write, under which name, and
/// the digest of the payload that name covers.
struct Posted {
    table: Table,
    rows: Vec<Value>,
    operation: String,
    digest: String,
}

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
        let operation = request.operation_id;
        let rows = request.rows;
        // The date is journaled, not read: `ctx.set` below compares the serialized payload on
        // replay, so a wall-clock read that has moved on to the next day would fail the invocation
        // with a journal mismatch instead of replaying it.
        let today = super::journaled_today(&ctx, &self.clock).await?;
        let mut state = self.load_object(&ctx).await?;
        let digest = payload_digest(table, &rows)?;
        // Whether this request is new work or a replay of work the store already holds is the
        // store's answer: its receipt commits with the rows, so an invocation that never heard its
        // own acknowledgement finds the receipt standing and appends nothing. The same operation id
        // offered with a different payload is refused there, terminally.
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

    /// Apply one operation to the store: its rows and its receipt, in one commit.
    ///
    /// The step's value is journaled as the [`Application`] the store answered, so a replay that
    /// never re-executes the closure reads back which of the two it was — while a replay that *does*
    /// re-execute it, because the first attempt died before the journal recorded it, is answered by
    /// the store's own receipt.
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
            .await?;
        Ok(application)
    }

    /// Update ingest state: counters, cursor, the operation just applied, timestamp. Must follow
    /// [`Self::apply`].
    ///
    /// The operation list is this object's own view for an operator reading `state`; the store's
    /// receipt is what decides a repeat, so a replay that appended nothing still names the operation
    /// it replayed rather than adding a second copy of it.
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
