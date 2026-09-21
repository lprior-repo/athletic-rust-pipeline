//! Request construction (verbs, bodies, conditional GET) and the retry backoff policy.

use super::cache::CacheMeta;
use super::{
    FetchError, FetchOptions, FetchOutcome, Fetcher, REQUEST_TIMEOUT_SECS, RETRY_BASE_DELAY_MS,
};
use std::time::Duration;

/// Request payload for POSTs: a pre-serialized JSON body.
#[derive(Debug, Clone)]
pub(super) enum RequestBody {
    Json(String),
}

impl Fetcher {
    /// GET a URL with caching, robots enforcement and per-host politeness.
    #[tracing::instrument(skip(self, options), fields(url, method = "GET"))]
    pub async fn get(&self, url: &str, options: &FetchOptions) -> Result<FetchOutcome, FetchError> {
        tracing::Span::current().record("url", url);
        self.fetch("GET", url, None, options, REQUEST_TIMEOUT_SECS)
            .await
    }

    /// POST a JSON body; cached by body content so repeated runs are free.
    ///
    /// Elasticsearch-backed result platforms take the query in the body, so the cache key must
    /// include that body: two different queries against one endpoint are two different documents.
    #[tracing::instrument(skip(self, options, body), fields(url, method = "POST"))]
    pub async fn post_json(
        &self,
        url: &str,
        body: &serde_json::Value,
        options: &FetchOptions,
    ) -> Result<FetchOutcome, FetchError> {
        tracing::Span::current().record("url", url);
        let encoded = serde_json::to_string(body).map_err(|source| FetchError::Encode {
            target: url.to_string(),
            source,
        })?;
        self.fetch(
            "POST",
            url,
            Some((encoded.clone(), RequestBody::Json(encoded))),
            options,
            REQUEST_TIMEOUT_SECS,
        )
        .await
    }
}

/// Build an HTTP request with headers, body, and conditional GET support.
pub(super) fn build_request<'a>(
    client: &'a reqwest::Client,
    method: &str,
    url: &str,
    body: Option<&'a RequestBody>,
    headers: &[(String, String)],
    cached: Option<&'a CacheMeta>,
    refresh: bool,
) -> Result<reqwest::RequestBuilder, FetchError> {
    let mut request = match method {
        "POST" => client.post(url),
        _ => client.get(url),
    };
    for (name, value) in headers {
        request = request.header(name.as_str(), value.as_str());
    }
    if method == "POST" {
        if let Some(payload) = body {
            match payload {
                RequestBody::Json(encoded) => {
                    request = request
                        .header("content-type", "application/json")
                        .body(encoded.clone());
                }
            }
        }
    }
    if let Some(meta) = cached {
        if !refresh {
            if let Some(etag) = &meta.etag {
                request = request.header("If-None-Match", etag.as_str());
            }
            if let Some(last_modified) = &meta.last_modified {
                request = request.header("If-Modified-Since", last_modified.as_str());
            }
        }
    }
    Ok(request)
}

/// Backoff for a retry: 500 ms doubling per attempt, capped at ten seconds, plus deterministic
/// ±25% jitter so parallel fetchers do not retry in lockstep.
///
/// The jitter is a splitmix64 mix of the attempt number: no random source, no allocation, and the
/// same attempt always yields the same delay, which keeps tests and replays repeatable.
pub(super) fn jittered_delay(attempt: u32) -> Duration {
    let step = attempt.saturating_sub(1).min(4);
    let cap_ms: u64 = 10_000;
    // Doubling per attempt, capped: `saturating_pow` cannot overflow and `step` is at most 4.
    let base_ms = RETRY_BASE_DELAY_MS
        .saturating_mul(2_u64.saturating_pow(step))
        .min(cap_ms);
    let spread = base_ms / 2;
    if spread == 0 {
        return Duration::from_millis(base_ms);
    }
    // The jitter is a value in `0..spread` pulled back by half a spread, so the delay lands
    // within ±25% of `base` and never leaves `0..=cap`. Subtracting the shortfall and adding the
    // excess of that shift keeps the whole calculation unsigned and saturating: nothing can
    // overflow, and the shortfall saturates at zero exactly where the old clamp did.
    let half = spread / 2;
    // `spread` is non-zero here, so the remainder is always defined; the fallback keeps the
    // calculation total without a panic path.
    let jitter = mix_attempt(attempt).checked_rem(spread).unwrap_or(0);
    let delay_ms = base_ms
        .saturating_sub(half.saturating_sub(jitter))
        .saturating_add(jitter.saturating_sub(half))
        .min(cap_ms);
    Duration::from_millis(delay_ms)
}

/// splitmix64 over one counter: cheap, deterministic, and dependency-free.
fn mix_attempt(attempt: u32) -> u64 {
    let mut z = u64::from(attempt).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
