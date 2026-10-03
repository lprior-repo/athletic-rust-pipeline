use super::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn cancelling_drain_keeps_started_effect_owned_for_the_next_drain() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
    let spawner = Spawner::new();
    let (started, began) = oneshot::channel();
    let (release, released) = std::sync::mpsc::channel();
    let caller = spawner.blocking(move || {
        let _ = started.send(());
        released.recv().map(|()| 7)
    });
    tokio::pin!(caller);
    tokio::select! {
        outcome = &mut caller => return Err(format!("effect finished before release: {outcome:?}").into()),
        result = began => result?,
    }
    let cancelled =
        tokio::time::timeout(Duration::from_millis(20), spawner.drain(Duration::ZERO)).await;
    check!(cancelled.is_err());
    check!(matches!(
        spawner.spawn(async {}),
        Err(SpawnError::RegionClosed)
    ));
    release.send(())?;
    let (outcome, report) = tokio::join!(caller, spawner.drain(Duration::from_secs(5)));
    check!(eq; outcome, Outcome::Ok(7));
    let report = report?;
    check!(eq; report.accepted, 1);
    check!(eq; report.completed, 1);
    check!(eq; report.remaining, 0);
    check!(eq; report.timed_out, 1);
    check!(eq; report.aborted, 0);
    Ok(())
        })
}

#[test]
fn shutdown_wakes_waiting_admission_without_running_its_effect() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
    let spawner = Spawner::with_capacity(1);
    spawner.spawn(std::future::pending())?;
    let ran = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed = Arc::clone(&ran);
    let waiting = spawner.blocking(move || {
        observed.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok::<_, ()>(())
    });
    tokio::pin!(waiting);
    tokio::select! {
        biased;
        outcome = &mut waiting => return Err(format!("full region admitted effect: {outcome:?}").into()),
        () = std::future::ready(()) => {}
    }
    let (outcome, report) = tokio::join!(waiting, spawner.drain(Duration::ZERO));
    check!(eq; outcome, Outcome::Cancelled);
    check!(!ran.load(std::sync::atomic::Ordering::SeqCst));
    let report = report?;
    check!(eq; report.accepted, 1);
    check!(eq; report.aborted, 1);
    check!(eq; report.remaining, 0);
    Ok(())
        })
}
