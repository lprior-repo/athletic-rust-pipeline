use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Barrier;
use std::time::Duration;

use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy)]
enum SpawnPath {
    Async,
    Blocking,
}

#[derive(Clone, Copy)]
enum DrainCaller {
    Active,
    Cancelled,
}

enum PreAdmitted {
    Async(Reservation),
    Blocking(tokio::sync::OwnedSemaphorePermit),
}

#[test]
fn preadmitted_async_push_is_rejected_during_active_drain() -> TestResult {
    run_admission_race(SpawnPath::Async, DrainCaller::Active)
}

#[tokio::test]
async fn a_saturated_blocking_lane_defers_the_next_effect_until_the_holder_releases() -> TestResult
{
    let spawner = Spawner::with_capacity(1);
    check!(eq; spawner.blocking.available_permits(), 1);
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));
    let (release, released) = oneshot::channel();
    let (attempted, attempt) = oneshot::channel();
    let holder = {
        let order = Arc::clone(&order);
        spawner.blocking(move || {
            order
                .lock()
                .map_err(|_| "log poisoned".to_string())?
                .push(1);
            released
                .blocking_recv()
                .map_err(|_| "the holder lost its release".to_string())?;
            Ok::<_, String>(1)
        })
    };
    let follower = {
        let order = Arc::clone(&order);
        let spawner = &spawner;
        async move {
            if attempted.send(()).is_err() {
                return Err("nobody awaited the attempt".to_string());
            }
            let outcome = spawner
                .blocking(move || {
                    order
                        .lock()
                        .map_err(|_| "log poisoned".to_string())?
                        .push(2);
                    Ok::<_, String>(2)
                })
                .await;
            Ok::<_, String>(outcome)
        }
    };
    let driver = {
        let spawner = &spawner;
        async move {
            if attempt.await.is_err() {
                return Err("the follower never attempted".to_string());
            }
            if spawner.blocking.available_permits() != 0 {
                return Err("a held lane still offered permits".to_string());
            }
            release
                .send(())
                .map_err(|_| "the holder vanished".to_string())?;
            Ok(())
        }
    };
    let (held, queued, driven) = tokio::join!(holder, follower, driver);
    check!(eq; driven, Ok(()));
    check!(eq; held, crate::outcome::Outcome::Ok(1));
    check!(eq; queued, Ok(crate::outcome::Outcome::Ok(2)));
    check!(eq; *order.lock().map_err(|_| "log poisoned")?, vec![1, 2]);
    check!(eq; spawner.blocking.available_permits(), 1);
    Ok(())
}

#[test]
fn preadmitted_blocking_push_is_rejected_during_active_drain() -> TestResult {
    run_admission_race(SpawnPath::Blocking, DrainCaller::Active)
}

#[test]
fn cancelled_drain_restores_closed_async_admission_and_exact_ledger() -> TestResult {
    run_admission_race(SpawnPath::Async, DrainCaller::Cancelled)
}

#[test]
fn cancelled_drain_restores_closed_blocking_admission_and_exact_ledger() -> TestResult {
    run_admission_race(SpawnPath::Blocking, DrainCaller::Cancelled)
}

fn run_admission_race(path: SpawnPath, caller: DrainCaller) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(admission_race(path, caller))
}

