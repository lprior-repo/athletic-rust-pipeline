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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;

mod cache;
mod client;
mod decode;
mod execute;
mod request;
mod robots;

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

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur during fetch operations.
#[derive(Debug, Error)]
pub enum FetchError {
    #[error("robots.txt disallows {0}")]
    Robots(String),
    #[error("http status {status} for {url}")]
    Http { status: u16, url: String },
    #[error("http 429 for {url} (retry-after: {retry_after_secs:?}s)")]
    RateLimited {
        url: String,
        retry_after_secs: Option<u64>,
    },
    #[error("response body for {url} exceeds {MAX_BODY_BYTES} bytes")]
    TooLarge { url: String },
    #[error("transport error for {url}: {source}")]
    Transport {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("cache i/o for {path}: {source}")]
    Cache {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("request timed out for {url} after {timeout_secs}s")]
    Timeout { url: String, timeout_secs: u64 },
    /// A request URL could not be parsed into its host, origin and path.
    #[error("invalid url {url}: {source}")]
    InvalidUrl {
        url: String,
        #[source]
        source: url::ParseError,
    },
    /// JSON did not decode.
    #[error("json decode failed for {target}: {source}")]
    Decode {
        /// The request URL for a response body, or the path of a cache artifact.
        target: String,
        #[source]
        source: serde_json::Error,
    },
    /// A JSON payload could not be encoded.
    #[error("json encode failed for {target}: {source}")]
    Encode {
        /// The request URL for a request body, or the path of a cache artifact.
        target: String,
        #[source]
        source: serde_json::Error,
    },
    /// The HTTP client could not be constructed.
    #[error("http client build failed: {source}")]
    Client {
        #[source]
        source: reqwest::Error,
    },
    /// An internal invariant was violated (a bug, not external input).
    ///
    /// Display is the carried message verbatim: the taxonomy's convention is that a message a
    /// test, golden or operator reads keeps its exact text.
    #[error("{detail}")]
    Invariant { detail: String },
}

impl FetchError {
    /// Whether another attempt can plausibly succeed.
    ///
    /// This is the *intrinsic* classification: status-driven decisions that depend on
    /// [`FetchOptions`] (`allow_not_found`, `refresh`) stay in the retry loop, which keeps its own
    /// policy.
    pub fn retryable(&self) -> bool {
        match self {
            Self::Transport { .. } | Self::Timeout { .. } | Self::RateLimited { .. } => true,
            Self::Http { status, .. } => *status >= 500 || *status == 429,
            Self::Robots(_)
            | Self::TooLarge { .. }
            | Self::Cache { .. }
            | Self::InvalidUrl { .. }
            | Self::Decode { .. }
            | Self::Encode { .. }
            | Self::Client { .. }
            | Self::Invariant { .. } => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

/// Options controlling fetch behaviour.
#[derive(Debug, Clone, Default)]
pub struct FetchOptions {
    /// Ignore any cached body and hit the network (still robots-checked).
    pub refresh: bool,
    /// Treat a 404 as a normal (cached) outcome instead of an error.
    pub allow_not_found: bool,
    /// Extra request headers (e.g. `Accept: application/json`).
    pub headers: Vec<(String, String)>,
}

/// Outcome of a single fetch. The `body` field carries the raw bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchOutcome {
    pub url: String,
    pub method: String,
    pub status: u16,
    pub sha256: String,
    pub bytes: usize,
    pub fetched_at: String,
    pub from_cache: bool,
    pub content_type: Option<String>,
    #[serde(skip)]
    pub body: Vec<u8>,
}

impl FetchOutcome {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }

    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> Result<T, FetchError> {
        serde_json::from_slice(&self.body).map_err(|source| FetchError::Decode {
            target: self.url.clone(),
            source,
        })
    }
}

/// Aggregated fetch statistics.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FetchStats {
    pub requests: u64,
    pub cache_hits: u64,
    pub conditional_304: u64,
    pub robots_blocked: u64,
    /// Requests that a robots rule disallowed but an explicit host authorization permitted. Kept
    /// separate from `robots_blocked` so the run record shows exactly what was overridden.
    pub robots_authorized: u64,
    pub bytes_downloaded: u64,
    pub errors: u64,
    pub per_host: HashMap<String, u64>,
}

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
    /// Hosts whose robots rules are recorded rather than enforced (operator authorization).
    /// A bare domain authorizes its subdomains.
    authorized_hosts: Vec<String>,
    hosts: Mutex<HashMap<String, HostState>>,
    robots: Mutex<HashMap<String, RobotsRules>>,
    stats: Mutex<FetchStats>,
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
}

// ---------------------------------------------------------------------------
// Time helpers
// ---------------------------------------------------------------------------

pub fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn today_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
