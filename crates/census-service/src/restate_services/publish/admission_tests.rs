use super::*;
use crate::admission::{TryResult, WORK_CLASS_STAGE};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn jobs_permit_retains_shared_byte_and_stage_grant() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let region = Arc::new(Spawner::with_capacity(1));
    let jobs = Jobs::new(store, Arc::clone(&region));
    let before = region.coordinator().remaining();
    let permit = jobs.permit().await.map_err(|error| error.to_string())?;
    check!(eq; region.coordinator().remaining().bytes,
        before.bytes - WORK_CLASS_STAGE.resources.bytes);
    check!(eq; region.coordinator().remaining().slots.stages, 0);
    check!(matches!(
        region.coordinator().try_reserve_profile(WORK_CLASS_STAGE),
        TryResult::Deferred(_)
    ));
    drop(permit);
    check!(eq; region.coordinator().remaining(), before);
    Ok(())
}