async fn admission_race(path: SpawnPath, caller: DrainCaller) -> TestResult {
    let spawner = Arc::new(Spawner::with_capacity(4));
    let (finished, finish) = oneshot::channel();
    spawner.spawn(async move {
        assert!(finished.send(()).is_ok());
    })?;
    finish.await?;
    let (release, released) = oneshot::channel();
    spawner.spawn(async move {
        assert!(released.await.is_ok());
    })?;
    let barrier = Arc::new(Barrier::new(2));
    let ran = Arc::new(AtomicBool::new(false));
    let (acquired, ready) = oneshot::channel();
    let worker = reserve_then_push(
        Arc::clone(&spawner),
        path,
        Arc::clone(&barrier),
        Arc::clone(&ran),
        acquired,
    );
    ready.await?;
    let mut draining = Box::pin(spawner.drain(Duration::from_secs(5)));
    tokio::select! {
        biased;
        result = &mut draining => return Err(format!("drain finished before release: {result:?}").into()),
        () = std::future::ready(()) => {}
    }
    let report = match caller {
        DrainCaller::Active => {
            barrier.wait();
            assert_rejected(worker, &ran)?;
            release
                .send(())
                .map_err(|_| "owned task lost its release receiver")?;
            draining.await?
        }
        DrainCaller::Cancelled => {
            drop(draining);
            barrier.wait();
            assert_rejected(worker, &ran)?;
            release
                .send(())
                .map_err(|_| "cancelled drain dropped its owned task")?;
            spawner.drain(Duration::from_secs(5)).await?
        }
    };
    check!(eq; report, TaskReport { accepted: 2, completed: 2, ..TaskReport::default() });
    check!(eq; spawner.drain(Duration::ZERO).await?, TaskReport::default());
    Ok(())
}

fn reserve_then_push(
    spawner: Arc<Spawner>,
    path: SpawnPath,
    barrier: Arc<Barrier>,
    ran: Arc<AtomicBool>,
    acquired: oneshot::Sender<()>,
) -> std::thread::JoinHandle<Result<bool, SpawnError>> {
    let runtime = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        let _entered = runtime.enter();
        let preadmitted = match path {
            SpawnPath::Async => PreAdmitted::Async(spawner.admit()?),
            SpawnPath::Blocking => PreAdmitted::Blocking(
                runtime
                    .block_on(spawner.acquire_blocking())
                    .ok_or(SpawnError::RegionClosed)?,
            ),
        };
        assert!(acquired.send(()).is_ok());
        barrier.wait();
        match preadmitted {
            PreAdmitted::Async(slot) => {
                match spawner.push_async(
                    async move { ran.store(true, Ordering::SeqCst) },
                    slot,
                    None,
                ) {
                    Ok(()) => Ok(true),
                    Err(SpawnError::RegionClosed) => Ok(false),
                    Err(error) => Err(error),
                }
            }
            PreAdmitted::Blocking(permit) => {
                Ok(spawner
                    .push_blocking(move |_counters| ran.store(true, Ordering::SeqCst), permit))
            }
        }
    })
}

fn assert_rejected(
    worker: std::thread::JoinHandle<Result<bool, SpawnError>>,
    ran: &AtomicBool,
) -> TestResult {
    let accepted = worker.join().map_err(|_| "preadmitted caller panicked")??;
    check!(!accepted, "a preadmitted task escaped the draining region");
    check!(!ran.load(Ordering::SeqCst), "a rejected effect ran");
    Ok(())
}

#[test]
fn cancelling_deadline_reap_keeps_started_blocking_effect_and_certificate() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()?
        .block_on(async {
            let spawner = Spawner::new();
            let (started, began) = oneshot::channel();
            let (release, released) = std::sync::mpsc::channel();
            let mut caller = Box::pin(spawner.blocking(move || {
                let delivered = started.send(()).is_ok();
                released.recv().map(|()| (delivered, 7))
            }));
            tokio::select! {
                outcome = &mut caller => return Err(format!("effect finished before release: {outcome:?}").into()),
                result = began => result?,
            }
            let grace = Duration::from_secs(1);
            let mut draining = Box::pin(spawner.drain(grace));
            tokio::select! {
                biased;
                report = &mut draining => return Err(format!("started effect escaped drain: {report:?}").into()),
                () = std::future::ready(()) => {}
            }
            tokio::time::advance(grace).await;
            tokio::select! {
                biased;
                report = &mut draining => return Err(format!("started effect escaped deadline reap: {report:?}").into()),
                () = std::future::ready(()) => {}
            }
            drop(draining);
            release.send(())?;
            let (outcome, report) = tokio::join!(caller, spawner.drain(Duration::from_secs(5)));
            check!(eq; outcome, Outcome::Ok((true, 7)), "the started signal was delivered");
            check!(eq; report?, TaskReport {
                accepted: 1,
                completed: 1,
                timed_out: 1,
                still_running: 0,
                ..TaskReport::default()
            });
            Ok(())
        })
}
