use super::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn cancelling_drain_keeps_started_effect_owned_for_the_next_drain() -> TestResult {
    let spawner = Spawner::new();
    let (started, began) = oneshot::channel();
    let (release, released) = std::sync::mpsc::channel();
    let caller = spawner.blocking(move || {
        let _ = started.send(());
        released.recv().map(|()| 7)
    });
    tokio::pin!(caller);
    tokio::select! {
        outcome = &mut caller => panic!("effect finished before release: {outcome:?}"),
        result = began => result?,
    }
    let cancelled =
        tokio::time::timeout(Duration::from_millis(20), spawner.drain(Duration::ZERO)).await;
    assert!(cancelled.is_err());
    assert!(matches!(
        spawner.spawn(async {}),
        Err(SpawnError::RegionClosed)
    ));
    release.send(())?;
    let (outcome, report) = tokio::join!(caller, spawner.drain(Duration::from_secs(5)));
    assert_eq!(outcome, Outcome::Ok(7));
    let report = report?;
    assert_eq!(report.accepted, 1);
    assert_eq!(report.completed, 1);
    assert_eq!(report.remaining, 0);
    assert_eq!(report.timed_out, 1);
    assert_eq!(report.aborted, 0);
    Ok(())
}

#[tokio::test]
async fn shutdown_wakes_waiting_admission_without_running_its_effect() -> TestResult {
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
        outcome = &mut waiting => panic!("full region admitted effect: {outcome:?}"),
        () = std::future::ready(()) => {}
    }
    let (outcome, report) = tokio::join!(waiting, spawner.drain(Duration::ZERO));
    assert_eq!(outcome, Outcome::Cancelled);
    assert!(!ran.load(std::sync::atomic::Ordering::SeqCst));
    let report = report?;
    assert_eq!(report.accepted, 1);
    assert_eq!(report.aborted, 1);
    assert_eq!(report.remaining, 0);
    Ok(())
}
