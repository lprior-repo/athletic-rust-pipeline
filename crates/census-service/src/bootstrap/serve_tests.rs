use std::path::PathBuf;
use std::time::Duration;

use athleticnet_browser::BrowserSettings;

use super::lane_client;
use super::ServeOptions;

#[tokio::test]
async fn a_deployment_without_a_lane_builds_no_client() {
    let plain = ServeOptions::default();
    assert!(
        lane_client(&plain).unwrap().is_none(),
        "no lane means no client, whatever the ingress does"
    );
}

#[tokio::test]
async fn a_deployment_that_serves_the_lane_hands_the_census_a_client() {
    assert!(
        lane_client(&serving_lane()).unwrap().is_some(),
        "the endpoint that serves the profile is the one whose census may use it"
    );
}

fn serving_lane() -> ServeOptions {
    let profile_dir = std::env::temp_dir().join("census-service-lane-wiring-test");
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
