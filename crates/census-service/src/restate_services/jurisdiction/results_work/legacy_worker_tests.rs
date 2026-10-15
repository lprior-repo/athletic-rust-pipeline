use super::*;
use crate::restate_services::jurisdiction::capture_admission::ResultsCaptureExecution;
use crate::restate_services::wire::HistoryWindow;
use crate::spawn::{Spawner, TaskReport};
use census_domain::UsJurisdiction;
use census_store::Store;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const WAIT: Duration = Duration::from_secs(5);

fn completion() -> Json<ResultsStageOutcome> {
    Json(ResultsStageOutcome::default())
}

fn require_one_completed(report: TaskReport) -> TestResult {
    check!(eq; report, TaskReport {
        accepted: 1, completed: 1, cancelled: 0, aborted: 0, panicked: 0,
        failed: 0, still_running: 0, timed_out: 0, remaining: 0,
    });
    check!(eq; report.accepted, report.disjoint_total());
    Ok(())
}

#[test]
fn legacy_results_selector_and_dns_finish_with_capacity_one() -> TestResult {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .max_blocking_threads(1)
        .enable_all()
        .build()?;
    let result = runtime.block_on(async {
        let root = tempfile::tempdir()?;
        let store = Arc::new(Store::open(root.path())?);
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let url = format!("http://localhost:{}/capture", listener.local_addr()?.port());
        let mut server = tokio::task::JoinSet::new();
        server.spawn(async move {
            let (mut socket, _) = listener.accept().await?;
            let mut request = [0_u8; 4096];
            if socket.read(&mut request).await? == 0 {
                return Err(std::io::Error::other("request missing"));
            }
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                .await
        });
        let fetcher = Arc::new(Fetcher::new(
            store.http_cache_dir(),
            None,
            Duration::ZERO,
            std::collections::HashMap::new(),
            vec!["localhost".into()],
        )?);
        let observed = Arc::clone(&fetcher);
        let scope = HistoricalStageScope {
            jurisdiction: UsJurisdiction::Wisconsin,
            year: 2026,
            refresh: false,
            window: HistoryWindow::new(2023, 2026, "2026-10-07")?,
            observed_on: chrono::NaiveDate::parse_from_str("2026-10-08", "%Y-%m-%d")?,
        };
        let selected =
            SourceSelection::parse(&[]).map_err(crate::restate_services::tests::sdk_error)?;
        let handle = tokio::runtime::Handle::current();
        let region = Spawner::with_capacity(1);
        let work = legacy_results_work(&region, move || {
            handle.block_on(async move {
                let outcome = jobs::results_stage(
                    store,
                    Arc::clone(&fetcher),
                    scope,
                    selected,
                    ResultsCaptureExecution::LegacyFresh,
                )
                .await?;
                let captured = fetcher
                    .get(&url, &census_crawl::net::FetchOptions::default())
                    .await
                    .map_err(|error| jobs::invariant(&error.to_string()))?;
                if captured.body != b"ok" {
                    return Err(jobs::invariant("resolver request body mismatch"));
                }
                Ok(outcome)
            })
        });
        let Json(outcome) = tokio::time::timeout(WAIT, work)
            .await?
            .map_err(crate::restate_services::tests::sdk_error)?;
        check!(outcome.per_source.is_empty());
        check!(outcome.pending.is_empty());
        check!(outcome.required_sources.is_empty());
        check!(eq; observed.stats().await.physical_requests(), 1);
        tokio::time::timeout(WAIT, server.join_next())
            .await?
            .ok_or("HTTP fixture missing")???;
        require_one_completed(region.drain(WAIT).await?)
    });
    runtime.shutdown_timeout(WAIT);
    result
}

#[tokio::test(flavor = "current_thread")]
async fn cancelled_legacy_caller_keeps_started_write_owned_until_drain() -> TestResult {
    let region = Spawner::with_capacity(1);
    let root = tempfile::tempdir()?;
    let path = root.path().join("retained");
    let written = path.clone();
    let (started, began) = tokio::sync::oneshot::channel();
    let (release, released) = std::sync::mpsc::channel();
    let mut caller = Box::pin(legacy_results_work(&region, move || {
        started
            .send(())
            .map_err(|_| jobs::invariant("start observer missing"))?;
        released
            .recv_timeout(WAIT)
            .map_err(|error| jobs::invariant(&error.to_string()))?;
        std::fs::write(&written, b"retained capture")
            .map_err(|error| jobs::invariant(&error.to_string()))?;
        std::fs::File::open(&written)
            .and_then(|file| file.sync_all())
            .map_err(|error| jobs::invariant(&error.to_string()))?;
        Ok(completion())
    }));
    tokio::select! {
        outcome = &mut caller => return Err(format!("write completed before release: {:?}", outcome.map(|Json(value)| value)).into()),
        result = tokio::time::timeout(WAIT, began) => result??,
    }
    drop(caller);
    check!(!path.exists());
    let ran = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&ran);
    let mut waiting = Box::pin(legacy_results_work(&region, move || {
        observed.store(true, Ordering::SeqCst);
        Ok(completion())
    }));
    tokio::select! {
        biased;
        outcome = &mut waiting => return Err(format!("running owner lost its capacity: {:?}", outcome.map(|Json(value)| value)).into()),
        () = std::future::ready(()) => {},
    }
    drop(waiting);
    release.send(())?;
    require_one_completed(region.drain(WAIT).await?)?;
    check!(eq; std::fs::read(path)?, b"retained capture");
    check!(!ran.load(Ordering::SeqCst));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn closed_legacy_region_refuses_without_running_work() -> TestResult {
    let region = Spawner::with_capacity(1);
    let report = region.drain(WAIT).await?;
    check!(eq; report.accepted, 0);
    check!(eq; report.remaining, 0);
    let ran = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&ran);
    let result = legacy_results_work(&region, move || {
        observed.store(true, Ordering::SeqCst);
        Ok(completion())
    })
    .await;
    match result {
        Err(error) => check!(error
            .to_string()
            .contains("capture results worker cancelled")),
        Ok(_) => return Err("closed region ran legacy work".into()),
    }
    check!(!ran.load(Ordering::SeqCst));
    Ok(())
}
