//! The vocabulary the fetcher speaks: what a fetch can fail with, what it was asked for, what it
//! returns, what it counted, and the timestamps those carry.
//!
//! Everything here is data. The state machine that produces it is [`super::Fetcher`]; keeping the
//! two apart means a caller can read the contract of a fetch — its errors, its options, its
//! statistics — without reading the machinery that enforces robots and per-host pacing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

use super::MAX_BODY_BYTES;

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
    /// The browser lane did not hand back a capture.
    ///
    /// One variant rather than one per verdict: the lane's own words travel in `detail`, and
    /// `retryable` is the verdict *it* carried — the transport classified the failure, so the census
    /// does not re-derive a classification from it. The access-condition row the refusal recorded is
    /// the authority on why (§69): `browser_unavailable:{host}` for a lane that is not there,
    /// `human_required:{host}` for a profile that wants a person.
    #[error("browser lane refused {url}: {detail}")]
    BrowserLane {
        url: String,
        detail: String,
        retryable: bool,
    },
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
    /// A request violated a transport policy: wrong profile on the browser lane, or a URL whose
    /// origin is not admitted to the browser transport.
    ///
    /// The detail names the conflict so an operator or a test can distinguish profile-mismatch
    /// from origin-excluded.  The taxonomy keeps `Policy` below `Invariant` because the caller
    /// is wrong (it asked the browser for the wrong origin), but the error is *not* a bug — it is
    /// the policy enforcement itself.
    #[error("policy: {detail}")]
    Policy { detail: String },
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
            Self::BrowserLane { retryable, .. } => *retryable,
            Self::Http { status, .. } => *status >= 500 || *status == 429,
            Self::Robots(_)
            | Self::TooLarge { .. }
            | Self::Cache { .. }
            | Self::InvalidUrl { .. }
            | Self::Decode { .. }
            | Self::Encode { .. }
            | Self::Client { .. }
            | Self::Policy { .. }
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

use super::latency::LATENCY_BUCKET_COUNT;

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
    /// Refusals that named a rate limit (§45's `429s`): the origin answered, and the answer was
    /// "slow down". Counted apart from `errors` because a throttled origin is a pace finding, not a
    /// defect in the transport.
    pub rate_limited: u64,
    /// Requests that reached their own deadline (§45's `timeouts`), likewise apart from `errors`.
    pub timeouts: u64,
    /// Challenges an origin served that only a person can answer (§45's `challenges`, §28's
    /// `HumanRequired`). A non-zero count is the reason a lane stopped, so it is never folded into
    /// the failure total.
    pub challenges: u64,
    /// Sum of every measured transport latency in milliseconds, and how many were measured, so the
    /// average is exact where the percentiles are bucketed.
    pub latency_ms_sum: u64,
    pub latency_ms_max: u64,
    pub latency_buckets: [u64; LATENCY_BUCKET_COUNT],
    /// What each origin saw from this client (§45's per-provider record).
    ///
    /// Keyed by host, never by URL: §10's admission budgets exist per remote origin, so a counter
    /// keyed by path would split one origin's traffic into as many rows as it has pages.
    pub per_host: HashMap<String, HostTraffic>,
}

/// What one origin saw from this client: §45's `requests`, `cache hits` and `bytes` for a provider.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostTraffic {
    /// Every fetch that named this host, cached or not — §45's `requests`.
    pub requests: u64,
    /// The share of those served from the cache. `requests - cache_hits` is what the origin itself
    /// saw: §45's `physical requests`, and the denominator of the efficiency metric.
    pub cache_hits: u64,
    /// Body bytes attributed to this host.
    pub bytes: u64,
}

impl HostTraffic {
    /// Requests that reached this origin (§45's `physical requests`).
    pub fn physical_requests(&self) -> u64 {
        self.requests.saturating_sub(self.cache_hits)
    }
}

impl FetchStats {
    /// Requests that reached the origin: every fetch that was not served from the cache.
    ///
    /// This is §45's *physical* request count, and the denominator of the one efficiency metric the
    /// objective names. It is derived rather than counted so the two halves cannot disagree.
    pub fn physical_requests(&self) -> u64 {
        self.requests.saturating_sub(self.cache_hits)
    }

    /// Requests the origin answered without an error.
    pub fn successful_requests(&self) -> u64 {
        self.requests.saturating_sub(self.errors)
    }

    /// §45's headline efficiency metric: verified useful records per physical request.
    ///
    /// `None` where no physical request was made, because the ratio of something to nothing is not
    /// zero — a cached run must not read as an infinitely efficient one.
    pub fn useful_records_per_physical_request(&self, records: u64) -> Option<f64> {
        let physical = self.physical_requests();
        if physical == 0 {
            return None;
        }
        // Converted through `u32` so the ratio needs no `as` cast: a count past four billion does
        // not occur in a census run, and reporting no ratio beats reporting an approximate one.
        let records = u32::try_from(records).ok()?;
        let physical = u32::try_from(physical).ok()?;
        Some(f64::from(records) / f64::from(physical))
    }
}

// ---------------------------------------------------------------------------
// Request keys
// ---------------------------------------------------------------------------

/// The host a request URL names, so the per-origin counters cannot be keyed by a whole URL.
///
/// `reqwest::Url` does the parsing. A URL it cannot parse — or one with no host, like `data:` —
/// keeps its own text as the key, which keeps the request counted rather than silently dropped.
pub(crate) fn host_of(url: &str) -> String {
    match reqwest::Url::parse(url) {
        Ok(parsed) => parsed
            .host_str()
            .map(str::to_string)
            .unwrap_or_else(|| url.to_string()),
        Err(_) => url.to_string(),
    }
}
