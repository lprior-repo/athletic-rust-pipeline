use crate::clock::SystemClock;
use crate::{BrowserManager, BrowserSettings, BrowserStartupError};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn settings(profile_dir: PathBuf) -> Result<BrowserSettings, url::ParseError> {
    Ok(BrowserSettings {
        cdp_endpoint: Some(Url::parse("http://127.0.0.1:9222/")?),
        executable: PathBuf::from("/no-browser-required"),
        profile_dir,
        source_origin: Url::parse("https://www.athletic.net/")?,
        tabs: 1,
        request_timeout: Duration::from_secs(1),
        challenge_wait: Duration::from_secs(1),
        headed: true,
    })
}

#[tokio::test]
async fn connect_validates_the_actual_argument_not_an_unrelated_valid_setting() -> TestResult {
    let settings = settings(std::env::temp_dir())?;
    for (url, expected) in [
        ("file:///tmp/cdp", "scheme"),
        ("http://192.0.2.1:9222/", "loopback"),
        ("http://127.0.0.1:9222/foreign", "path"),
        ("http://user@127.0.0.1:9222/", "credentials"),
    ] {
        let result = BrowserManager::prepare_connect(
            Url::parse(url)?,
            settings.clone(),
            Arc::new(SystemClock),
        );
        check!(match (expected, result) {
            ("scheme", Err(BrowserStartupError::CdpBadScheme))
            | ("loopback", Err(BrowserStartupError::CdpNotLoopback))
            | ("path", Err(BrowserStartupError::CdpBadPath))
            | ("credentials", Err(BrowserStartupError::CdpHasCredentials)) => true,
            _ => false,
        });
    }
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn launch_preserves_the_profile_permission_refusal_before_child_creation() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let profile = std::env::temp_dir().join(format!(
        "adlaw-browser-permissions-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&profile)?;
    std::fs::set_permissions(&profile, std::fs::Permissions::from_mode(0o755))?;
    let (_startup, mut owner) =
        BrowserManager::prepare_launch(settings(profile.clone())?, Arc::new(SystemClock))?;
    let (_stop, stopping) = tokio::sync::watch::channel(false);
    let result = owner.run(stopping).await;
    let cleanup = owner.finalize().await;
    std::fs::remove_dir(&profile)?;
    check!(matches!(
        result,
        crate::BrowserExit::Failed(BrowserStartupError::ProfilePermissions)
    ));
    check!(matches!(
        cleanup,
        crate::BrowserCleanupAttempt::Reaped { report, failure: None }
        if report == crate::drain::DrainReport::default()
    ));
    Ok(())
}
