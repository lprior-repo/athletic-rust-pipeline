use super::*;
use crate::actor::Actor;
use crate::clock::SystemClock;
use crate::lifecycle::{ActorJoinState, BrowserManager};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{mpsc, Mutex as AsyncMutex};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn settings() -> Result<BrowserSettings, url::ParseError> {
    Ok(BrowserSettings {
        cdp_endpoint: None,
        executable: PathBuf::from("/no-browser-required"),
        profile_dir: PathBuf::from("/no-browser-required/profile"),
        source_origin: url::Url::parse("https://www.athletic.net/")?,
        tabs: 1,
        request_timeout: std::time::Duration::from_secs(1),
        challenge_wait: std::time::Duration::from_secs(1),
        headed: false,
    })
}

#[tokio::test]
async fn dropping_readiness_before_owner_registration_cancels_without_profile_or_child_work(
) -> TestResult {
    let (startup, mut owner) = BrowserManager::prepare_launch(settings()?, Arc::new(SystemClock))?;
    let (_stop, stopping) = watch::channel(false);
    drop(startup);
    check!(matches!(owner.run(stopping).await, BrowserExit::Cancelled));
    check!(owner.child.is_none());
    check!(owner.manager.is_none());
    check!(matches!(owner.phase, Phase::Prepared(_)));
    check!(matches!(
        owner.finalize().await,
        BrowserCleanupAttempt::Reaped { report, failure: None }
        if report == DrainReport::default()
    ));
    Ok(())
}

struct OperationWitness(Arc<AtomicUsize>);

impl Drop for OperationWitness {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn dropped_run_polls_reenter_the_same_retained_connection_operation() -> TestResult {
    let (_startup, mut owner) = BrowserManager::prepare_connect(
        url::Url::parse("http://127.0.0.1:9222/")?,
        settings()?,
        Arc::new(SystemClock),
    )?;
    let drops = Arc::new(AtomicUsize::new(0));
    let witness = OperationWitness(Arc::clone(&drops));
    owner.phase = Phase::Connecting(
        async move {
            let _witness = witness;
            std::future::pending().await
        }
        .boxed(),
    );
    let (_stop, stopping) = watch::channel(false);
    for _ in 0..2 {
        let mut running = Box::pin(owner.run(stopping.clone()));
        check!(futures::poll!(running.as_mut()).is_pending());
        drop(running);
        check!(eq; drops.load(Ordering::SeqCst), 0);
    }
    check!(matches!(
        owner.finalize().await,
        BrowserCleanupAttempt::Reaped { report, failure: None }
        if report == DrainReport::default()
    ));
    check!(eq; drops.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn global_stopping_before_first_run_keeps_the_preparation_inert() -> TestResult {
    let (_startup, mut owner) = BrowserManager::prepare_launch(settings()?, Arc::new(SystemClock))?;
    let (_stop, stopping) = watch::channel(true);
    check!(matches!(owner.run(stopping).await, BrowserExit::Cancelled));
    check!(matches!(owner.phase, Phase::Prepared(_)));
    check!(owner.child.is_none());
    check!(matches!(
        owner.finalize().await,
        BrowserCleanupAttempt::Reaped { report, failure: None }
        if report == DrainReport::default()
    ));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn dropping_finalize_retains_the_actual_actor_and_handler_until_the_next_attempt(
) -> TestResult {
    let (_startup, mut owner) = BrowserManager::prepare_connect(
        url::Url::parse("http://127.0.0.1:9222/")?,
        settings()?,
        Arc::new(SystemClock),
    )?;
    let (finish, finished) = oneshot::channel();
    let handler = tokio::spawn(async move {
        if let Err(error) = finished.await {
            tracing::error!(%error, "test handler completion sender dropped");
        }
    });
    let (tx, rx) = mpsc::channel(2);
    let (reply, _report) = oneshot::channel();
    let actor = Actor::draining_for_test(rx, handler, reply)?;
    let manager = Arc::new(BrowserManager {
        tx,
        status: actor.status.clone(),
        cooldown_until: actor.cooldown_until.clone(),
        gate: actor.gate.clone(),
        clock: actor.clock.clone(),
        join: Arc::new(AsyncMutex::new(ActorJoinState::Running(tokio::spawn(
            actor.run(),
        )))),
    });
    owner.manager = Some(Arc::clone(&manager));
    owner.phase = Phase::Live;
    let mut cleanup = Box::pin(owner.finalize());
    check!(futures::poll!(cleanup.as_mut()).is_pending());
    drop(cleanup);
    check!(matches!(
        &*manager.join.lock().await,
        ActorJoinState::Running(_)
    ));
    finish.send(()).map_err(|_| "the actual handler was lost")?;
    let result = owner.finalize().await;
    check!(matches!(
        result,
        BrowserCleanupAttempt::Reaped { report, failure: None }
        if report == DrainReport::default()
    ));
    check!(matches!(
        &*manager.join.lock().await,
        ActorJoinState::Finished(_)
    ));
    check!(matches!(
        owner.finalize().await,
        BrowserCleanupAttempt::Reaped { report, failure: None }
        if report == DrainReport::default()
    ));
    Ok(())
}
