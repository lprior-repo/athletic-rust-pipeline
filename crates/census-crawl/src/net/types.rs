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

use census_store::clock::{Clock, SystemClock};
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
// Time helpers
// ---------------------------------------------------------------------------

/// Wall-clock timestamp for request evidence and cache metadata.
///
/// Delegates to the [`Clock`] capability so the crate has one source of wall-clock time: the
/// signatures stay as they are, because their callers — cache writes, decode, the collection
/// default — carry no clock of their own to inject.
pub fn now_iso8601() -> String {
    SystemClock.today_iso8601()
}

/// Today's date (`YYYY-MM-DD`), the default `observed_on` for a collection.
pub fn today_iso() -> String {
    SystemClock.today()
}

/// The instant a cooldown that starts now stops applying (RFC 3339 UTC, `Z`).
///
/// The one place a cooldown instant is computed, kept beside [`now_iso8601`] so the two timestamps a
/// condition carries are produced the same way. Clamped rather than panicking: a cooldown the clock
/// cannot represent is expressed as the far future, which is the honest reading of "blocked".
pub fn cooldown_until_iso8601(seconds: u64) -> String {
    let seconds = i64::try_from(seconds).unwrap_or(i64::MAX);
    chrono::Utc::now()
        .checked_add_signed(chrono::Duration::seconds(seconds))
        .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// One instant read from unix milliseconds (RFC 3339 UTC, `Z`), when it is one this clock can state.
///
/// The browser lane's capture carries the instant it was taken as milliseconds, and the evidence
/// written from it wants the same shape the HTTP path writes: the format is [`now_iso8601`]'s, so a
/// receipt does not say which transport produced it. `None` for a value that is not an instant —
/// absence is then the caller's decision, the same way an absent `Retry-After` is.
pub fn instant_iso8601(millis: u64) -> Option<String> {
    let millis = i64::try_from(millis).ok()?;
    chrono::DateTime::from_timestamp_millis(millis)
        .map(|instant| instant.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}
