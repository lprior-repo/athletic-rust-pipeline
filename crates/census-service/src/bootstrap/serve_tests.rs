use std::path::PathBuf;
use std::time::Duration;

use athleticnet_browser::BrowserSettings;

use super::lane_client;
use super::ServeOptions;
use crate::bootstrap::EndpointShutdown;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn a_deployment_without_a_lane_builds_no_client() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let plain = ServeOptions::default();
            if !lane_client(&plain)?.is_none() {
                return Err("no lane means no client, whatever the ingress does".into());
            }
            Ok(())
        })
}

#[test]
fn a_deployment_that_serves_the_lane_hands_the_census_a_client() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            if !lane_client(&serving_lane()?)?.is_some() {
                return Err(
                    "the endpoint that serves the profile is the one whose census may use it"
                        .into(),
                );
            }
            Ok(())
        })
}

fn serving_lane() -> TestResult<ServeOptions> {
    let profile_dir = std::env::temp_dir().join("census-service-lane-wiring-test");
    Ok(ServeOptions {
        lane: Some(BrowserSettings {
            cdp_endpoint: None,
            executable: PathBuf::from("/nonexistent/chromium"),
            profile_dir,
            source_origin: url::Url::parse("https://www.athletic.net")?,
            tabs: 2,
            request_timeout: Duration::from_secs(30),
            challenge_wait: Duration::from_secs(30),
            headed: true,
        }),
        ..ServeOptions::default()
    })
}

#[test]
fn an_endpoint_that_finishes_inside_the_grace_reports_a_clean_shutdown() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (ended, done) = tokio::sync::oneshot::channel::<()>();
            ended.send(()).map_err(|_| "receiver dropped")?;
            let state = super::await_endpoint_shutdown(done, Duration::from_millis(50)).await;
            if state != EndpointShutdown::Completed {
                return Err(
                    format!("an endpoint at rest must report Completed, got {state:?}").into(),
                );
            }
            Ok(())
        })
}

#[test]
fn an_endpoint_that_outlasts_the_grace_is_reported_and_the_drain_proceeds() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (ended, done) = tokio::sync::oneshot::channel::<()>();
            let state = super::await_endpoint_shutdown(done, Duration::from_millis(50)).await;
            if state != EndpointShutdown::TimedOut {
                return Err(format!(
                    "a live endpoint must not block admission forever, got {state:?}"
                )
                .into());
            }
            drop(ended);
            Ok(())
        })
}

#[test]
fn an_endpoint_task_that_vanishes_without_signalling_still_ends_the_wait() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (ended, done) = tokio::sync::oneshot::channel::<()>();
            drop(ended);
            let state = super::await_endpoint_shutdown(done, Duration::from_secs(30)).await;
            if state != EndpointShutdown::Completed {
                return Err(
                    format!("a vanished endpoint task ends the wait, got {state:?}").into(),
                );
            }
            Ok(())
        })
}

#[cfg(feature = "native-fault-injection")]
#[test]
fn the_http_exit_fault_predicate_is_pure_and_feature_gated() {
    assert!(!super::http_exit_fault_requested(None));
    assert!(super::http_exit_fault_requested(Some(
        std::ffi::OsStr::new("1")
    )));
    assert_eq!(
        super::http_exit_trigger_path(Some(std::ffi::OsStr::new("/run/fault-trigger"))),
        Some(std::path::PathBuf::from("/run/fault-trigger"))
    );
}

#[test]
fn the_http_exit_trigger_path_is_none_for_immediate_faults() {
    assert_eq!(super::http_exit_trigger_path(None), None);
    assert_eq!(
        super::http_exit_trigger_path(Some(std::ffi::OsStr::new(""))),
        None
    );
    assert_eq!(
        super::http_exit_trigger_path(Some(std::ffi::OsStr::new("1"))),
        None
    );
}
