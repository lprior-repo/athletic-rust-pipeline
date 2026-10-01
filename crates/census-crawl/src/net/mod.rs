use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use census_domain::model::{AccessBlockKind, SourceAccessCondition};
use destination_guard::DestinationGuard;

pub mod bridge;

pub(crate) mod cache;
mod client;
mod destination_guard;
mod execute;
mod latency;
mod request;
mod robots;
mod time;
mod types;

pub use time::{cooldown_until_iso8601, instant_iso8601, now_iso8601, today_iso};
pub use types::{FetchError, FetchOptions, FetchOutcome, FetchStats, HostTraffic};

pub(crate) use types::host_of;

pub use client::PacingState;
use robots::RobotsPolicy;

pub const DEFAULT_USER_AGENT: &str =
    "census-service/0.1 (independent HS track & field research collector; polite; contact: repo owner)";

const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;
const REQUEST_TIMEOUT_SECS: u64 = 45;

const MIN_AUTHORIZED_DELAY: Duration = Duration::from_millis(500);

pub const BLOCK_COOLDOWN_SECONDS: u64 = 6 * 60 * 60;

pub const DEFAULT_FAMILY_PARALLELISM: usize = 1;

const DEFAULT_SOURCE: &str = "unknown";

pub struct Fetcher {
    client: reqwest::Client,
    destination: Arc<DestinationGuard>,
    cache_dir: PathBuf,
    default_delay: Duration,
    host_delays: HashMap<String, Duration>,
    family_delays: HashMap<String, Duration>,
    family_parallelism: usize,
    pacing: Arc<PacingState>,
    authorized_hosts: Vec<String>,
    robots: Mutex<HashMap<String, RobotsPolicy>>,
    robots_gates: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    stats: Mutex<FetchStats>,
    source: String,
    blocks: Mutex<HashMap<String, SourceAccessCondition>>,
    lane: Option<bridge::BrowserLane>,
    offline: bool,
}

impl Fetcher {
    pub fn is_authorized_host(&self, host: &str) -> bool {
        let host = host.trim().to_ascii_lowercase();
        self.authorized_hosts
            .iter()
            .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
    }

    pub fn with_family_budgets(mut self, families: HashMap<String, Duration>) -> Self {
        self.family_delays = families
            .into_iter()
            .map(|(family, delay)| (family.trim().to_ascii_lowercase(), delay))
            .filter(|(family, _)| !family.is_empty())
            .collect();
        self
    }

    pub fn with_family_parallelism(mut self, parallelism: usize) -> Self {
        self.family_parallelism = parallelism.max(DEFAULT_FAMILY_PARALLELISM);
        self
    }

    pub(super) fn family_of(&self, host: &str) -> Option<String> {
        let host = host.trim().to_ascii_lowercase();
        self.family_delays
            .keys()
            .filter(|family| host == family.as_str() || host.ends_with(&format!(".{family}")))
            .max_by_key(|family| family.len())
            .cloned()
    }

    pub(super) fn family_delay(&self, family: &str) -> Option<Duration> {
        self.family_delays.get(family).copied()
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    #[tracing::instrument(skip(self))]
    pub async fn stats(&self) -> FetchStats {
        self.stats.lock().await.clone()
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    pub fn with_browser_lane(mut self, lane: bridge::BrowserLane) -> Self {
        self.lane = Some(lane);
        self
    }

    pub fn with_shared_pacing(mut self, shared: Arc<PacingState>) -> Self {
        self.pacing = shared;
        self
    }

    pub fn pacing_state(&self) -> Arc<PacingState> {
        Arc::clone(&self.pacing)
    }

    pub fn has_browser_lane(&self) -> bool {
        self.lane.is_some()
    }
    pub fn with_offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    pub fn is_offline(&self) -> bool {
        self.offline
    }

    pub async fn record_access_condition(
        &self,
        host: &str,
        kind: AccessBlockKind,
        status: u16,
        retry_after_seconds: Option<u64>,
        detail: impl Into<String>,
    ) -> SourceAccessCondition {
        let host = host.trim().to_ascii_lowercase();
        let cooldown_until = match kind {
            AccessBlockKind::HumanRequired => None,
            _ => Some(cooldown_until_iso8601(
                retry_after_seconds.unwrap_or(BLOCK_COOLDOWN_SECONDS),
            )),
        };
        let condition = SourceAccessCondition::new(
            self.source.clone(),
            host,
            kind,
            status,
            now_iso8601(),
            detail,
        )
        .with_retry_after(retry_after_seconds)
        .with_cooldown_until(cooldown_until);
        if matches!(kind, AccessBlockKind::HumanRequired) {
            let mut stats = self.stats.lock().await;
            stats.challenges = stats.challenges.saturating_add(1);
        }
        let mut blocks = self.blocks.lock().await;
        blocks.insert(condition.id.clone(), condition.clone());
        condition
    }

    pub async fn access_conditions(&self) -> Vec<SourceAccessCondition> {
        let blocks = self.blocks.lock().await;
        let mut rows: Vec<SourceAccessCondition> = blocks.values().cloned().collect();
        rows.sort_by(|left, right| left.id.cmp(&right.id));
        rows
    }

    pub async fn blocked_hosts(&self, now_iso8601: &str) -> Vec<String> {
        let blocks = self.blocks.lock().await;
        let mut hosts: Vec<String> = blocks
            .values()
            .filter(|condition| condition.is_blocking(now_iso8601))
            .map(|condition| condition.host.clone())
            .collect();
        hosts.sort();
        hosts.dedup();
        hosts
    }

    pub async fn host_blocked(&self, host: &str, now_iso8601: &str) -> bool {
        let host = host.trim().to_ascii_lowercase();
        let blocks = self.blocks.lock().await;
        blocks
            .values()
            .any(|condition| condition.host == host && condition.is_blocking(now_iso8601))
    }
}

#[cfg(test)]
mod tests;
