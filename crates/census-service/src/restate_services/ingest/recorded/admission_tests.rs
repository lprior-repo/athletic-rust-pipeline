use super::*;
use crate::admission::WORK_CLASS_STAGE;
use crate::spawn::Spawner;
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn prepared_recording_retains_shared_stage_budget() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(census_store::Store::open(root.path())?);
    let region = Arc::new(Spawner::with_capacity(1));
    let recorded = census_store::recorded::Recorded::default();
    let encoded = serde_json::to_vec(&recorded)?;
    let manifest = serde_json::json!({"digest": format!("{:x}", Sha256::digest(&encoded)),
        "encoded_bytes": encoded.len(), "rows": 0});
    let operation = census_crawl::recording::spool::fragment_key("budget-check", 0);
    store.journal_done(
        census_crawl::recording::spool::FRAGMENT_PHASE,
        &operation,
        &recorded,
    )?;
    store.journal_done("meet_recorded_fragment_manifests_v1", &operation, &manifest)?;
    let summary = census_store::recorded::FragmentSummary {
        fragments: 1,
        digest: format!("{:x}", Sha256::digest(serde_json::to_vec(&manifest)?)),
        encoded_bytes: encoded.len(),
        rows: 0,
    };
    let ingest = Ingest::new(
        store,
        Arc::new(census_store::clock::SystemClock),
        Arc::clone(&region),
    );
    let held = region.coordinator().reserve(WORK_CLASS_STAGE).await?;
    let mut pending =
        Box::pin(ingest.prepare_recording("budget-check".to_string(), 0, summary, operation));
    check!(futures::poll!(pending.as_mut()).is_pending());
    drop(held);
    let prepared = pending.await.map_err(|error| format!("{error:?}"))?;
    check!(eq; region.coordinator().remaining().slots.stages, 0);
    drop(prepared);
    check!(eq; region.coordinator().remaining().slots.stages, 1);
    region.drain(std::time::Duration::from_secs(5)).await?;
    Ok(())
}
