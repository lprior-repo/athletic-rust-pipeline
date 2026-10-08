use std::error::Error;
use std::sync::{mpsc::SyncSender, Arc};
use std::time::Duration;

use tokio::sync::oneshot;

use super::super::marker_worker;
use super::control::{completed, runtime, Runtime};
use super::{fixture, read_marker, reserved_marker, BoundaryError, TestResult, OPERATION};
use crate::restate_services::{blocking, JobError};

const DEADLINE: Duration = Duration::from_secs(5);

struct Suspended {
    release: SyncSender<()>,
    finished: oneshot::Receiver<Result<(), BoundaryError>>,
}

async fn cancel_marker_caller(
    runtime: &Runtime,
    directory: super::super::Directory,
) -> Result<Suspended, Box<dyn Error>> {
    let admission = Arc::new(
        runtime
            .source
            .admit(OPERATION)
            .await
            .map_err(super::terminal)?,
    );
    let worker = marker_worker(
        admission,
        Arc::clone(runtime.jobs.store()),
        directory,
        reserved_marker(1)?,
        Duration::ZERO,
    );
    let (started, began) = oneshot::channel();
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let (finished, completion) = oneshot::channel();
    let mut caller = Box::pin(blocking(Arc::clone(&runtime.region), move || {
        started.send(()).map_err(|()| JobError::Terminal {
            message: "marker start observer closed".to_string(),
        })?;
        released
            .recv_timeout(DEADLINE)
            .map_err(|error| JobError::Terminal {
                message: format!("marker release failed: {error}"),
            })?;
        let outcome = worker();
        finished.send(outcome).map_err(|_| JobError::Terminal {
            message: "marker completion observer closed".to_string(),
        })?;
        Ok::<(), JobError>(())
    }));
    tokio::time::timeout(DEADLINE, async {
        tokio::select! {
            result = &mut caller => Err::<(), Box<dyn Error>>(format!("marker worker completed before release: {result:?}").into()),
            result = began => { result?; Ok(()) },
        }
    }).await??;
    drop(caller);
    Ok(Suspended {
        release,
        finished: completion,
    })
}

#[test]
fn cancelled_caller_cannot_release_source_or_capacity_while_marker_worker_survives() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let runtime = runtime()?;
            let fixture = fixture(1, 60)?;
            let suspended = cancel_marker_caller(&runtime, fixture.armed()?.directory).await?;
            check!(eq; runtime.load.available_permits(), 1);
            check!(eq; std::fs::symlink_metadata(fixture.marker())
    .err()
    .map(|error| error.kind()),
Some(std::io::ErrorKind::NotFound));
            let mut waiting = Box::pin(runtime.source.admit(OPERATION));
            check!(futures::poll!(waiting.as_mut()).is_pending());
            check!(eq; runtime.load.available_permits(), 0);
            suspended.release.send(())?;
            let outcome = tokio::time::timeout(DEADLINE, suspended.finished).await??;
            check!(matches!(outcome, Err(BoundaryError::HoldExpired { operation, attempt: 1, seconds: 0 }) if operation == OPERATION),
                "the owned native marker worker must fail closed after its hold deadline");
            let admission = tokio::time::timeout(DEADLINE, waiting)
                .await?
                .map_err(super::terminal)?;
            let marker = read_marker(&fixture)?;
            check!(eq; marker.get("operation").and_then(serde_json::Value::as_str), Some(OPERATION));
            check!(eq; serde_json::from_value::<census_store::NativeEffectCheckpoint>(
                marker.get("acknowledged_effects").ok_or("original source checkpoint missing after caller cancellation")?.clone())?,
                runtime.jobs.store().native_effect_checkpoint()?);
            check!(eq; runtime.load.available_permits(), 1);
            drop(admission);
            check!(eq; runtime.load.available_permits(), 2);
            check!(eq; runtime.region.drain(DEADLINE).await?, completed(1));
            Ok(())
        })
}
