use super::*;
use crate::admission::{
    DepartureReason, Reservation, TryResult, MAX_BYPASSES, MAX_ORIGIN_WAITERS, WORK_CLASS_NORMAL,
    WORK_CLASS_STAGE,
};
use crate::spawn::Spawner;
use census_store::Store;

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

fn grant(result: TryResult<Reservation>) -> Result<Reservation, DepartureReason> {
    match result {
        TryResult::Granted(reservation) => Ok(reservation),
        TryResult::Deferred(reason) | TryResult::Refused(reason) => Err(reason),
    }
}

#[tokio::test]
async fn control_permit_is_granted_while_the_work_lane_is_protected() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let region = Arc::new(Spawner::with_capacity(1));
    let jobs = Jobs::new(store, Arc::clone(&region));
    let coordinator = region.coordinator();
    let before = coordinator.remaining();
    let held = grant(coordinator.try_reserve_profile(WORK_CLASS_STAGE))?;
    let starving = coordinator.try_enqueue(WORK_CLASS_STAGE)?;
    for _ in 0..MAX_BYPASSES {
        drop(grant(coordinator.try_reserve_profile(WORK_CLASS_NORMAL))?);
    }
    check!(matches!(
        coordinator.try_reserve_profile(WORK_CLASS_STAGE),
        TryResult::Deferred(_)
    ));
    let mut waiting = vec![starving];
    for _ in 1..MAX_ORIGIN_WAITERS {
        waiting.push(coordinator.try_enqueue(WORK_CLASS_NORMAL)?);
    }
    check!(matches!(
        coordinator.try_enqueue(WORK_CLASS_NORMAL),
        Err(DepartureReason::OriginWaitersFull)
    ));
    let control = jobs
        .permit_control()
        .await
        .map_err(|error| error.to_string())?;
    drop(control);
    drop(waiting);
    drop(held);
    check!(eq; coordinator.remaining(), before);
    Ok(())
}
