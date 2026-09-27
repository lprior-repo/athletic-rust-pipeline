use crate::lifecycle::error::BrowserStartupError;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};
use url::Url;

#[derive(Clone, Debug)]
pub struct BrowserSettings {
    pub cdp_endpoint: Option<Url>,
    pub executable: PathBuf,
    pub profile_dir: PathBuf,
    pub source_origin: Url,
    pub tabs: usize,
    pub request_timeout: Duration,
    pub challenge_wait: Duration,
    pub headed: bool,
}

impl BrowserSettings {
    pub fn validate(&self) -> Result<(), BrowserStartupError> {
        if !(1..=8).contains(&self.tabs) {
            return Err(BrowserStartupError::InvalidTabCount);
        }
        if !self.executable.is_absolute() || !self.profile_dir.is_absolute() {
            return Err(BrowserStartupError::InvalidPaths);
        }
        if !matches!(self.source_origin.scheme(), "http" | "https")
            || self.source_origin.host_str().is_none()
        {
            return Err(BrowserStartupError::InvalidOrigin);
        }
        if self.request_timeout.is_zero() || self.challenge_wait.is_zero() {
            return Err(BrowserStartupError::InvalidTimeout);
        }
        if let Some(ref endpoint) = self.cdp_endpoint {
            validate_cdp_endpoint(endpoint)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserState {
    Ready,
    Challenged,
    CoolingDown,
    HumanRequired,
    Restarting,
    Stopped,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BrowserStatus {
    pub state: BrowserState,
    pub active_requests: usize,
    pub tabs: usize,
    pub cooldown_ms: u64,
}

fn validate_cdp_endpoint(url: &Url) -> Result<(), BrowserStartupError> {
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(BrowserStartupError::CdpBadScheme);
    }
    let host = url.host().ok_or(BrowserStartupError::CdpNoHost)?;
    match host {
        url::Host::Domain("localhost") | url::Host::Ipv4(std::net::Ipv4Addr::LOCALHOST) => {}
        url::Host::Ipv6(std::net::Ipv6Addr::LOCALHOST) => {}
        _ => return Err(BrowserStartupError::CdpNotLoopback),
    }
    if url.port().is_none() {
        return Err(BrowserStartupError::CdpNoPort);
    }
    if url.path() != "/" {
        return Err(BrowserStartupError::CdpBadPath);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(BrowserStartupError::CdpHasCredentials);
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(BrowserStartupError::CdpHasQueryFragment);
    }
    Ok(())
}
