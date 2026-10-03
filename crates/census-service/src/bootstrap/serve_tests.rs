use std::path::PathBuf;
use std::time::Duration;

use athleticnet_browser::BrowserSettings;

use super::lane_client;
use super::ServeOptions;

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
