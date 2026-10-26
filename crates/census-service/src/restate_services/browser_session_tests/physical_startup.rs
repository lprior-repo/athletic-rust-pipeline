use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use athleticnet_browser::clock::SystemClock;
use athleticnet_browser::BrowserSettings;
use tokio::net::TcpListener;
use url::Url;

use super::super::{BrowserSession, SESSION_KEY};
use crate::admission::WORK_CLASS_BROWSER;
use crate::spawn::{Spawner, TaskReport};

const CDP_ACCEPT_TIMEOUT: Duration = Duration::from_secs(15);
const DRAIN_TIMEOUT: Duration = Duration::from_secs(30);

fn private_directory(path: &Path) -> anyhow::Result<()> {
    std::fs::DirBuilder::new().mode(0o700).create(path)?;
    let mode = std::fs::symlink_metadata(path)?.permissions().mode() & 0o777;
    anyhow::ensure!(
        mode == 0o700,
        "{} is not a private directory: mode {mode:o}",
        path.display()
    );
    Ok(())
}

fn fixture_executable(fixture_dir: &Path, port: u16) -> anyhow::Result<PathBuf> {
    let path = fixture_dir.join("fixture_browser.sh");
    let script = format!(
        "#!/bin/sh\n\
         set -e\n\
         set -C\n\
         umask 077\n\
         printf '%s\\n' \"$$\" > \"${{0%/*}}/fixture.pid\"\n\
         printf 'DevTools listening on ws://127.0.0.1:{port}/devtools/browser/physical-startup-fixture\\n' >&2\n\
         exec /bin/sleep 30\n"
    );
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o700)
        .open(&path)?;
    file.write_all(script.as_bytes())?;
    file.sync_all()?;
    let mode = file.metadata()?.permissions().mode() & 0o777;
    anyhow::ensure!(
        mode == 0o700,
        "{} is not a private executable: mode {mode:o}",
        path.display()
    );
    Ok(path)
}

fn fixture_settings(executable: PathBuf, profile_dir: PathBuf) -> anyhow::Result<BrowserSettings> {
    Ok(BrowserSettings {
        cdp_endpoint: None,
        executable,
        profile_dir,
        source_origin: Url::parse("https://www.athletic.net/")?,
        follow_same_site_redirects: false,
        tabs: 1,
        request_timeout: Duration::from_secs(30),
        challenge_wait: Duration::from_secs(10),
        headed: false,
    })
}

fn read_fixture_pid(path: &Path) -> anyhow::Result<u32> {
    let metadata = std::fs::symlink_metadata(path)?;
    anyhow::ensure!(
        metadata.file_type().is_file(),
        "{} is not a regular fixture PID file",
        path.display()
    );
    let mode = metadata.permissions().mode() & 0o777;
    anyhow::ensure!(
        mode == 0o600,
        "{} is not a private fixture PID file: mode {mode:o}",
        path.display()
    );
    let text = std::fs::read_to_string(path)?;
    let pid = text.trim().parse::<u32>()?;
    anyhow::ensure!(pid != 0, "the fixture wrote an invalid PID: {text:?}");
    Ok(pid)
}

#[cfg(target_os = "linux")]
fn process_pid_present(pid: u32) -> bool {
    Path::new("/proc").join(pid.to_string()).exists()
}

async fn cleanup_after_failed_start(
    session: &BrowserSession,
    region: &Spawner,
    failure: String,
) -> anyhow::Error {
    let published = session.lane.lock().await.manager.clone();
    let retired = match published {
        Some(manager) => {
            let (report, error) = manager.shutdown().await;
            format!(
                "; a published manager was retired with report {report:?} and failure {error:?}"
            )
        }
        None => String::new(),
    };
    match region.drain(DRAIN_TIMEOUT).await {
        Ok(report) => {
            anyhow::anyhow!("{failure}{retired}; the owned region drained with {report:?}")
        }
        Err(error) => {
            anyhow::anyhow!("{failure}{retired}; the owned region drain failed: {error}")
        }
    }
}

