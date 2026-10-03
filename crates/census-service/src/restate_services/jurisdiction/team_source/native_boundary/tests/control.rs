use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use census_store::{clock::SystemClock, Store};
use tokio::sync::Semaphore;

use super::super::{hold, wait_configured};
use super::{
    expected_marker, fixture, identity, read_marker, write_new, BoundaryError, TestResult,
    OPERATION,
};
use crate::restate_services::jurisdiction::{team_source::TeamsSource, JurisdictionCensus};
use crate::restate_services::{JobError, Jobs};
use crate::spawn::{Spawner, TaskReport};

pub(super) struct Runtime {
    _root: tempfile::TempDir,
    pub(super) source: TeamsSource,
    pub(super) jobs: Jobs,
    pub(super) region: Arc<Spawner>,
    pub(super) load: Arc<Semaphore>,
}

pub(super) fn runtime() -> Result<Runtime, Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let region = Arc::new(Spawner::with_capacity(2));
    let load = Arc::new(Semaphore::new(2));
    let owner = JurisdictionCensus::new(Arc::clone(&store), Arc::new(SystemClock), None);
    let jobs = Jobs::new(store, Arc::clone(&load), Arc::clone(&region));
    let source = TeamsSource::new(owner, jobs.clone());
    Ok(Runtime {
        _root: root,
        source,
        jobs,
        region,
        load,
    })
}

#[test]
fn unrelated_operations_and_later_reserved_attempts_ignore_owned_marker_artifacts() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let runtime = runtime()?;
            let fixture = fixture(1, 60)?;
            write_new(&fixture.marker(), b"retained unrelated authority")?;
            write_new(&fixture.pending(), b"retained unrelated pending")?;
            for (operation, attempt) in [
                ("jurisdiction:RI:2026-27:1/teams/riil", 1),
                ("jurisdiction:RI:2026-27:2/teams/milesplit", 1),
                (OPERATION, 2),
                (OPERATION, 3),
            ] {
                let admission = Arc::new(
                    runtime
                        .source
                        .admit(operation)
                        .await
                        .map_err(super::terminal)?,
                );
                wait_configured(
                    &runtime.jobs,
                    admission,
                    fixture.config.clone(),
                    operation,
                    attempt,
                    &identity(),
                )
                .await?;
                check!(eq; runtime.load.available_permits(), 2);
                check!(eq; std::fs::read(fixture.marker())?,
    b"retained unrelated authority");
                check!(eq; std::fs::read(fixture.pending())?,
    b"retained unrelated pending");
            }
            let report = runtime.region.drain(Duration::from_secs(5)).await?;
            check!(eq; report.remaining, 0);
            check!(eq; report.panicked, 0);
            Ok(())
        })
}

#[test]
fn exact_reserved_attempt_publishes_original_identity_before_failed_hold_expiry() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()?
        .block_on(async {
            let runtime = runtime()?;
            let fixture = fixture(1, 60)?;
            let admission = Arc::new(
                runtime
                    .source
                    .admit(OPERATION)
                    .await
                    .map_err(super::terminal)?,
            );
            let began = tokio::time::Instant::now();
            let result = wait_configured(
                &runtime.jobs,
                admission,
                fixture.config.clone(),
                OPERATION,
                1,
                &identity(),
            )
            .await;
            check!(matches!(result, Err(JobError::Terminal { .. })));
            check!(eq; began.elapsed(), Duration::from_secs(60));
            check!(eq; read_marker(&fixture)?, expected_marker(1));
            check!(eq; std::fs::symlink_metadata(fixture.pending())
    .err()
    .map(|error| error.kind()),
Some(std::io::ErrorKind::NotFound));
            check!(eq; runtime.load.available_permits(), 2);
            check!(eq; runtime.region.drain(Duration::from_secs(5)).await?,
completed(2));
            Ok(())
        })
}

#[test]
fn hold_remains_pending_until_exact_deadline_and_never_returns_success() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().start_paused(true).build()?.block_on(async { let mut holding = Box::pin(hold(OPERATION, 3, Duration::from_secs(60)));
check!(futures::poll!(holding.as_mut()).is_pending());
tokio::time::advance(Duration::from_secs(59)).await;
check!(futures::poll!(holding.as_mut()).is_pending());
tokio::time::advance(Duration::from_secs(1)).await;
check!(matches!(holding.await, Err(BoundaryError::HoldExpired { operation, attempt: 3, seconds: 60 }) if operation == OPERATION));
Ok(()) })
}

#[test]
fn malformed_present_config_fails_even_for_an_unrelated_source_without_publishing() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let runtime = runtime()?;
            let fixture = super::Fixture::new(b"{}")?;
            let admission = Arc::new(
                runtime
                    .source
                    .admit(OPERATION)
                    .await
                    .map_err(super::terminal)?,
            );
            let result = wait_configured(
                &runtime.jobs,
                admission,
                fixture.config.clone(),
                "unrelated-source",
                2,
                &identity(),
            )
            .await;
            check!(matches!(result, Err(JobError::Terminal { .. })));
            check!(eq; std::fs::symlink_metadata(fixture.marker())
    .err()
    .map(|error| error.kind()),
Some(std::io::ErrorKind::NotFound));
            check!(eq; runtime.load.available_permits(), 2);
            check!(eq; runtime.region.drain(Duration::from_secs(5)).await?,
completed(1));
            Ok(())
        })
}

pub(super) fn completed(count: u64) -> TaskReport {
    TaskReport {
        accepted: count,
        completed: count,
        cancelled: 0,
        timed_out: 0,
        remaining: 0,
        aborted: 0,
        panicked: 0,
    }
}
