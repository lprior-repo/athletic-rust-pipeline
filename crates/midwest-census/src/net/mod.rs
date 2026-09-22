//! Polite, resumable HTTP layer.
//!
//! Properties the rest of the crate relies on:
//!
//! * **robots.txt is enforced**, not advisory: a disallowed path returns [`FetchError::Robots`] and
//!   is counted in [`FetchStats::robots_blocked`] — unless the operator named that host on
//!   `--authorized-host`, in which case the rule is counted in [`FetchStats::robots_authorized`]
//!   and the request proceeds under the 2 rps ceiling below. Default: no host is authorized.
//! * **Hard pacing: 2 requests/second per host** — an authorized host never gets closer spacing than
//!   `MIN_AUTHORIZED_DELAY`, whatever `--delay-ms` says.
//! * Robots rules never leave the process. The rule set is fetched once per host, cached on disk and
//!   honoured for the remainder of the run.
//! * **Requests are cached on disk** by content hash, so a re-run is free and interrupted collections
//!   resume without re-fetching. Conditional GETs (`If-None-Match` / `If-Modified-Since`) are used when
//!   the origin supports them.
//! * **Politeness is per host**: at most one in-flight request per host, minimum spacing between
//!   requests (default 1 s, overridable per host — e.g. Bound's `Crawl-delay: 10`).
//! * No authentication, no cookie jar, no CAPTCHA handling, no challenge evasion.
//!
//! # Concurrency and timing guarantees
//!
//! * All shared state (hosts, robots, stats) uses `tokio::sync::Mutex` — no `std::sync::Mutex` is
//!   ever held across an `.await`.
//! * Every network request is guarded by a per-request timeout (default 45 s) and retries with
//!   bounded exponential backoff + jitter (max 3 attempts, 500 ms base delay).
//! * Response bodies are capped at 32 MiB; oversized responses return [`FetchError::TooLarge`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::Mutex;

use census_domain::model::{AccessBlockKind, SourceAccessCondition};

mod cache;
mod client;
mod decode;
mod execute;
mod request;
mod robots;
mod types;

pub use types::{
    cooldown_until_iso8601, now_iso8601, today_iso, FetchError, FetchOptions, FetchOutcome,
    FetchStats,
};

use client::HostState;
use robots::RobotsRules;

#[cfg(test)]
use request::jittered_delay;
#[cfg(test)]
use robots::parse_robots;

pub const DEFAULT_USER_AGENT: &str =
    "midwest-census/0.1 (independent HS track & field research collector; polite; contact: repo owner)";

const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;
const REQUEST_TIMEOUT_SECS: u64 = 45;
const MAX_RETRIES: u32 = 3;
const RETRY_BASE_DELAY_MS: u64 = 500;

/// Hard pacing ceiling for an explicitly authorized host: 2 requests/second.
///
/// `robots.txt` stays authoritative by default. `--authorized-host` records an operator decision
/// that one host's rules are logged rather than enforced, because the operator has authorized that
/// host explicitly. Speed is *not* part of that authorization: the ceiling the collection policy
/// states (2 rps per host) is applied as a floor on spacing for every authorized host, so raising
/// `--delay-ms` never lets an authorized host be hit faster than the policy allows.
const MIN_AUTHORIZED_DELAY: Duration = Duration::from_millis(500);

/// How long a hard access block stays in force when the source published no `Retry-After`.
///
/// §69 of the mission brief: a lane stops on a hard access block, persists the condition and leaves
/// the source alone. Six hours is the default the operator can re-derive sooner by running the lane
/// again after deleting the row; the row is what stops the next run paying for the same refusal
/// request by request.
pub const BLOCK_COOLDOWN_SECONDS: u64 = 6 * 60 * 60;

/// The source slug a fetcher carries until a lane stamps its own.
const DEFAULT_SOURCE: &str = "unknown";

// ---------------------------------------------------------------------------
// Fetcher
// ---------------------------------------------------------------------------

/// Polite, cache-first, per-host-rate-limited HTTP fetcher.
pub struct Fetcher {
    client: reqwest::Client,
    cache_dir: PathBuf,
    user_agent: String,
    default_delay: Duration,
    host_delays: HashMap<String, Duration>,
    /// Source families charged to one shared budget across every host below them. A family key
    /// matches its own host and anything under it, the way an authorized host does.
    family_delays: HashMap<String, Duration>,
    families: Mutex<HashMap<String, HostState>>,
    /// Hosts whose robots rules are recorded rather than enforced (operator authorization).
    /// A bare domain authorizes its subdomains.
    authorized_hosts: Vec<String>,
    hosts: Mutex<HashMap<String, HostState>>,
    robots: Mutex<HashMap<String, RobotsRules>>,
    stats: Mutex<FetchStats>,
    /// The adapter slug stamped into every access condition this fetcher records.
    source: String,
    /// Access conditions observed in this run, keyed by row id, so a repeated block refreshes one
    /// row instead of minting a second.
    blocks: Mutex<HashMap<String, SourceAccessCondition>>,
}

