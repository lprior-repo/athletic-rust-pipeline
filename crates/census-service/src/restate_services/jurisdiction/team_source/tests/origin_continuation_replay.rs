use super::{key, ledger, replay, request};
use crate::restate_services::national::publication_tests::codec;
use census_store::Store;
use std::sync::Arc;

mod fixture;
mod history;
mod transport;

use fixture::{Driver, TestResult};
use history::{phase, Seed};

#[tokio::test]
async fn committed_continuation_replays_before_run_ack_after_cold_store_reopen() -> TestResult {
    let root = tempfile::tempdir()?;
    let (store, writer) = transport::qualified(root.path())?;
    let seed = Seed::new(&store)?;
    let mut original = Driver::new(&seed.original, &writer)?;
    let original_output = original.until_output(&store, &seed.original, "run").await?;
    check!(eq; original_output, seed.original_output);
    let mut continuation = Driver::new(&seed.continuation, &writer)?;
    continuation.until_committed(&store, &seed.original).await?;
    seed.assert_original(&store)?;
    let settled = seed.assert_continued(&store)?;
    let committed = phase(&store, ledger::origin_continuation::PHASE)?;
    let retained = continuation.withhold_last_ack()?;
    check!(eq; retained, serde_json::json!({ "Settled": settled }));
    let replayed_runs = continuation
        .run_values
        .len()
        .checked_add(1)
        .ok_or("fixture Run count overflow")?;
    let journal_path = root.path().join("continuation.journal");
    continuation.persist(&journal_path)?;
    store.flush()?;
    check!(eq; Arc::strong_count(&store), 1);
    drop((writer, store));
    let (store, writer) = Store::open_current_qualification_v3(root.path())?.into_parts();
    let store = Arc::new(store);
    let writer = Arc::new(writer);
    original.writer = Arc::clone(&writer);
    continuation.writer = Arc::clone(&writer);
    continuation.restore(&journal_path)?;
    let original_runs = original.run_values.len();
    check!(eq; original.until_output(&store, &seed.original, "run").await?, original_output);
    check!(eq; original.run_values.len(), original_runs);
    let continued_output = continuation
        .until_output(&store, &seed.original, "continue_origin_held")
        .await?;
    check!(eq; continued_output, serde_json::to_vec(&settled)?);
    check!(eq; continuation.run_values.len(), replayed_runs);
    check!(eq; continuation.run_values.last(), Some(&retained));
    check!(eq; phase(&store, ledger::origin_continuation::PHASE)?, committed);
    seed.assert_original(&store)?;
    seed.assert_continued(&store)?;
    assert_reattached(&store, &writer, &seed, &original_output, &continued_output).await?;
    check!(eq; phase(&store, ledger::origin_continuation::PHASE)?, committed);
    Ok(())
}

async fn assert_reattached(
    store: &Arc<Store>,
    writer: &Arc<census_store::CaptureWriterV1>,
    seed: &Seed,
    original_output: &[u8],
    continued_output: &[u8],
) -> TestResult {
    let mut replayed = Driver::new(&seed.original, writer)?;
    check!(eq; replayed.until_output(store, &seed.original, "run").await?, original_output);
    let mut reattached = Driver::new(&seed.continuation, writer)?;
    check!(eq; reattached.until_output(store, &seed.original, "continue_origin_held").await?, continued_output);
    check!(eq; reattached.run_values.len(), 2);
    seed.assert_original(store)?;
    seed.assert_continued(store)?;
    Ok(())
}

#[tokio::test]
async fn wrong_original_digest_is_refused_without_authorization_or_slot_two() -> TestResult {
    let root = tempfile::tempdir()?;
    let (store, writer) = transport::qualified(root.path())?;
    let mut seed = Seed::new(&store)?;
    seed.continuation.original_outcome_digest = "0".repeat(64);
    let mut driver = Driver::new(&seed.continuation, &writer)?;
    let failure = driver.until_failure(&store, &seed.original).await?;
    check!(failure.contains("origin continuation original outcome digest mismatch"));
    check!(phase(&store, ledger::origin_continuation::PHASE)?.is_empty());
    seed.assert_original(&store)?;
    check!(eq; ledger::baseline(&store, &seed.operation)?, 1);
    check!(!store.journal_contains(
        ledger::PHASE,
        &format!("{}/attempt/2/reserved", seed.operation)
    )?);
    check!(!store.root().join("fixture-origin-locks").exists());
    Ok(())
}
