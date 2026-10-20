use super::support;
use super::*;
use crate::fixture_schema::NORMAL_BODY;
use std::future::Future;
use std::task::{Context, Poll, Waker};

use athleticnet_browser::{BrowserError, BrowserOutcome, BrowserResponse, Verdict};
use reqwest::header::CONTENT_TYPE;

pub(super) async fn capture_case(
    fixtures: &Path,
    name: &'static str,
    path: &'static str,
    exact_body: bool,
) -> ProbeResult<()> {
    let profile = fresh_profile(fixtures, name)?;
    let server = start_server(fixtures, name)?;
    let origin = loopback_origin(server.origin(), name)?;
    let (manager, mut supervisor) = launch(&profile, origin.clone(), name).await?;
    let mut deviations = ready_deviation(&manager);
    let (spec, expected_url) = spec_for(&origin, path, name)?;
    let outcome = manager.fetch(spec).await;
    let (observed, extra) = describe_outcome(exact_body, &expected_url, &outcome);
    deviations.extend(extra);
    let result = finish_case(name, &manager, server, path, observed, deviations, false).await;
    let cleanup = supervisor.stop_and_join();
    combine_cleanup(result, cleanup, name)
}

pub(super) async fn cancellation_case(fixtures: &Path) -> ProbeResult<()> {
    let name = "cancel";
    let profile = fresh_profile(fixtures, name)?;
    let server = start_server(fixtures, name)?;
    let origin = loopback_origin(server.origin(), name)?;
    let (manager, mut supervisor) = launch(&profile, origin, name).await?;
    let mut deviations = ready_deviation(&manager);
    let first_poll = match poll_shutdown_once(&manager) {
        Ok(state) => state.to_string(),
        Err(detail) => {
            deviations.push(format!("first poll was not pending: {detail}"));
            detail
        }
    };
    let observed = format!("first_poll={first_poll}");
    let result = finish_case(name, &manager, server, "/", observed, deviations, true).await;
    let cleanup = supervisor.stop_and_join();
    combine_cleanup(result, cleanup, name)
}

fn combine_cleanup(
    result: ProbeResult<()>,
    cleanup: Result<(), String>,
    name: &str,
) -> ProbeResult<()> {
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(cleanup)) => Err(err(name, cleanup)),
        (Err(error), Err(cleanup)) => Err(err(name, format!("{error}; {cleanup}"))),
    }
}

pub(super) fn poll_shutdown_once(manager: &BrowserManager) -> Result<&'static str, String> {
    let mut shutdown = Box::pin(manager.shutdown());
    let mut context = Context::from_waker(Waker::noop());
    match Future::poll(shutdown.as_mut(), &mut context) {
        Poll::Pending => Ok("pending"),
        Poll::Ready((report, failure)) => Err(format!("ready {report:?} {failure:?}")),
    }
}

pub(super) async fn finish_case(
    name: &str,
    manager: &BrowserManager,
    server: support::Server,
    path: &str,
    observed: String,
    mut deviations: Vec<String>,
    repeat: bool,
) -> ProbeResult<()> {
    let (report, failure) = manager.shutdown().await;
    let state = manager.status().state;
    let alive = manager.is_alive();
    let counters = match server.finish() {
        Ok(counters) => counters,
        Err(error) => return Err(err(name, format!("server finish: {error}"))),
    };
    if let Some(error) = failure.as_ref() {
        if repeat {
            deviations.push(format!(
                "repeat-after-cancel needs a fix or qualification: {error}"
            ));
        } else {
            deviations.push(format!("shutdown reported {error}"));
        }
    }
    let total = report
        .completed
        .checked_add(report.cancelled)
        .and_then(|total| total.checked_add(report.aborted))
        .and_then(|total| total.checked_add(report.panicked));
    if report.remaining != 0 || total != Some(report.accepted) {
        deviations.push(format!("incoherent drain {report:?}"));
    }
    if alive || state != BrowserState::Stopped || report.aborted != 0 || report.panicked != 0 {
        deviations.push(format!("alive={alive} state={state:?} drain={report:?}"));
    }
    let data_gets = DATA_PATHS.iter().try_fold(0u64, |total, data_path| {
        total
            .checked_add(counters.get(*data_path).copied().map_or(0, |count| count))
            .ok_or_else(|| err(name, "request counter overflow".to_string()))
    })?;
    let want_data_gets = u64::from(path != "/");
    let path_gets = counters.get(path).copied().map_or(0, |count| count);
    let bootstrap = counters.get("/").copied().map_or(0, |count| count);
    if path_gets != 1 || data_gets != want_data_gets || bootstrap == 0 {
        deviations.push(format!("GET distribution {counters:?}"));
    }
    println!("CASE {name} path={path} {observed} state={state:?} alive={alive} shutdown_error={failure:?} drain={report:?} counters={counters:?}");
    if deviations.is_empty() {
        return Ok(());
    }
    Err(err(name, deviations.join("; ")))
}

pub(super) fn describe_outcome(
    exact_body: bool,
    expected_url: &str,
    outcome: &BrowserOutcome,
) -> (String, Vec<String>) {
    let mut deviations = Vec::new();
    if outcome.verdict() != Verdict::Terminal {
        deviations.push(format!("verdict {:?}", outcome.verdict()));
    }
    let description = match outcome {
        BrowserOutcome::Captured(capture) => {
            let response = &capture.response;
            if response.response_url.as_deref() != Some(expected_url) {
                deviations.push("response URL differs from the issued request".to_string());
            }
            let status = response.status.as_u16();
            if exact_body && status != 200 {
                deviations.push(format!("http status {status}"));
            }
            if exact_body && response.body.as_slice() != NORMAL_BODY {
                deviations.push("body is not wire-smoke-ok".to_string());
            }
            if exact_body && !text_plain(response) {
                deviations.push("content type is not text/plain".to_string());
            }
            if !exact_body {
                deviations.push("expected payload_limit, captured".to_string());
            }
            format!(
                "captured status={status} bytes={} exact_body={} url_exact={}",
                response.body.len(),
                response.body.as_slice() == NORMAL_BODY,
                response.response_url.as_deref() == Some(expected_url),
            )
        }
        BrowserOutcome::Failed(failure) => {
            if exact_body {
                deviations.push("the normal response was not captured".to_string());
            }
            if failure.error != BrowserError::PayloadLimit {
                deviations.push(format!("error {:?}", failure.error));
            }
            format!(
                "failed error={:?} verdict={:?}",
                failure.error, failure.verdict
            )
        }
    };
    (description, deviations)
}

pub(super) fn text_plain(response: &BrowserResponse) -> bool {
    response
        .headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/plain"))
}
