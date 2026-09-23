//! The shell's lane wiring: which deployments hand the census a client, and which do not.
//!
//! Nothing here launches a browser. What is proved is the decision itself, because it is the input to
//! the run's source plan: a client the census does not have turns a browser-transported source into a
//! refusal by name, and one it does have makes that source ordinary work.

use std::path::PathBuf;
use std::time::Duration;

use athleticnet_browser::BrowserSettings;

use super::lane_client;
use super::ServeOptions;

/// A deployment that serves no lane builds no client: the census then refuses a browser-transported
/// source by name instead of failing requests against a lane that is not there.
#[tokio::test]
async fn a_deployment_without_a_lane_builds_no_client() {
    let plain = ServeOptions::default();
    assert!(
        lane_client(&plain).unwrap().is_none(),
        "no lane means no client, whatever the ingress does"
    );
}

/// A deployment that serves the profile hands the census a client for it: the object installs it on
/// the shared fetcher, which is what the plan asks.
#[tokio::test]
async fn a_deployment_that_serves_the_lane_hands_the_census_a_client() {
    assert!(
        lane_client(&serving_lane()).unwrap().is_some(),
        "the endpoint that serves the profile is the one whose census may use it"
    );
}

/// The options a deployment that serves the lane runs with. The profile directory is a tempdir and
/// the executable does not exist: neither is read until an operator starts the lane.
fn serving_lane() -> ServeOptions {
    let profile_dir = std::env::temp_dir().join("midwest-census-lane-wiring-test");
    ServeOptions {
        lane: Some(BrowserSettings {
            cdp_endpoint: None,
            executable: PathBuf::from("/nonexistent/chromium"),
            profile_dir,
            source_origin: url::Url::parse("https://www.athletic.net").expect("the lane's origin"),
            tabs: 2,
            request_timeout: Duration::from_secs(30),
            challenge_wait: Duration::from_secs(30),
            headed: true,
        }),
        ..ServeOptions::default()
    }
}
