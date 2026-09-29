use super::{FetchError, FetchStats, Fetcher, DEFAULT_USER_AGENT, REQUEST_TIMEOUT_SECS};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

pub(super) struct HostState {
    pub(super) gate: Arc<Mutex<()>>,
    pub(super) next_allowed: Option<tokio::time::Instant>,
    pub(super) delay: Duration,
}

pub struct PacingState {
    pub(super) families: Mutex<HashMap<String, HostState>>,
    pub(super) hosts: Mutex<HashMap<String, HostState>>,
}

impl PacingState {
    pub fn new() -> Self {
        Self {
            families: Mutex::new(HashMap::new()),
            hosts: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for PacingState {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetcher {
    pub fn new(
        cache_dir: impl AsRef<Path>,
        user_agent: Option<String>,
        default_delay: Duration,
        host_delays: HashMap<String, Duration>,
        authorized_hosts: Vec<String>,
    ) -> Result<Self, FetchError> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&cache_dir).map_err(|source| FetchError::Cache {
            path: cache_dir.clone(),
            source,
        })?;
        let user_agent = user_agent.unwrap_or_else(|| DEFAULT_USER_AGENT.to_string());
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|source| FetchError::Client { source })?;
        Ok(Self {
            client,
            cache_dir,
            default_delay,
            host_delays,
            family_delays: HashMap::new(),
            pacing: Arc::new(PacingState::new()),
            authorized_hosts: authorized_hosts
                .into_iter()
                .map(|host| host.trim().to_ascii_lowercase())
                .filter(|host| !host.is_empty())
                .collect(),
            robots: Mutex::new(HashMap::new()),
            robots_gates: Mutex::new(HashMap::new()),
            stats: Mutex::new(FetchStats::default()),
            source: super::DEFAULT_SOURCE.to_string(),
            blocks: Mutex::new(HashMap::new()),
            lane: None,
            offline: false,
        })
    }
}
