use super::*;
use crate::admission::{DepartureReason, Resources, TryResult, WORK_CLASS_DRAIN};
use crate::restate_services::JobError;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver};
use std::task::{Context, Poll, Waker};
use std::time::Duration;
use tokio::sync::oneshot;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const DEADLINE: Duration = Duration::from_secs(5);

struct Fixture {
    jobs: Jobs,
    region: Arc<Spawner>,
    _directory: tempfile::TempDir,
}

impl Fixture {
    fn new(capacity: usize) -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let store = Arc::new(Store::open(directory.path())?);
        let region = Arc::new(Spawner::with_capacity(capacity));
        Ok(Self {
            jobs: Jobs::new(store, Arc::clone(&region)),
            region,
            _directory: directory,
        })
    }

    fn other_jobs(&self) -> Jobs {
        Jobs::new(Arc::clone(self.jobs.store()), Arc::clone(&self.region))
    }
}

async fn bounded<F: Future>(future: F) -> TestResult<F::Output> {
    Ok(tokio::time::timeout(DEADLINE, future).await?)
}

async fn permit(jobs: &Jobs) -> TestResult<Reservation> {
    Ok(bounded(jobs.permit_publication())
        .await?
        .map_err(|error| error.to_string())?)
}

fn pending<F: Future>(future: Pin<&mut F>) -> TestResult {
    let mut context = Context::from_waker(Waker::noop());
    check!(matches!(future.poll(&mut context), Poll::Pending));
    Ok(())
}

fn charged(mut before: Resources) -> TestResult<Resources> {
    let units = WORK_CLASS_PUBLICATION.resources;
    before.bytes = before.bytes.checked_sub(units.bytes).ok_or("byte debit")?;
    before.work = before.work.checked_sub(units.work).ok_or("work debit")?;
    before.slots.stages = before.slots.stages.checked_sub(1).ok_or("stage debit")?;
    before.slots.populations = before
        .slots
        .populations
        .checked_sub(1)
        .ok_or("population debit")?;
    Ok(before)
}

fn terminal(message: &str) -> JobError {
    JobError::Terminal {
        message: message.to_string(),
    }
}

fn wait_release(release: Receiver<()>) -> Result<(), JobError> {
    release
        .recv_timeout(DEADLINE)
        .map_err(|error| terminal(&error.to_string()))
}

struct DropWitness(Arc<AtomicBool>);

