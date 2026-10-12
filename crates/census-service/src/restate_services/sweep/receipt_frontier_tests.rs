use super::blocking_prune_receipts::{
    blocking_prune_receipts, sweep_receipts, ReceiptPruneFailure, ReceiptPruning,
};
use crate::restate_services::JobError;
use crate::spawn::Spawner;
use census_store::{Pruned, ReplayFrontier, ReplayWitness, Store};
use std::sync::Arc;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn frontier() -> ReplayFrontier {
    ReplayFrontier {
        at: "2100-01-01".to_string(),
        witnesses: vec![ReplayWitness {
            invocation: "active-invocation".to_string(),
            operation: "active-operation".to_string(),
        }],
    }
}

fn receipt(store: &Store, operation: &str) -> TestResult {
    store.write_batch().commit_once(operation, "digest")?;
    Ok(())
}

async fn drain(region: &Spawner) -> TestResult {
    let report = region.drain(Duration::from_secs(5)).await?;
    check!(eq; report.remaining, 0);
    check!(eq; report.panicked, 0);
    Ok(())
}

#[tokio::test]
async fn sweep_refuses_pruning_without_a_durable_replay_source_on_a_fresh_store() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Arc::new(Store::open(dir.path())?);
    receipt(&store, "active-operation")?;
    let region = Arc::new(Spawner::new());
    let result = sweep_receipts(Arc::clone(&region), Arc::clone(&store)).await?;
    check!(eq; result.pruned, Pruned::default());
    match result.failure {
        Some(ReceiptPruneFailure::DurableReplaySourceUnavailable {
            standing_frontier,
            owed,
        }) => {
            check!(eq; standing_frontier, None);
            check!(owed.contains("Restate Admin POST /query"));
            check!(owed.contains("sys_invocation (id, status)"));
            check!(owed.contains("durable invocation-to-commit_once operation"));
        }
        other => return Err(format!("expected typed source refusal, got {other:?}").into()),
    }
    check!(eq; store.receipt_count()?, 1);
    check!(eq; store.receipt_frontier()?, None);
    drain(&region).await
}

#[tokio::test]
async fn sweep_preserves_the_reopened_standing_frontier_and_all_receipts_when_refused() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let standing = frontier();
    {
        let store = Store::open(dir.path())?;
        receipt(&store, "active-operation")?;
        store.prune_receipts(&standing)?;
        receipt(&store, "later-active-operation")?;
    }
    let store = Arc::new(Store::open(dir.path())?);
    let region = Arc::new(Spawner::new());
    let result = sweep_receipts(Arc::clone(&region), Arc::clone(&store)).await?;
    check!(eq; result.pruned, Pruned::default());
    match result.failure {
        Some(ReceiptPruneFailure::DurableReplaySourceUnavailable {
            standing_frontier, ..
        }) => {
            check!(eq; standing_frontier, Some(standing.clone()));
        }
        other => return Err(format!("expected typed source refusal, got {other:?}").into()),
    }
    check!(eq; store.receipt_count()?, 2);
    check!(eq; store.receipt_frontier()?, Some(standing));
    drain(&region).await
}

#[tokio::test]
async fn sweep_prune_passes_the_replay_frontier_and_retains_active_receipts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Arc::new(Store::open(dir.path())?);
    receipt(&store, "active-operation")?;
    receipt(&store, "completed-operation")?;
    let region = Arc::new(Spawner::new());
    let result =
        blocking_prune_receipts(Arc::clone(&region), Arc::clone(&store), frontier()).await?;
    check!(eq; result, Pruned {
        removed: 1,
        undated: 0,
        witnesses_kept: 1,
        witnesses_missing: 0,
        frontier_advanced: true,
    });
    check!(store.receipt("active-operation")?.is_some());
    check!(eq; store.receipt("completed-operation")?, None);
    let same = blocking_prune_receipts(Arc::clone(&region), Arc::clone(&store), frontier()).await?;
    check!(eq; same.removed, 0);
    check!(!same.frontier_advanced);
    check!(eq; ReceiptPruning::completed(same).failure, None);
    drain(&region).await
}

#[tokio::test]
async fn sweep_prune_marks_missing_active_recovery_witnesses_unrecoverable() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Arc::new(Store::open(dir.path())?);
    let region = Arc::new(Spawner::new());
    let pruned = blocking_prune_receipts(Arc::clone(&region), store, frontier()).await?;
    let result = ReceiptPruning::completed(pruned);
    check!(eq; result.pruned.witnesses_missing, 1);
    check!(eq; result.failure, Some(ReceiptPruneFailure::RecoveryWitnessesMissing { missing: 1 }));
    let encoded = serde_json::to_value(&result)?;
    check!(eq; encoded.pointer("/failure/RecoveryWitnessesMissing/missing"), Some(&serde_json::json!(1)));
    drain(&region).await
}

#[tokio::test]
async fn sweep_prune_refuses_a_backwards_day_without_mutating_the_frontier() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Arc::new(Store::open(dir.path())?);
    receipt(&store, "active-operation")?;
    store.prune_receipts(&frontier())?;
    let mut behind = frontier();
    behind.at = "2099-12-31".to_string();
    let region = Arc::new(Spawner::new());
    let result = blocking_prune_receipts(Arc::clone(&region), Arc::clone(&store), behind).await;
    match result {
        Err(JobError::Terminal { message }) => check!(message.contains("moves behind")),
        other => return Err(format!("expected terminal backwards refusal, got {other:?}").into()),
    }
    check!(eq; store.receipt_frontier()?, Some(frontier()));
    check!(eq; store.receipt_count()?, 1);
    drain(&region).await
}

#[tokio::test]
async fn sweep_prune_refuses_oversized_witnesses_without_losing_receipts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Arc::new(Store::open(dir.path())?);
    receipt(&store, "active-operation")?;
    let region = Arc::new(Spawner::new());
    let mut oversized = frontier();
    oversized.witnesses = vec![ReplayWitness {
        invocation: "i".repeat(census_store::MAX_OPERATION_BYTES.saturating_add(1)),
        operation: "active-operation".to_string(),
    }];
    match blocking_prune_receipts(Arc::clone(&region), Arc::clone(&store), oversized).await {
        Err(JobError::Terminal { message }) => check!(message.contains("byte ceiling")),
        other => return Err(format!("expected bounded witness refusal, got {other:?}").into()),
    }
    let mut oversized = frontier();
    oversized.witnesses = vec![
        oversized.witnesses.first().ok_or("witness")?.clone();
        census_store::MAX_FRONTIER_WITNESSES.saturating_add(1)
    ];
    match blocking_prune_receipts(Arc::clone(&region), Arc::clone(&store), oversized).await {
        Err(JobError::Terminal { message }) => check!(message.contains("witness ceiling")),
        other => return Err(format!("expected witness count refusal, got {other:?}").into()),
    }
    check!(eq; store.receipt_count()?, 1);
    check!(eq; store.receipt_frontier()?, None);
    drain(&region).await
}

#[test]
fn sweep_prune_preserves_all_counts_when_flagging_unrecoverable_witnesses() -> TestResult {
    let pruned = Pruned {
        removed: 7,
        undated: 3,
        witnesses_kept: 2,
        witnesses_missing: 4,
        frontier_advanced: true,
    };
    let result = ReceiptPruning::completed(pruned);
    check!(eq; result.pruned, pruned);
    check!(eq; result.failure, Some(ReceiptPruneFailure::RecoveryWitnessesMissing { missing: 4 }));
    Ok(())
}
