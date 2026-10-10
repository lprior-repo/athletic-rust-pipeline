use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

use super::MAX_BODY_BYTES;

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("http status {status} for {url}")]
    Http { status: u16, url: String },
    #[error("http 429 for {url} (retry-after: {retry_after_secs:?}s)")]
    RateLimited {
        url: String,
        retry_after_secs: Option<u64>,
    },
    #[error("host {host} is inside a recorded access cooldown")]
    Cooldown { host: String },
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
    #[error("origin {origin} is held by another census-service process (holder: {holder})")]
    OriginHeld { origin: String, holder: String },
    #[error("origin lock i/o for {path}: {source}")]
    OriginLockIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("request timed out for {url} after {timeout_secs}s")]
    Timeout { url: String, timeout_secs: u64 },
    #[error("browser lane refused {url}: {detail}")]
    BrowserLane {
        url: String,
        detail: String,
        retryable: bool,
    },
    #[error("invalid url {url}: {source}")]
    InvalidUrl {
        url: String,
        #[source]
        source: url::ParseError,
    },
    #[error("json decode failed for {target}: {source}")]
    Decode {
        target: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("json encode failed for {target}: {source}")]
    Encode {
        target: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("http client build failed: {source}")]
    Client {
        #[source]
        source: reqwest::Error,
    },
    #[error("{detail}")]
    Invariant { detail: String },
    #[error("policy: {detail}")]
    Policy { detail: String },
    #[error("offline and not cached: {url}")]
    Offline { url: String },
}

impl FetchError {
    pub fn retryable(&self) -> bool {
        match self {
            Self::Transport { .. }
            | Self::Timeout { .. }
            | Self::RateLimited { .. }
            | Self::Cooldown { .. } => true,
            Self::BrowserLane { retryable, .. } => *retryable,
            Self::Http { status, .. } => *status >= 500 || *status == 429,
            Self::TooLarge { .. }
            | Self::Cache { .. }
            | Self::OriginHeld { .. }
            | Self::OriginLockIo { .. }
            | Self::InvalidUrl { .. }
            | Self::Decode { .. }
            | Self::Encode { .. }
            | Self::Client { .. }
            | Self::Policy { .. }
            | Self::Invariant { .. }
            | Self::Offline { .. } => false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FetchOptions {
    pub refresh: bool,
    pub allow_not_found: bool,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedirectHop {
    pub status: u16,
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLane {
    Http,
    Browser,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefusalEvidenceLoss {
    pub source: String,
    pub url: String,
    pub status: u16,
    pub lane: EvidenceLane,
    pub detail: String,
    pub observed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchOutcome {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_url: Option<String>,
    pub method: String,
    pub status: u16,
    pub content_digest: String,
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
    pub bytes_downloaded: u64,
    pub errors: u64,
    pub rate_limited: u64,
    pub timeouts: u64,
    pub challenges: u64,
    pub latency_ms_sum: u64,
    pub latency_ms_max: u64,
    pub latency_buckets: [u64; LATENCY_BUCKET_COUNT],
    pub per_host: HashMap<String, HostTraffic>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostTraffic {
    pub requests: u64,
    pub cache_hits: u64,
    pub bytes: u64,
}

impl HostTraffic {
    pub fn physical_requests(&self) -> u64 {
        self.requests.saturating_sub(self.cache_hits)
    }
}

impl FetchStats {
    pub fn physical_requests(&self) -> u64 {
        self.requests.saturating_sub(self.cache_hits)
    }

    pub fn useful_records_per_physical_request(&self, records: u64) -> Option<f64> {
        let physical = self.physical_requests();
        if physical == 0 {
            return None;
        }
        let records = u32::try_from(records).ok()?;
        let physical = u32::try_from(physical).ok()?;
        Some(f64::from(records) / f64::from(physical))
    }
}

pub(crate) fn host_of(url: &str) -> String {
    match reqwest::Url::parse(url) {
        Ok(parsed) => match parsed.host_str().map(str::to_string) {
            Some(value) => value,
            None => url.to_string(),
        },
        Err(_) => url.to_string(),
    }
}