impl Drop for DropWitness {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn independently_constructed_jobs_share_population_admission_and_control_progress(
) -> TestResult {
    let fixture = Fixture::new(4)?;
    let before = fixture.region.coordinator().remaining();
    let held = permit(&fixture.jobs).await?;
    let other = fixture.other_jobs();
    let mut queued = Box::pin(other.permit_publication());
    pending(queued.as_mut())?;
    check!(eq; fixture.region.coordinator().remaining(), charged(before)?);
    let control = bounded(other.permit_control())
        .await?
        .map_err(|error| error.to_string())?;
    let drain = bounded(fixture.region.coordinator().reserve(WORK_CLASS_DRAIN)).await??;
    check!(eq; fixture.region.coordinator().remaining(), charged(before)?);
    drop((control, drain, held));
    let next = bounded(queued).await?.map_err(|error| error.to_string())?;
    check!(eq; fixture.region.coordinator().remaining(), charged(before)?);
    drop(next);
    check!(eq; fixture.region.coordinator().remaining(), before);
    Ok(())
}

#[tokio::test]
async fn cancelled_caller_keeps_population_charged_until_owned_worker_drops_its_state() -> TestResult
{
    let fixture = Fixture::new(4)?;
    let before = fixture.region.coordinator().remaining();
    let held = permit(&fixture.jobs).await?;
    let dropped = Arc::new(AtomicBool::new(false));
    let witness = Arc::clone(&dropped);
    let (started, entered) = oneshot::channel();
    let (release, released) = sync_channel(1);
    let caller = tokio::spawn(run_reserved(Arc::clone(&fixture.region), held, move || {
        let _population = DropWitness(witness);
        started
            .send(())
            .map_err(|_| terminal("lost start observer"))?;
        wait_release(released)
    }));
    bounded(entered).await??;
    caller.abort();
    check!(matches!(bounded(caller).await?, Err(error) if error.is_cancelled()));
    check!(!dropped.load(Ordering::SeqCst));
    check!(eq; fixture.region.coordinator().remaining(), charged(before)?);
    check!(matches!(
        fixture
            .region
            .coordinator()
            .try_reserve_profile(WORK_CLASS_PUBLICATION),
        TryResult::Deferred(DepartureReason::NoSlots)
    ));
    let other = fixture.other_jobs();
    let mut next = Box::pin(other.permit_publication());
    pending(next.as_mut())?;
    let control = bounded(other.permit_control())
        .await?
        .map_err(|error| error.to_string())?;
    let drain = bounded(fixture.region.coordinator().reserve(WORK_CLASS_DRAIN)).await??;
    drop((control, drain));
    release.send(())?;
    let next = bounded(next).await?.map_err(|error| error.to_string())?;
    check!(dropped.load(Ordering::SeqCst));
    check!(eq; fixture.region.coordinator().remaining(), charged(before)?);
    check!(eq; bounded(run_reserved(Arc::clone(&fixture.region), next, || Ok::<_, JobError>(7))).await??, 7);
    check!(eq; fixture.region.coordinator().remaining(), before);
    let report = bounded(fixture.region.drain(DEADLINE)).await??;
    check!(eq; report.accepted, 2);
    check!(eq; report.completed, 2);
    check!(eq; report.remaining, 0);
    Ok(())
}

#[tokio::test]
async fn cancellation_while_waiting_for_blocking_lane_releases_unstarted_publication() -> TestResult
{
    let fixture = Fixture::new(1)?;
    let before = fixture.region.coordinator().remaining();
    let (started, entered) = oneshot::channel();
    let (release, released) = sync_channel(1);
    let region = Arc::clone(&fixture.region);
    let holder = tokio::spawn(async move {
        region
            .blocking(move || {
                started
                    .send(())
                    .map_err(|_| terminal("lost lane observer"))?;
                wait_release(released)
            })
            .await
    });
    bounded(entered).await??;
    let held = permit(&fixture.jobs).await?;
    let ran = Arc::new(AtomicBool::new(false));
    let entered_job = Arc::clone(&ran);
    let mut queued = Box::pin(run_reserved(Arc::clone(&fixture.region), held, move || {
        entered_job.store(true, Ordering::SeqCst);
        Ok::<_, JobError>(())
    }));
    pending(queued.as_mut())?;
    check!(eq; fixture.region.coordinator().remaining(), charged(before)?);
    drop(queued);
    check!(!ran.load(Ordering::SeqCst));
    check!(eq; fixture.region.coordinator().remaining(), before);
    release.send(())?;
    check!(matches!(
        bounded(holder).await??,
        crate::outcome::Outcome::Ok(())
    ));
    let report = bounded(fixture.region.drain(DEADLINE)).await??;
    check!(eq; report.accepted, 1);
    check!(eq; report.completed, 1);
    check!(!ran.load(Ordering::SeqCst));
    Ok(())
}

#[tokio::test]
async fn reserved_success_and_ordinary_error_drop_state_and_restore_every_unit() -> TestResult {
    let fixture = Fixture::new(2)?;
    let before = fixture.region.coordinator().remaining();
    let dropped = Arc::new(AtomicBool::new(false));
    let witness = Arc::clone(&dropped);
    let held = permit(&fixture.jobs).await?;
    let result = bounded(run_reserved(Arc::clone(&fixture.region), held, move || {
        let _population = DropWitness(witness);
        Ok::<_, JobError>(17)
    }))
    .await??;
    check!(eq; result, 17);
    check!(dropped.load(Ordering::SeqCst));
    check!(eq; fixture.region.coordinator().remaining(), before);
    dropped.store(false, Ordering::SeqCst);
    let witness = Arc::clone(&dropped);
    let held = permit(&fixture.jobs).await?;
    let result = bounded(run_reserved(Arc::clone(&fixture.region), held, move || {
        let _population = DropWitness(witness);
        Err::<(), _>(terminal("fixture publication refusal"))
    }))
    .await?;
    check!(
        matches!(result, Err(JobError::Terminal { message }) if message == "fixture publication refusal")
    );
    check!(dropped.load(Ordering::SeqCst));
    check!(eq; fixture.region.coordinator().remaining(), before);
    let report = bounded(fixture.region.drain(DEADLINE)).await??;
    check!(eq; report.accepted, 2);
    check!(eq; report.failed, 1);
    check!(eq; report.remaining, 0);
    Ok(())
}

#[tokio::test]
async fn reserved_unwind_drops_population_and_restores_every_unit_before_reap() -> TestResult {
    let fixture = Fixture::new(2)?;
    let before = fixture.region.coordinator().remaining();
    let held = permit(&fixture.jobs).await?;
    let dropped = Arc::new(AtomicBool::new(false));
    let witness = Arc::clone(&dropped);
    let result = bounded(run_reserved::<(), JobError, _>(
        Arc::clone(&fixture.region),
        held,
        move || {
            let _population = DropWitness(witness);
            std::panic::resume_unwind(Box::new("fixture publication unwind"))
        },
    ))
    .await?;
    check!(matches!(result, Err(JobError::Terminal { message }) if message == "job panicked"));
    check!(dropped.load(Ordering::SeqCst));
    check!(eq; fixture.region.coordinator().remaining(), before);
    let report = bounded(fixture.region.drain(DEADLINE)).await??;
    check!(eq; report.accepted, 1);
    check!(eq; report.panicked, 1);
    check!(eq; report.remaining, 0);
    Ok(())
}

#[tokio::test]
async fn closed_region_refuses_preadmitted_effect_and_releases_its_population() -> TestResult {
    let fixture = Fixture::new(2)?;
    let before = fixture.region.coordinator().remaining();
    let held = permit(&fixture.jobs).await?;
    bounded(fixture.region.drain(DEADLINE)).await??;
    check!(eq; fixture.region.coordinator().remaining(), charged(before)?);
    let ran = Arc::new(AtomicBool::new(false));
    let entered = Arc::clone(&ran);
    let result = bounded(run_reserved(Arc::clone(&fixture.region), held, move || {
        entered.store(true, Ordering::SeqCst);
        Ok::<_, JobError>(())
    }))
    .await?;
    check!(matches!(result, Err(JobError::Terminal { message }) if message == "job cancelled"));
    check!(!ran.load(Ordering::SeqCst));
    check!(eq; fixture.region.coordinator().remaining(), before);
    check!(bounded(fixture.jobs.permit_publication()).await?.is_err());
    Ok(())
}
