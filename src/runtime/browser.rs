use reqwest::{header::HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};
use thiserror::Error;
use url::Url;

mod actor;
mod lifecycle;
pub(crate) mod navigation;
mod ops;
mod pool;
pub(crate) mod transport;
pub(crate) use lifecycle::BrowserManager;
pub(crate) mod gate;

#[derive(Clone, Debug)]
pub(crate) struct BrowserSettings {
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
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        if !(1..=8).contains(&self.tabs) {
            anyhow::bail!("browser tab count must be in 1..=8");
        }
        if !self.executable.is_absolute() || !self.profile_dir.is_absolute() {
            anyhow::bail!("browser executable and profile paths must be absolute");
        }
        if !matches!(self.source_origin.scheme(), "http" | "https")
            || self.source_origin.host_str().is_none()
        {
            anyhow::bail!("browser source origin must be an HTTP URL with a host");
        }
        if self.request_timeout.is_zero() || self.challenge_wait.is_zero() {
            anyhow::bail!("browser timeouts must be positive");
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

#[derive(Debug)]
pub(crate) struct BrowserResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
    pub rankings: Option<super::protocol::RankingPageObservation>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserError {
    #[error("browser transport failed")]
    Transport,
    #[error("browser request timed out")]
    Timeout,
    #[error("browser response exceeds payload limit")]
    PayloadLimit,
    #[error("browser redirect rejected")]
    Redirect,
    #[error("browser is unavailable")]
    Unavailable,
    #[error("human verification is required")]
    HumanRequired,
    #[error("browser protocol returned an invalid response")]
    Protocol,
    #[error("browser is shutting down")]
    Shutdown,
}

pub(crate) const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);

fn validate_cdp_endpoint(url: &Url) -> anyhow::Result<()> {
    if url.scheme() != "http" && url.scheme() != "https" {
        anyhow::bail!("cdp endpoint must use http or https scheme");
    }
    let host = url.host().ok_or_else(|| anyhow::anyhow!("cdp endpoint must have a host"))?;
    match host {
        url::Host::Domain("localhost") | url::Host::Ipv4(std::net::Ipv4Addr::LOCALHOST) => {}
        url::Host::Ipv6(std::net::Ipv6Addr::LOCALHOST) => {}
        _ => anyhow::bail!("cdp endpoint must resolve to loopback only"),
    }
    if url.port().is_none() {
        anyhow::bail!("cdp endpoint must have an explicit port");
    }
    if url.path() != "/" {
        anyhow::bail!("cdp endpoint path must be root (/)");
    }
    if !url.username().is_empty() || url.password().is_some() {
        anyhow::bail!("cdp endpoint must have no credentials");
    }
    if url.query().is_some() || url.fragment().is_some() {
        anyhow::bail!("cdp endpoint must have no query or fragment");
    }
    Ok(())
}
