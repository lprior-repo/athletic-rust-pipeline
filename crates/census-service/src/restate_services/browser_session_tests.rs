use std::path::PathBuf;
use std::time::Duration;

use athleticnet_browser::clock::SystemClock;
use athleticnet_browser::request::RequestAction;
use url::Url;

use super::*;

#[path = "browser_session_replay_tests.rs"]
mod replay;

#[path = "browser_session_tests/native_publication.rs"]
mod native_publication;
#[path = "browser_session_tests/physical_startup.rs"]
mod physical_startup;

fn settings() -> anyhow::Result<BrowserSettings> {
    Ok(BrowserSettings {
        cdp_endpoint: None,
        executable: PathBuf::from("/nonexistent/chromium"),
        profile_dir: PathBuf::from("/nonexistent/profile"),
        source_origin: Url::parse("https://www.athletic.net/")?,
        follow_same_site_redirects: false,
        tabs: 1,
        request_timeout: Duration::from_secs(30),
        challenge_wait: Duration::from_secs(10),
        headed: true,
    })
}

fn session() -> anyhow::Result<BrowserSession> {
    Ok(BrowserSession::new(
        settings()?,
        Arc::new(SystemClock),
        Arc::new(crate::spawn::Spawner::new()),
    ))
}

#[test]
fn a_foreign_origin_is_refused_before_the_lane_is_reached() -> anyhow::Result<()> {
    let session = session()?;
    let spec = |url: &str| -> anyhow::Result<RequestSpec> {
        Ok(RequestSpec {
            url: Url::parse(url)?,
            semantic_url: url.to_string(),
            action: RequestAction::Fetch {},
            headers: Vec::new(),
        })
    };
    for url in [
        "file://www.athletic.net/etc/passwd",
        "ftp://www.athletic.net/mirror",
        "chrome://www.athletic.net/settings",
        "http://www.athletic.net/api/v1/x",
        "https://www.athletic.net:8443/api/v1/x",
        "https://www.athletic.net.evil.example/api/v1/x",
    ] {
        let Some(refusal) = session.foreign_origin_refusal(&spec(url)?) else {
            anyhow::bail!("{url} must be refused before any navigation");
        };
        anyhow::ensure!(
            refusal.contains("https://www.athletic.net/"),
            "the refusal names the configured origin: {refusal}"
        );
        anyhow::ensure!(
            refusal.contains(url),
            "the refusal names the refused URL: {refusal}"
        );
    }
    let census: RequestSpec = serde_json::from_str(include_str!(
        "../../../../fixtures/wire/athleticnet-browser-request.json"
    ))?;
    anyhow::ensure!(
        session.foreign_origin_refusal(&census).is_none(),
        "the census's own source request must reach the lane"
    );
    anyhow::ensure!(
        session
            .foreign_origin_refusal(&spec("https://www.athletic.net/track-and-field/rankings")?)
            .is_none(),
        "a public source page must reach the lane"
    );
    Ok(())
}

#[test]
fn an_unstarted_lane_reads_as_not_running_and_refuses_fetches() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
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
        })
}

#[tokio::test]
async fn a_launch_is_not_superseded_by_its_own_epoch_bump() -> anyhow::Result<()> {
    let session = session()?;
    let epoch = session
        .begin_launch()
        .await
        .map_err(|error| anyhow::anyhow!("the launch could not begin: {error:?}"))?;
    anyhow::ensure!(
        session.lane.lock().await.epoch == epoch,
        "a start must not supersede its own launch epoch"
    );
    Ok(())
}

#[tokio::test]
async fn a_start_waits_for_an_in_flight_launch_instead_of_bumping_its_epoch() -> anyhow::Result<()>
{
    let session = session()?;
    let held = session.launch_admission().await;
    let mut waiting = Box::pin(session.launch_admission());
    anyhow::ensure!(
        futures::poll!(waiting.as_mut()).is_pending(),
        "a second start must wait for the in-flight launch instead of launching beside it"
    );
    anyhow::ensure!(
        session.lane.lock().await.epoch == 0,
        "a waiting start must not bump the in-flight launch epoch"
    );
    drop(held);
    waiting.await;
    anyhow::ensure!(
        session.lane.lock().await.epoch == 0,
        "the epoch moves only when a launch actually begins"
    );
    Ok(())
}

#[tokio::test]
async fn an_exhausted_launch_sequence_refuses_without_reusing_ownership() -> anyhow::Result<()> {
    let session = session()?;
    session.lane.lock().await.epoch = u64::MAX;
    anyhow::ensure!(session.begin_launch().await.is_err());
    let lane = session.lane.lock().await;
    check!(eq; lane.epoch, u64::MAX);
    check!(lane.manager.is_none());
    Ok(())
}

#[tokio::test]
async fn cancelling_before_dedicated_registration_leaves_no_launch_epoch_or_admission(
) -> anyhow::Result<()> {
    let session = BrowserSession::new(
        settings()?,
        Arc::new(SystemClock),
        Arc::new(crate::spawn::Spawner::with_capacity(1)),
    );
    let held = session
        .region
        .acquire_blocking()
        .await
        .ok_or_else(|| anyhow::anyhow!("blocking slot missing"))?;
    let mut starting = Box::pin(session.start_owned());
    check!(futures::poll!(starting.as_mut()).is_pending());
    drop(starting);
    let lane = session.lane.lock().await;
    check!(eq; lane.epoch, 0);
    check!(!lane.starting);
    check!(lane.manager.is_none());
    drop(lane);
    let reservation = match session
        .region
        .coordinator()
        .try_reserve_profile(crate::admission::WORK_CLASS_BROWSER)
    {
        crate::admission::TryResult::Granted(reservation) => reservation,
        other => anyhow::bail!("cancelled startup retained its coordination grant: {other:?}"),
    };
    drop(reservation);
    let _admission = session.launch_admission().await;
    drop(held);
    Ok(())
}
