use super::super::Census;
use crate::admission::WORK_CLASS_CONTROL;
use crate::restate_services::Jobs;
use crate::spawn::{Spawner, TaskReport};
use census_store::{clock::SystemClock, Store};
use std::sync::Arc;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CONTROL_HOLDERS: usize = 64;

#[tokio::test(flavor = "current_thread")]
async fn status_filesystem_walk_runs_on_the_blocking_region() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let directory = root.path().join("status-probe/nested");
    std::fs::create_dir_all(&directory)?;
    std::fs::write(directory.join("retained"), [1_u8; 4096])?;
    let region = Arc::new(Spawner::with_capacity(1));
    let jobs = Jobs::new(Arc::clone(&store), Arc::clone(&region));
    let census = Census::new(store, Arc::new(SystemClock), jobs);
    let mut held = Vec::new();
    for _ in 0..CONTROL_HOLDERS {
        held.push(region.coordinator().reserve(WORK_CLASS_CONTROL).await?);
    }
    let mut pending = Box::pin(census.status_stats());
    check!(futures::poll!(pending.as_mut()).is_pending());
    drop(held);
    let stats = pending.await?;
    check!(stats.store_bytes >= 4096);
    check!(eq; stats.observations, 0);
    check!(eq; region.coordinator().remaining().slots.stages, 1);
    check!(eq; region.drain(Duration::from_secs(5)).await?, TaskReport {
        accepted: 1,
        completed: 1,
        ..TaskReport::default()
    });
    Ok(())
}
