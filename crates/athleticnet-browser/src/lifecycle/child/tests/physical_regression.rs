use super::super::*;
use crate::lifecycle::startup::build_launch_command;
use crate::BrowserSettings;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> TestResult<Self> {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
        let root = std::env::temp_dir().join(format!(
            "adlaw-owned-child-{}-{}",
            std::process::id(),
            stamp.as_nanos(),
        ));
        std::fs::DirBuilder::new().mode(0o700).create(&root)?;
        let fixture = Self { root };
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o700)
            .open(fixture.root.join("browser"))?;
        file.write_all(b"#!/bin/sh\nexec /bin/sleep 30\n")?;
        file.sync_all()?;
        Ok(fixture)
    }

    fn settings(&self) -> TestResult<BrowserSettings> {
        Ok(BrowserSettings {
            cdp_endpoint: None,
            executable: self.root.join("browser"),
            profile_dir: self.root.join("profile"),
            source_origin: url::Url::parse("https://www.athletic.net/")?,
            follow_same_site_redirects: false,
            tabs: 1,
            request_timeout: Duration::from_secs(1),
            challenge_wait: Duration::from_secs(1),
            headed: false,
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.root) {
            tracing::error!(%error, root = ?self.root, "owned child fixture cleanup failed");
        }
    }
}

#[tokio::test]
async fn dropping_discovery_retains_the_owned_process_until_terminating_and_reaping() -> TestResult
{
    let fixture = Fixture::new()?;
    let mut command = build_launch_command(&fixture.settings()?)?;
    let mut child = OwnedChild::launch(&mut command)?;
    let initial = child.child.try_wait();
    let pending = {
        let discovery = child.endpoint();
        tokio::pin!(discovery);
        futures::poll!(discovery.as_mut()).is_pending()
    };
    let retained = child.child.try_wait();
    let cleanup = child.terminate_and_wait().await;
    let reaped = child.child.wait().await;
    let repeated = child.terminate_and_wait().await;
    let after_repeated = child.child.try_wait();
    check!(pending);
    check!(initial?.is_none());
    check!(retained?.is_none());
    cleanup?;
    let status = reaped?;
    repeated?;
    check!(eq; status.signal(), Some(15));
    check!(eq; after_repeated?, Some(status));
    Ok(())
}