impl Fetcher {
    /// Whether the operator explicitly authorized this host.
    ///
    /// An entry authorizes exactly the host it names plus anything below it: `athletic.net`
    /// authorizes `www.athletic.net` and `api.athletic.net`, while `www.example.com` authorizes only
    /// itself and its own subdomains — never the parent domain, so naming a narrow host can never
    /// widen into a whole site.
    pub fn is_authorized_host(&self, host: &str) -> bool {
        let host = host.trim().to_ascii_lowercase();
        self.authorized_hosts
            .iter()
            .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
    }

    /// Charge every host below each key to one shared budget instead of its own (§10).
    ///
    /// A source that enforces its ceiling per *client* cannot be paced by a per-host gate. MileSplit
    /// refuses every `*.milesplit.com` host at once, while the walk's per-host model holds one
    /// budget per state subdomain: the 2026-09-22 national fan-out therefore spent fifty-one
    /// independent budgets against one enforced one and was refused after ~127 rosters per state,
    /// which is the observation recorded in `var/midwest-census/out/run-evidence/source-refusal.txt`.
    /// A family entry replaces the per-host budget for its hosts, so the family shares one gate and
    /// one spacing no matter how many subdomains the run spreads across.
    pub fn with_family_budgets(mut self, families: HashMap<String, Duration>) -> Self {
        self.family_delays = families
            .into_iter()
            .map(|(family, delay)| (family.trim().to_ascii_lowercase(), delay))
            .filter(|(family, _)| !family.is_empty())
            .collect();
        self
    }

    /// The shared-budget family `host` belongs to, when the run configured one.
    ///
    /// The longest matching key wins, so a run can pace `example.com` as a family and still give
    /// `slow.example.com` its own narrower entry without the family silently overriding it.
    pub(super) fn family_of(&self, host: &str) -> Option<String> {
        let host = host.trim().to_ascii_lowercase();
        self.family_delays
            .keys()
            .filter(|family| host == family.as_str() || host.ends_with(&format!(".{family}")))
            .max_by_key(|family| family.len())
            .cloned()
    }

    /// The spacing configured for a family key, if the run named one.
    pub(super) fn family_delay(&self, family: &str) -> Option<Duration> {
        self.family_delays.get(family).copied()
    }

    /// Borrow the user-agent string without taking ownership.
    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    /// Borrow the cache directory path.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Snapshot current fetch statistics.
    #[tracing::instrument(skip(self))]
    pub async fn stats(&self) -> FetchStats {
        self.stats.lock().await.clone()
    }

    /// Stamp the adapter slug into every access condition this fetcher records.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    /// Record (or refresh) the access condition one host imposed, and return the row.
    ///
    /// Called from the fetch path the moment a blocking status is seen, so the whole run shares one
    /// answer to "is this host refusing us?" instead of re-discovering it request by request. §69:
    /// the observation is what a lane stops on and what a later run reads before spending anything.
    pub async fn record_access_condition(
        &self,
        host: &str,
        kind: AccessBlockKind,
        status: u16,
        retry_after_seconds: Option<u64>,
        detail: impl Into<String>,
    ) -> SourceAccessCondition {
        let host = host.trim().to_ascii_lowercase();
        let condition = SourceAccessCondition::new(
            self.source.clone(),
            host,
            kind,
            status,
            now_iso8601(),
            detail,
        )
        .with_retry_after(retry_after_seconds)
        .with_cooldown_until(Some(cooldown_until_iso8601(
            retry_after_seconds.unwrap_or(BLOCK_COOLDOWN_SECONDS),
        )));
        let mut blocks = self.blocks.lock().await;
        blocks.insert(condition.id.clone(), condition.clone());
        condition
    }

    /// Every access condition observed in this run, sorted by row id.
    pub async fn access_conditions(&self) -> Vec<SourceAccessCondition> {
        let blocks = self.blocks.lock().await;
        let mut rows: Vec<SourceAccessCondition> = blocks.values().cloned().collect();
        rows.sort_by(|left, right| left.id.cmp(&right.id));
        rows
    }

    /// Hosts whose condition still blocks work at `now_iso8601`, sorted and deduplicated.
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

    /// Whether one host still blocks work at `now_iso8601`.
    pub async fn host_blocked(&self, host: &str, now_iso8601: &str) -> bool {
        let host = host.trim().to_ascii_lowercase();
        let blocks = self.blocks.lock().await;
        blocks
            .values()
            .any(|condition| condition.host == host && condition.is_blocking(now_iso8601))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
