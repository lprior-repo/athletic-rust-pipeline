use std::sync::Arc;

use restate_sdk::prelude::*;

use crate::clock::Clock;
use crate::store::Store;

use super::wire::{OpenWorkReply, OpenWorkRequest, StatusReply, TableCount};

/// `Census`: the operator's read surface over the store.
///
/// Only the read lives here. Each of the four heavy jobs is a workflow of its own — see
/// [`Jobs`](super::publish::Jobs) — because a job is worth finishing: its journal and completion are
/// retained, so a caller that repeats the job attaches to the result instead of running months of
/// work again.
///
/// A read has no such need. It answers from the store in milliseconds, and retaining an invocation
/// per status check would be retention with nothing behind it.
#[derive(Clone)]
pub struct Census {
    store: Arc<Store>,
    clock: Arc<dyn Clock>,
}

impl Census {
    pub fn new(store: Arc<Store>, clock: Arc<dyn Clock>) -> Self {
        Self { store, clock }
    }
}

#[service(
    journal_retention = "1 hour",
    idempotency_retention = "30 days",
    invocation_retry_policy(
        initial_interval = "500ms",
        max_interval = "1m",
        max_attempts = 70,
        on_max_attempts = "pause"
    )
)]
impl Census {
    #[handler]
    async fn status(&self, _ctx: Context<'_>) -> Result<Json<StatusReply>, HandlerError> {
        let stats = self.store.stats()?;
        let tables = stats
            .tables
            .iter()
            .map(|(table, rows)| TableCount {
                table: (*table).to_string(),
                rows: *rows,
            })
            .collect();
        Ok(Json(StatusReply {
            tables,
            observations: stats.observations,
            bytes_on_disk: stats.bytes_on_disk,
            today: self.clock.today(),
        }))
    }

    /// The durable run's own open work: which jurisdiction sweeps still owe stages, and which source
    /// objects have accepted nothing.
    ///
    /// A read like [`Self::status`], but from the workflow's objects rather than from the store:
    /// these two counts are properties of the run, and the objects that did the work are the only
    /// surface that records them. The season and revision name the run, so a caller reads one run's
    /// work and not another's, and a count nobody could take comes back `None` rather than as zero.
    #[handler]
    async fn open_work(
        &self,
        ctx: Context<'_>,
        Json(request): Json<OpenWorkRequest>,
    ) -> Result<Json<OpenWorkReply>, HandlerError> {
        Ok(Json(super::open_work::measure(&ctx, &request).await?))
    }
}
