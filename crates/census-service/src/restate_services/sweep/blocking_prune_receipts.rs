use std::sync::Arc;

use census_store::{Pruned, ReplayFrontier, Store, StoreResult};
use serde::{Deserialize, Serialize};

use crate::spawn::Spawner;

use super::super::{blocking, JobError};

const OWED_REPLAY_SOURCE: &str = "Restate Admin POST /query: sys_invocation (id, status) and \
    sys_journal (id, index, entry_json), joined to a durable invocation-to-commit_once operation \
    registry and a certified fully committed replay day; Sweep has no admin client or registry";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum ReceiptPruneFailure {
    #[error(
        "receipt pruning refused without a durable replay day and complete active witness set; \
        standing frontier is preserved: {standing_frontier:?}; owed: {owed}"
    )]
    DurableReplaySourceUnavailable {
        standing_frontier: Option<ReplayFrontier>,
        owed: String,
    },
    #[error("{missing} active recovery witness operations have no standing receipt")]
    RecoveryWitnessesMissing { missing: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ReceiptPruning {
    pub pruned: Pruned,
    pub failure: Option<ReceiptPruneFailure>,
}

impl ReceiptPruning {
    pub(super) fn completed(pruned: Pruned) -> Self {
        let failure = match pruned.witnesses_missing {
            0 => None,
            missing => Some(ReceiptPruneFailure::RecoveryWitnessesMissing { missing }),
        };
        Self { pruned, failure }
    }

    fn refused(failure: ReceiptPruneFailure) -> Self {
        Self {
            pruned: Pruned::default(),
            failure: Some(failure),
        }
    }
}

fn replay_frontier(store: &Store) -> StoreResult<Result<ReplayFrontier, ReceiptPruneFailure>> {
    Ok(Err(ReceiptPruneFailure::DurableReplaySourceUnavailable {
        standing_frontier: store.receipt_frontier()?,
        owed: OWED_REPLAY_SOURCE.to_string(),
    }))
}

#[tracing::instrument(skip_all)]
pub(super) async fn sweep_receipts(
    spawner: Arc<Spawner>,
    store: Arc<Store>,
) -> Result<ReceiptPruning, JobError> {
    let source = Arc::clone(&store);
    let frontier = blocking(Arc::clone(&spawner), move || replay_frontier(&source)).await?;
    match frontier {
        Ok(frontier) => blocking_prune_receipts(spawner, store, frontier)
            .await
            .map(ReceiptPruning::completed),
        Err(failure) => Ok(ReceiptPruning::refused(failure)),
    }
}

#[tracing::instrument(skip_all)]
pub(super) async fn blocking_prune_receipts(
    spawner: Arc<Spawner>,
    store: Arc<Store>,
    frontier: ReplayFrontier,
) -> Result<Pruned, JobError> {
    blocking(spawner, move || store.prune_receipts(&frontier)).await
}
