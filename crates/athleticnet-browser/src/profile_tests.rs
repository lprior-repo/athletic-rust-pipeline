use super::*;
use crate::lifecycle::error::BrowserStartupError;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

fn settings(profile_dir: PathBuf) -> Result<BrowserSettings, url::ParseError> {
    Ok(BrowserSettings {
        cdp_endpoint: None,
        executable: PathBuf::from("/usr/bin/chromium"),
        profile_dir,
        source_origin: Url::parse("https://www.athletic.net/")?,
        tabs: 2,
        request_timeout: Duration::from_secs(30),
        challenge_wait: Duration::from_secs(5),
        headed: true,
    })
}

#[test]
fn a_relative_profile_directory_is_an_invalid_path() -> Result<(), Box<dyn std::error::Error>> {
    let settings = settings(PathBuf::from("var/browser-profile"))?;
    check!(matches!(
        settings.validate(),
        Err(BrowserStartupError::InvalidPaths)
    ));
    Ok(())
}

#[test]
fn an_absolute_profile_directory_with_between_one_and_eight_tabs_is_valid(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut settings = settings(PathBuf::from("/tmp/browser-profile"))?;
    check!(settings.validate().is_ok());
    settings.tabs = 0;
    check!(matches!(
        settings.validate(),
        Err(BrowserStartupError::InvalidTabCount)
    ));
    settings.tabs = 9;
    check!(matches!(
        settings.validate(),
        Err(BrowserStartupError::InvalidTabCount)
    ));
    Ok(())
}

#[test]
fn a_relative_executable_is_an_invalid_path() -> Result<(), Box<dyn std::error::Error>> {
    let mut settings = settings(PathBuf::from("/tmp/browser-profile"))?;
    settings.executable = PathBuf::from("chromium");
    check!(matches!(
        settings.validate(),
        Err(BrowserStartupError::InvalidPaths)
    ));
    Ok(())
}
