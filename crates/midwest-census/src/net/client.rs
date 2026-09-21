//! Client construction and the per-host pacing state.

use super::{FetchStats, Fetcher, DEFAULT_USER_AGENT, REQUEST_TIMEOUT_SECS};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Per-host politeness state.
pub(super) struct HostState {
    pub(super) gate: Arc<Mutex<()>>,
    /// When the next request to this host may start (reserved before sleeping so that the spacing
    /// holds even when several tasks queue behind the gate).
    pub(super) next_allowed: Option<std::time::Instant>,
    pub(super) delay: Duration,
}

impl Fetcher {
    pub fn new(
        cache_dir: impl AsRef<Path>,
        user_agent: Option<String>,
        default_delay: Duration,
        host_delays: HashMap<String, Duration>,
        authorized_hosts: Vec<String>,
    ) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&cache_dir)
            .with_context(|| format!("creating cache dir {}", cache_dir.display()))?;
        let user_agent = user_agent.unwrap_or_else(|| DEFAULT_USER_AGENT.to_string());
        let client = reqwest::Client::builder()
            .user_agent(user_agent.clone())
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .context("building HTTP client")?;
        Ok(Self {
            client,
            cache_dir,
            user_agent,
            default_delay,
            host_delays,
            authorized_hosts: authorized_hosts
                .into_iter()
                .map(|host| host.trim().to_ascii_lowercase())
                .filter(|host| !host.is_empty())
                .collect(),
            hosts: Mutex::new(HashMap::new()),
            robots: Mutex::new(HashMap::new()),
            stats: Mutex::new(FetchStats::default()),
        })
    }
}
