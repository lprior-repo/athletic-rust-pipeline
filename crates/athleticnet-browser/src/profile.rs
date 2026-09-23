//! The persistent profile the transport drives: how it is configured, how that configuration is
//! validated, and the durable state the manager reports for it.
//!
//! One `profile_dir` belongs to one manager (see the crate docs), so these settings are the whole
//! description of the identity a lane runs under: the executable that is launched, the origin that
//! is read, how many tabs the pool may hold, and the two timeouts one attempt is bounded by.
//!
//! [`BrowserState`] and [`BrowserStatus`] are the answer about that profile after it has been
//! driven. The state is what the lifecycle latches and what the profile gate revokes, and it is not
//! a caller's to reinterpret: `Challenged` and `HumanRequired` mean the profile needs a human step
//! before it serves traffic, not that another invocation is worth making.

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
    pub fn validate(&self) -> anyhow::Result<()> {
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

fn validate_cdp_endpoint(url: &Url) -> anyhow::Result<()> {
    if url.scheme() != "http" && url.scheme() != "https" {
        anyhow::bail!("cdp endpoint must use http or https scheme");
    }
    let host = url
        .host()
        .ok_or_else(|| anyhow::anyhow!("cdp endpoint must have a host"))?;
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
