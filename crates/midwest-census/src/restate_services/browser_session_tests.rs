//! Colocated tests for the lane's operator surface: the wire shapes an operator reads, and the
//! unstarted reading a deployment must produce after a restart (the manager is process state).
//!
//! Nothing here launches a browser. The transport's own contract is proved by the committed
//! `fixtures/wire/` captures, read on both sides of the lane; what is proved here is that this
//! object's replies carry the engine's counters and states without re-encoding them.

use std::path::PathBuf;
use std::time::Duration;

use athleticnet_browser::clock::SystemClock;
use athleticnet_browser::drain::DrainReport;
use url::Url;

use super::*;

fn settings() -> anyhow::Result<BrowserSettings> {
    Ok(BrowserSettings {
        cdp_endpoint: None,
        executable: PathBuf::from("/nonexistent/chromium"),
        profile_dir: PathBuf::from("/nonexistent/profile"),
        source_origin: Url::parse("https://www.athletic.net/")?,
        tabs: 1,
        request_timeout: Duration::from_secs(30),
        challenge_wait: Duration::from_secs(10),
        headed: true,
    })
}

fn session() -> anyhow::Result<BrowserSession> {
    Ok(BrowserSession::new(settings()?, Arc::new(SystemClock)))
}

/// Every counter the engine reports survives the move onto the wire: a §42 accounting reads
/// `remaining`, a failure report reads the rest, and a dropped field would read as zero.
#[test]
fn drain_counts_keep_every_counter() {
    let report = DrainReport {
        accepted: 7,
        completed: 5,
        cancelled: 2,
        timed_out: 1,
        aborted: 1,
        panicked: 0,
        remaining: 0,
    };
    let counts = DrainCounts::from(report);
    assert_eq!(counts.accepted, 7);
    assert_eq!(counts.completed, 5);
    assert_eq!(counts.cancelled, 2);
    assert_eq!(counts.timed_out, 1);
    assert_eq!(counts.aborted, 1);
    assert_eq!(counts.panicked, 0);
    assert_eq!(counts.remaining, 0);
}

/// The replies an operator's client parses: field names are the client's contract, so a rename is a
/// breaking change and has to fail here rather than in a deployment.
#[test]
fn operator_replies_round_trip() -> anyhow::Result<()> {
    let status = BrowserSessionStatus {
        key: SESSION_KEY.to_string(),
        running: false,
        status: None,
        error: Some("browser is unavailable".to_string()),
    };
    let json = serde_json::to_value(&status)?;
    anyhow::ensure!(json["key"] == SESSION_KEY, "the key field moved: {json}");
    anyhow::ensure!(
        json["running"] == serde_json::Value::Bool(false),
        "the running field moved: {json}"
    );
    anyhow::ensure!(
        json["error"] == "browser is unavailable",
        "the error field moved: {json}"
    );
    let read: BrowserSessionStatus = serde_json::from_value(json)?;
    anyhow::ensure!(read.key == SESSION_KEY, "the key did not round trip");

    let drain = BrowserSessionDrain {
        key: SESSION_KEY.to_string(),
        drain: DrainCounts::default(),
        error: None,
    };
    let json = serde_json::to_value(&drain)?;
    anyhow::ensure!(
        json["drain"]["remaining"] == 0,
        "the remaining counter moved: {json}"
    );
    anyhow::ensure!(
        json["drain"]["accepted"] == 0,
        "the accepted counter moved: {json}"
    );
    Ok(())
}

/// The key the census's client addresses. The two crates agree by this string, so it is asserted
/// rather than compared at a call site.
#[test]
fn the_key_names_the_one_profile() {
    assert_eq!(SESSION_KEY, "profile-0");
}

/// After an endpoint restart there is no manager, and that is a state rather than a failure: the
/// reading says so, and `fetch` refuses instead of launching a browser the operator did not ask for.
#[tokio::test]
async fn an_unstarted_lane_reads_as_not_running_and_refuses_fetches() -> anyhow::Result<()> {
    let session = session()?;
    let reading = session.reading(SESSION_KEY).await;
    anyhow::ensure!(reading.key == SESSION_KEY, "the reading names another key");
    anyhow::ensure!(!reading.running, "a fresh process has no live manager");
    anyhow::ensure!(
        reading.status.is_none(),
        "no manager has no engine reading: {reading:?}"
    );
    anyhow::ensure!(
        reading.error.is_none(),
        "an unstarted lane is a state, not a failure: {reading:?}"
    );
    anyhow::ensure!(
        session.live().await.is_err(),
        "fetch must refuse an unstarted lane instead of launching a browser"
    );
    Ok(())
}
