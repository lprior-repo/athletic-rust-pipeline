use super::*;
use crate::restate_services::JobError;
use crate::spawn::Spawner;
use census_store::Store;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const WAIT: Duration = Duration::from_secs(5);

#[tokio::test(flavor = "current_thread")]
async fn cancelled_results_caller_keeps_original_owner_fenced_until_worker_drains() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let region = Arc::new(Spawner::with_capacity(2));
    let jobs = Jobs::new(Arc::clone(&store), Arc::clone(&region));
    let shared = jobs.clone();
    let original_key = "jurisdiction:MI:2026-27:1";
    let admission = jobs
        .admit_work(original_key)
        .await
        .map_err(|error| error.to_string())?;
    let (started, began) = tokio::sync::oneshot::channel();
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker_region = Arc::clone(&region);
    let mut caller = Box::pin(async move {
        worker_region
            .dedicated(admission, move || {
                started.send(()).map_err(|()| JobError::Terminal {
                    message: "start receiver closed".into(),
                })?;
                released
                    .recv_timeout(WAIT)
                    .map_err(|error| JobError::Terminal {
                        message: error.to_string(),
                    })?;
                Ok::<_, JobError>(())
            })
            .await
    });
    tokio::time::timeout(WAIT, async {
        tokio::select! {
            outcome = &mut caller => Err::<(), Box<dyn std::error::Error>>(format!("worker returned early: {outcome:?}").into()),
            outcome = began => { outcome?; Ok(()) },
        }
    }).await??;
    drop(caller);
    let mut next = Box::pin(shared.admit_work(original_key));
    check!(futures::poll!(next.as_mut()).is_pending());
    release.send(())?;
    let next = tokio::time::timeout(WAIT, next)
        .await?
        .map_err(|error| error.to_string())?;
    drop(next);
    let report = region.drain(WAIT).await?;
    check!(eq; report.accepted, 1);
    check!(eq; report.completed, 1);
    check!(eq; report.remaining, 0);
    check!(eq; report.panicked, 0);
    check!(eq; region.coordinator().remaining().slots.stages, 2);
    Ok(())
}