#[tokio::test]
async fn cancelled_physical_startup_reaps_the_exact_process_and_returns_the_reservation(
) -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let fixture_dir = temp.path().join("fixture");
    let profile_dir = temp.path().join("profile");
    private_directory(&fixture_dir)?;
    private_directory(&profile_dir)?;

    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();

    let executable = fixture_executable(&fixture_dir, port)?;
    let pid_file = fixture_dir.join("fixture.pid");
    let settings = fixture_settings(executable, profile_dir)?;

    let region = Arc::new(Spawner::new());
    let session = BrowserSession::new(settings, Arc::new(SystemClock), Arc::clone(&region));
    let baseline = region.coordinator().remaining();

    let mut starting = Box::pin(session.start_owned());
    let accepted = tokio::select! {
        biased;
        result = &mut starting => Err(match result {
            Ok(()) => "the browser lane published a manager without a CDP handshake".to_owned(),
            Err(error) => format!(
                "the browser startup ended before the fixture connected: {error:?}"
            ),
        }),
        accepted = tokio::time::timeout(CDP_ACCEPT_TIMEOUT, listener.accept()) => match accepted {
            Ok(Ok((stream, _peer))) => Ok(stream),
            Ok(Err(error)) => Err(format!(
                "the parent CDP listener could not accept the fixture connection: {error}"
            )),
            Err(_elapsed) => Err(format!(
                "the fixture did not connect to the parent CDP listener within {CDP_ACCEPT_TIMEOUT:?}"
            )),
        },
    };

    let _accepted_connection = match accepted {
        Ok(connection) => connection,
        Err(failure) => {
            drop(starting);
            return Err(cleanup_after_failed_start(&session, &region, failure).await);
        }
    };

    let pid = match read_fixture_pid(&pid_file) {
        Ok(pid) => pid,
        Err(error) => {
            drop(starting);
            let failure = format!("the accepted fixture PID could not be read: {error}");
            return Err(cleanup_after_failed_start(&session, &region, failure).await);
        }
    };
    let held = region.coordinator().remaining();
    #[cfg(target_os = "linux")]
    let observed_live = process_pid_present(pid);

    drop(starting);
    let drain = region.drain(DRAIN_TIMEOUT).await?;

    check!(eq;
        baseline.bytes.checked_sub(held.bytes),
        Some(WORK_CLASS_BROWSER.resources.bytes),
        "the accepted CDP connection must be backed by a live WORK_CLASS_BROWSER byte reservation"
    );
    check!(eq;
        baseline.slots.browser.checked_sub(held.slots.browser),
        Some(WORK_CLASS_BROWSER.resources.slots.browser),
        "the accepted CDP connection must be backed by a live browser slot"
    );
    check!(eq;
        drain,
        TaskReport {
            accepted: 1,
            completed: 1,
            ..TaskReport::default()
        },
        "the cancelled owner must be the only accepted worker and must complete exactly once"
    );
    check!(eq;
        region.coordinator().remaining(),
        baseline,
        "the cancelled browser must return its exact reservation"
    );
    #[cfg(target_os = "linux")]
    {
        check!(
            observed_live,
            "the exact fixture process {pid} was not observable after accept"
        );
        check!(
            !process_pid_present(pid),
            "the exact fixture process {pid} survived cancellation and the owned worker drain"
        );
    }
    {
        let lane = session.lane.lock().await;
        check!(eq; lane.epoch, 1, "exactly one physical launch must have begun");
        check!(
            !lane.starting,
            "the cancelled launch must clear the starting flag"
        );
        check!(
            lane.manager.is_none(),
            "the cancelled launch must not retain a manager"
        );
        check!(
            lane.failure.is_none(),
            "a cancelled launch is not a failure"
        );
    }

    let admission = session.launch_admission().await;
    drop(admission);

    let reading = session.reading(SESSION_KEY).await;
    check!(eq; reading.key.as_str(), SESSION_KEY);
    check!(
        !reading.running,
        "the cancelled launch must read as not running"
    );
    check!(
        reading.status.is_none(),
        "no manager means no engine reading"
    );
    check!(
        reading.error.is_none(),
        "a cancelled launch is a state, not a failure"
    );
    Ok(())
}
