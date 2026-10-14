use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const WAIT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy)]
enum WorkerPath {
    Blocking,
    Dedicated,
}

#[derive(Clone, Copy)]
enum TailCaller {
    Waiting,
    Cancelled,
}

#[tokio::test(flavor = "current_thread")]
async fn capacity_one_async_parent_can_await_its_blocking_child() -> TestResult {
    parent_child(WorkerPath::Blocking).await
}

#[tokio::test(flavor = "current_thread")]
async fn capacity_one_async_parent_can_await_its_dedicated_child() -> TestResult {
    parent_child(WorkerPath::Dedicated).await
}

async fn parent_child(path: WorkerPath) -> TestResult {
    let region = Arc::new(Spawner::with_capacity(1));
    let worker = Arc::clone(&region);
    let (finished, done) = oneshot::channel();
    let (started, began) = oneshot::channel();
    let (release, released) = std::sync::mpsc::sync_channel(1);
    region.spawn_with(WORK_CLASS_STAGE, move || async move {
        let job = move || {
            started.send(()).map_err(|()| "start receiver closed")?;
            released
                .recv_timeout(WAIT)
                .map_err(|_| "child release missing")?;
            Ok::<_, &str>(7)
        };
        let outcome = match path {
            WorkerPath::Blocking => Ok(worker.blocking(job).await),
            WorkerPath::Dedicated => worker.dedicated((), job).await,
        };
        assert!(finished.send(outcome).is_ok());
    })?;
    tokio::time::timeout(WAIT, began).await??;
    {
        let owned = region.lock();
        check!(eq; owned.pending()?, 2);
        check!(!owned.worker_has_room(1)?);
        check!(matches!(
            owned.require_async_room(1),
            Err(SpawnError::RegionFull { capacity: 1 })
        ));
    }
    release.send(())?;
    check!(eq; tokio::time::timeout(WAIT, done).await???, Outcome::Ok(7));
    let report = region.drain(WAIT).await?;
    check!(eq; report.accepted, 2);
    check!(eq; report.completed, 2);
    check!(eq; report.remaining, 0);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn async_descriptor_refusal_never_evaluates_the_build_closure() -> TestResult {
    let region = Spawner::with_capacity(1);
    region.spawn_with(WORK_CLASS_CONTROL, std::future::pending)?;
    let built = AtomicBool::new(false);
    let refused = region.spawn_with(WORK_CLASS_STAGE, || {
        built.store(true, Ordering::Release);
        std::future::pending()
    });
    check!(matches!(
        refused,
        Err(SpawnError::RegionFull { capacity: 1 })
    ));
    check!(!built.load(Ordering::Acquire));
    let report = region.drain(Duration::ZERO).await?;
    check!(eq; report.accepted, 1);
    check!(eq; report.aborted, 1);
    check!(eq; report.remaining, 0);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn blocking_follower_queues_after_lane_release_until_owned_tail_is_reaped() -> TestResult {
    retiring_worker(WorkerPath::Blocking, TailCaller::Waiting).await
}

#[tokio::test(flavor = "current_thread")]
async fn dedicated_follower_queues_after_lane_release_until_owned_tail_is_reaped() -> TestResult {
    retiring_worker(WorkerPath::Dedicated, TailCaller::Waiting).await
}

#[tokio::test(flavor = "current_thread")]
async fn cancelling_blocking_descriptor_wait_returns_the_unstarted_lane() -> TestResult {
    retiring_worker(WorkerPath::Blocking, TailCaller::Cancelled).await
}

#[tokio::test(flavor = "current_thread")]
async fn cancelling_dedicated_descriptor_wait_returns_the_unstarted_lane() -> TestResult {
    retiring_worker(WorkerPath::Dedicated, TailCaller::Cancelled).await
}

async fn retiring_worker(path: WorkerPath, caller: TailCaller) -> TestResult {
    let region = Spawner::with_capacity(1);
    let permit = region.acquire_blocking().await.ok_or("worker lane")?;
    let (retired, in_tail) = oneshot::channel();
    let (release, released) = std::sync::mpsc::sync_channel(1);
    {
        let mut owned = region.lock();
        let counters = Arc::new(Counters::default());
        let witness = Arc::clone(&counters);
        let task = owned.tasks.spawn_blocking(move || {
            counters.record_started();
            drop(permit);
            assert!(retired.send(()).is_ok());
            assert!(released.recv_timeout(WAIT).is_ok());
        });
        owned.ledger.track_blocking(task.id(), witness);
        owned.ledger.accept();
    }
    tokio::time::timeout(WAIT, in_tail).await??;
    check!(eq; region.blocking.available_permits(), 1);
    let ran = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&ran);
    let mut follower = Box::pin(async {
        match path {
            WorkerPath::Blocking => Ok(region
                .blocking(move || {
                    observed.store(true, Ordering::Release);
                    Ok::<_, ()>(9)
                })
                .await),
            WorkerPath::Dedicated => {
                region
                    .dedicated((), move || {
                        observed.store(true, Ordering::Release);
                        Ok::<_, ()>(9)
                    })
                    .await
            }
        }
    });
    check!(futures::poll!(follower.as_mut()).is_pending());
    check!(!ran.load(Ordering::Acquire));
    check!(eq; region.lock().pending()?, 1);
    check!(eq; region.blocking.available_permits(), 0);
    let accepted = match caller {
        TailCaller::Waiting => {
            release.send(())?;
            check!(eq; tokio::time::timeout(WAIT, follower).await??, Outcome::Ok(9));
            check!(ran.load(Ordering::Acquire));
            2
        }
        TailCaller::Cancelled => {
            drop(follower);
            check!(eq; region.blocking.available_permits(), 1);
            check!(!ran.load(Ordering::Acquire));
            release.send(())?;
            1
        }
    };
    let report = region.drain(WAIT).await?;
    check!(eq; report.accepted, accepted);
    check!(eq; report.completed, accepted);
    check!(eq; report.remaining, 0);
    Ok(())
}
