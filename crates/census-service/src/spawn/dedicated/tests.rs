use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const WAIT: Duration = Duration::from_secs(5);

struct ExitGate {
    entered: Option<oneshot::Sender<()>>,
    release: std::sync::mpsc::Receiver<()>,
    released: Arc<AtomicBool>,
}

impl Drop for ExitGate {
    fn drop(&mut self) {
        if let Some(entered) = self.entered.take() {
            let _ = entered.send(());
        }
        self.released
            .store(self.release.recv_timeout(WAIT).is_ok(), Ordering::Release);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn published_result_does_not_release_region_owned_thread_before_actual_exit() -> TestResult {
    let region = Spawner::with_capacity(1);
    let (entered, in_tail) = oneshot::channel();
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let exited = Arc::new(AtomicBool::new(false));
    let owner = ExitGate {
        entered: Some(entered),
        release: released,
        released: Arc::clone(&exited),
    };
    check!(eq; region.dedicated(owner, || Ok::<_, ()>(42)).await?, Outcome::Ok(42));
    tokio::time::timeout(WAIT, in_tail).await??;
    check!(eq; region.lock().threads.len(), 1);
    let cancelled_drain =
        tokio::time::timeout(Duration::from_millis(20), region.drain(Duration::ZERO)).await;
    check!(cancelled_drain.is_err());
    check!(eq; region.lock().threads.len(), 1);
    check!(!exited.load(Ordering::Acquire));
    check!(matches!(
        region.dedicated((), || Ok::<_, ()>(0)).await?,
        Outcome::Cancelled
    ));
    release.send(())?;
    tokio::time::timeout(WAIT, async {
        loop {
            {
                let mut owned = region.lock();
                owned.reap_finished();
                if owned.threads.is_empty() {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await?;
    let report = region.drain(WAIT).await?;
    check!(exited.load(Ordering::Acquire));
    check!(eq; report.accepted, 1);
    check!(eq; report.completed, 1);
    check!(eq; report.timed_out, 1);
    check!(eq; report.aborted, 0);
    check!(eq; report.remaining, 0);
    check!(eq; report.disjoint_total(), 1);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn dedicated_worker_survives_caller_cancellation_and_releases_lane_after_completion(
) -> TestResult {
    let region = Spawner::with_capacity(1);
    let (started, began) = oneshot::channel();
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let mut caller = Box::pin(region.dedicated((), move || {
        started.send(()).map_err(|()| "start receiver closed")?;
        released.recv_timeout(WAIT).map_err(|_| "release missing")?;
        Ok::<_, &str>(())
    }));
    tokio::select! {
        outcome = &mut caller => return Err(format!("worker finished early: {outcome:?}").into()),
        result = began => result?,
    }
    drop(caller);
    check!(eq; region.lock().threads.len(), 1);
    check!(eq; region.blocking.available_permits(), 0);
    release.send(())?;
    tokio::time::timeout(WAIT, async {
        loop {
            {
                let mut owned = region.lock();
                owned.reap_finished();
                if owned.threads.is_empty() {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await?;
    check!(eq; region.blocking.available_permits(), 1);
    check!(eq; region.blocking(|| Ok::<_, &str>(())).await, Outcome::Ok(()));
    let report = region.drain(WAIT).await?;
    check!(eq; report.completed, report.accepted);
    check!(eq; report.remaining, 0);
    check!(eq; report.panicked, 0);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn dedicated_domain_error_and_panic_have_distinct_region_accounting() -> TestResult {
    let failed = Spawner::with_capacity(1);
    check!(eq; failed.dedicated((), || Err::<(), _>("domain")).await?, Outcome::Err("domain"));
    let report = failed.drain(WAIT).await?;
    check!(eq; report.accepted, 1);
    check!(eq; report.completed, 1);
    check!(eq; report.failed, 1);
    check!(eq; report.panicked, 0);
    let panicked = Spawner::with_capacity(1);
    check!(matches!(
        panicked
            .dedicated((), || -> Result<(), ()> {
                panic!("dedicated panic fixture")
            })
            .await?,
        Outcome::Panicked
    ));
    let report = panicked.drain(WAIT).await?;
    check!(eq; report.accepted, 1);
    check!(eq; report.panicked, 1);
    check!(eq; report.failed, 0);
    check!(eq; report.disjoint_total(), 1);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn thread_start_failure_restores_lane_and_creates_no_accepted_unowned_work() -> TestResult {
    let region = Spawner::with_capacity(1);
    let permit = region.acquire_blocking().await.ok_or("worker lane")?;
    let ran = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&ran);
    let run = move || {
        let _permit = permit;
        observed.store(true, Ordering::Release);
    };
    let failed = region
        .push_dedicated_with(run, Arc::new(Counters::default()), |_run| {
            Err(std::io::Error::other("deterministic thread start failure"))
        })
        .await;
    check!(matches!(failed, Err(SpawnError::ThreadSpawn { .. })));
    check!(!ran.load(Ordering::Acquire));
    check!(eq; region.blocking.available_permits(), 1);
    check!(eq; region.lock().threads.len(), 0);
    check!(eq; region.drain(WAIT).await?.accepted, 0);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn mixed_async_and_dedicated_deadline_is_counted_once_across_drain_reentry() -> TestResult {
    let region = Spawner::with_capacity(2);
    region.spawn(std::future::pending())?;
    let (entered, in_tail) = oneshot::channel();
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let owner = ExitGate {
        entered: Some(entered),
        release: released,
        released: Arc::new(AtomicBool::new(false)),
    };
    check!(eq; region.dedicated(owner, || Ok::<_, ()>(())).await?, Outcome::Ok(()));
    tokio::time::timeout(WAIT, in_tail).await??;
    check!(
        tokio::time::timeout(Duration::from_millis(20), region.drain(Duration::ZERO))
            .await
            .is_err()
    );
    release.send(())?;
    let report = region.drain(WAIT).await?;
    check!(eq; report.accepted, 2);
    check!(eq; report.completed, 1);
    check!(eq; report.aborted, 1);
    check!(eq; report.timed_out, 2);
    check!(eq; report.remaining, 0);
    check!(eq; report.disjoint_total(), 2);
    Ok(())
}

#[test]
fn dedicated_worker_inherits_the_submitting_span_and_dispatcher() -> TestResult {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    tracing::subscriber::with_default(tracing_subscriber::registry(), || {
        let parent = tracing::info_span!("submitting_results");
        parent.in_scope(|| {
            runtime.block_on(async {
                let region = Spawner::with_capacity(1);
                let observed = region
                    .dedicated((), || {
                        Ok::<_, ()>(
                            tracing::Span::current()
                                .metadata()
                                .map(|metadata| metadata.name()),
                        )
                    })
                    .await?;
                check!(eq; observed, Outcome::Ok(Some("dedicated")));
                let report = region.drain(WAIT).await?;
                check!(eq; report.accepted, 1);
                check!(eq; report.completed, 1);
                Ok(())
            })
        })
    })
}
