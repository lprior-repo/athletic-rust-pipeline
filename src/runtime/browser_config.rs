use super::{browser::BrowserSettings, ExecutionMode};
use anyhow::{bail, Result};
use serde::Deserialize;
use std::{
    path::{Component, Path, PathBuf},
    time::Duration,
};
use url::Url;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct RawBrowserConfig {
    executable: PathBuf,
    profile_dir: Option<PathBuf>,
    tabs: usize,
    challenge_wait_seconds: u64,
    headed: bool,
    cdp_endpoint: Option<Url>,
}
impl Default for RawBrowserConfig {
    fn default() -> Self {
        Self {
            executable: PathBuf::from("/usr/bin/chromium"),
            profile_dir: None,
            tabs: 2,
            challenge_wait_seconds: 30,
            headed: true,
            cdp_endpoint: None,
        }
    }
}

impl RawBrowserConfig {
    pub(crate) fn validate(&self, mode: ExecutionMode) -> Result<()> {
        if !(1..=8).contains(&self.tabs) {
            bail!("browser tabs must be in 1..=8");
        }
        if !(15..=30).contains(&self.challenge_wait_seconds) {
            bail!("browser challenge wait must be in 15..=30 seconds");
        }
        if mode == ExecutionMode::Live && !self.headed {
            bail!("live browser must be headed for human challenge completion");
        }
        validate_path(&self.executable)?;
        if let Some(profile) = &self.profile_dir {
            validate_path(profile)?;
        }
        if let Some(ref endpoint) = self.cdp_endpoint {
            validate_cdp_endpoint(endpoint)?;
        }
        Ok(())
    }

    pub(crate) fn settings(
        &self,
        storage: &Path,
        source_origin: Url,
        request_timeout: Duration,
    ) -> BrowserSettings {
        BrowserSettings {
            executable: self.executable.clone(),
            profile_dir: self
                .profile_dir
                .clone()
                .unwrap_or_else(|| storage.with_extension("chrome-profile")),
            tabs: self.tabs,
            source_origin,
            request_timeout,
            challenge_wait: Duration::from_secs(self.challenge_wait_seconds),
            headed: self.headed,
            cdp_endpoint: self.cdp_endpoint.clone(),
        }
    }
}

fn validate_path(path: &Path) -> Result<()> {
    if !path.is_absolute()
        || path.file_name().is_none()
        || path
            .to_str()
            .is_none_or(|text| text.chars().any(char::is_control))
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        bail!("browser paths must be absolute named paths without traversal or controls");
    }
    Ok(())
}

fn validate_cdp_endpoint(url: &Url) -> Result<()> {
    if url.scheme() != "http" && url.scheme() != "https" {
        bail!("cdp endpoint must use http or https scheme");
    }
    let host = url
        .host()
        .ok_or_else(|| anyhow::anyhow!("cdp endpoint must have a host"))?;
    match host {
        url::Host::Domain("localhost") | url::Host::Ipv4(std::net::Ipv4Addr::LOCALHOST) => {}
        url::Host::Ipv6(std::net::Ipv6Addr::LOCALHOST) => {}
        _ => bail!("cdp endpoint must resolve to loopback only"),
    }
    if url.port().is_none() {
        bail!("cdp endpoint must have an explicit port");
    }
    if url.path() != "/" {
        bail!("cdp endpoint path must be root (/)");
    }
    if !url.username().is_empty() || url.password().is_some() {
        bail!("cdp endpoint must have no credentials");
    }
    if url.query().is_some() || url.fragment().is_some() {
        bail!("cdp endpoint must have no query or fragment");
    }
    Ok(())
}
