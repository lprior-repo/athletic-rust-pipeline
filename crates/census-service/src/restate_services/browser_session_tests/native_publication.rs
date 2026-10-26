#[path = "native_publication/http_fixture.rs"]
mod http_fixture;

use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use athleticnet_browser::clock::SystemClock;
use athleticnet_browser::request::{safe, RequestAction};
use athleticnet_browser::{BrowserOutcome, BrowserSettings, BrowserState};
use url::Url;

use crate::restate_services::browser_session::{BrowserSession, SESSION_KEY};
use crate::spawn::Spawner;

const LIFECYCLE_TIMEOUT: Duration = Duration::from_secs(30);

fn fixture_settings(
    executable: PathBuf,
    profile_dir: PathBuf,
    source_origin: Url,
) -> BrowserSettings {
    BrowserSettings {
        cdp_endpoint: None,
        executable,
        profile_dir,
        source_origin,
        follow_same_site_redirects: false,
        tabs: 1,
        request_timeout: Duration::from_secs(15),
        challenge_wait: Duration::from_secs(5),
        headed: false,
    }
}

async fn exercise_live_lane(session: &BrowserSession, origin: &Url) -> anyhow::Result<()> {
    session
        .start_owned()
        .await
        .map_err(|error| anyhow::anyhow!("start: {error:?}"))?;
    let reading = session.reading(SESSION_KEY).await;
    anyhow::ensure!(
        reading.running && reading.error.is_none(),
        "unexpected reading: {reading:?}"
    );
    let status = reading
        .status
        .ok_or_else(|| anyhow::anyhow!("missing engine status"))?;
    anyhow::ensure!(
        status.state == BrowserState::Ready,
        "engine not ready: {status:?}"
    );
    anyhow::ensure!(
        session.admission.try_lock().is_err(),
        "live browser lost its admission"
    );
    let manager = session
        .live()
        .await
        .map_err(|error| anyhow::anyhow!("live: {error:?}"))?;
    let requested = origin.join("owned")?;
    let request = safe(requested.clone(), RequestAction::Fetch {})?;
    match manager.fetch(request).await {
        BrowserOutcome::Captured(capture) => {
            anyhow::ensure!(
                capture.response.status.as_u16() == 200,
                "unexpected HTTP status"
            );
            anyhow::ensure!(
                capture.response.response_url.as_deref() == Some(requested.as_str()),
                "response URL changed"
            );
            anyhow::ensure!(
                capture.response.body.as_slice() == http_fixture::BODY,
                "captured bytes differ"
            );
        }
        BrowserOutcome::Failed(failure) => anyhow::bail!("native capture failed: {failure:?}"),
    }
    Ok(())
}

#[ignore = "explicit isolated Chromium fixture"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_start_ready_stop_reaps_physically() -> anyhow::Result<()> {
    let executable = PathBuf::from(std::env::var("R14_NATIVE_BROWSER_EXECUTABLE")?);
    anyhow::ensure!(
        executable.is_file(),
        "native browser executable is not a file"
    );
    let temp = tempfile::tempdir()?;
    let profile = temp.path().join("browser-profile");
    std::fs::DirBuilder::new().mode(0o700).create(&profile)?;
    let mut fixture = http_fixture::bind()?;
    let origin = fixture.origin()?;
    let settings = fixture_settings(executable, profile, origin.clone());
    let region = Arc::new(Spawner::new());
    let session = BrowserSession::new(settings, Arc::new(SystemClock), Arc::clone(&region));
    let baseline = region.coordinator().remaining();
    let exercised =
        tokio::time::timeout(LIFECYCLE_TIMEOUT, exercise_live_lane(&session, &origin)).await;
    let stopped =
        tokio::time::timeout(LIFECYCLE_TIMEOUT, session.shutdown_current(SESSION_KEY)).await;
    let drained = region.drain(LIFECYCLE_TIMEOUT).await;
    let served = fixture.stop();
    exercised??;
    let stopped = stopped?.map_err(|error| anyhow::anyhow!("stop: {error:?}"))?;
    let drained = drained?;
    let served = served?;
    anyhow::ensure!(
        stopped.error.is_none() && stopped.drain.remaining == 0,
        "browser stop failed: {stopped:?}"
    );
    anyhow::ensure!(
        drained.accepted == 1 && drained.completed == 1 && drained.failed == 0,
        "owner did not complete: {drained:?}"
    );
    anyhow::ensure!(
        region.coordinator().remaining() == baseline,
        "browser resources were not returned"
    );
    anyhow::ensure!(served > 0, "native browser never reached the owned fixture");
    let lane = session.lane.lock().await;
    anyhow::ensure!(
        lane.manager.is_none() && !lane.starting && lane.failure.is_none(),
        "browser lane retained failed or live state"
    );
    drop(lane);
    anyhow::ensure!(
        session.admission.try_lock().is_ok(),
        "physical owner retained launch admission"
    );
    temp.close()?;
    Ok(())
}
