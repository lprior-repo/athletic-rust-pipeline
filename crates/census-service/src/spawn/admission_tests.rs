use std::sync::atomic::AtomicBool;
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

#[test]
fn preadmitted_async_push_is_rejected_during_active_drain() -> TestResult {
    run_admission_race(SpawnPath::Async, DrainCaller::Active)
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
        let slot = match path {
            SpawnPath::Async => spawner.admit()?,
            SpawnPath::Blocking => runtime
                .block_on(Arc::clone(&spawner.permits).acquire_owned())
                .map_err(|_| SpawnError::RegionClosed)?,
        };
        assert!(acquired.send(()).is_ok());
        barrier.wait();
        match path {
            SpawnPath::Async => {
                match spawner.push_async(async move { ran.store(true, Ordering::SeqCst) }, slot) {
                    Ok(()) => Ok(true),
                    Err(SpawnError::RegionClosed) => Ok(false),
                    Err(error) => Err(error),
                }
            }
            SpawnPath::Blocking => {
                Ok(spawner.push_blocking(move || ran.store(true, Ordering::SeqCst), slot))
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
                assert!(started.send(()).is_ok());
                released.recv().map(|()| 7)
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
            check!(eq; outcome, Outcome::Ok(7));
            check!(eq; report?, TaskReport {
                accepted: 1,
                completed: 1,
                timed_out: 1,
                ..TaskReport::default()
            });
            Ok(())
        })
}
