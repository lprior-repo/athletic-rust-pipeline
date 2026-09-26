//! Evidence building for browser captures: decoding, hashing, caching, and outcome construction.
//!
//! This module holds the body of `mint_capture`, `handle_404_capture`, and `count_capture` — the part
//! of the seat that turns a captured response into evidence. The transport classifies; this code
//! records the classification's consequences: a verified body goes to cache and becomes
//! [`FetchOutcome`], a status the run must stop on bumps an error counter.

use super::FetchPlan;
use crate::net::bridge::BrowserCapture;
use crate::net::cache::{write_cache, CacheMeta};
use crate::net::{instant_iso8601, now_iso8601, FetchError, FetchOutcome, Fetcher, MAX_BODY_BYTES};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use sha2::{Digest, Sha256};
use tracing::warn;

/// Decode one capture's body, refuse it before it is allocated when it is over the ceiling.
///
/// Base64 is four characters per three bytes, plus padding: refusing on the encoded length keeps
/// a body over the ceiling from being allocated only to be thrown away.
pub(super) fn decode_capture_body(
    plan: &FetchPlan<'_>,
    capture: &BrowserCapture,
) -> Result<Vec<u8>, FetchError> {
    if capture.response.body.len() > MAX_BODY_BYTES / 3 * 4 + 4 {
        return Err(FetchError::TooLarge {
            url: plan.url.to_string(),
        });
    }
    let body = BASE64
        .decode(capture.response.body.as_bytes())
        .map_err(|error| FetchError::Invariant {
            detail: format!("browser lane body for {} is not base64 ({error})", plan.url),
        })?;
    if body.len() > MAX_BODY_BYTES {
        return Err(FetchError::TooLarge {
            url: plan.url.to_string(),
        });
    }
    Ok(body)
}

/// The content type a capture published, when it published one.
pub(super) fn content_type(headers: &[(String, String)]) -> Option<String> {
    headers
        .iter()
        .rev()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.clone())
}

impl Fetcher {
    /// Mint the evidence one capture carries, in the shape a response body gets.
    ///
    /// The cache write comes first, exactly as it does for HTTP: a body the source served is worth
    /// keeping even when the status that carried it is one the run must stop on.
    pub(super) async fn mint_capture(
        &self,
        plan: &FetchPlan<'_>,
        capture: BrowserCapture,
    ) -> Result<FetchOutcome, FetchError> {
        let status = capture.response.status;
        let body = decode_capture_body(plan, &capture)?;
        let mut hasher = Sha256::new();
        hasher.update(&body);
        let digest = hasher.finalize();
        let key_hex: String = digest
            .get(..16)
            .unwrap_or(digest.as_slice())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let content_hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        let bytes = body.len();
        let content_type = content_type(&capture.response.headers);
        let fetched_at = capture
            .fetched_at_ms
            .and_then(instant_iso8601)
            .unwrap_or_else(now_iso8601);
        let meta = CacheMeta {
            url: plan.url.to_string(),
            method: plan.method.to_string(),
            status,
            key_prefix: key_hex.clone(),
            content_digest: content_hex,
            bytes,
            fetched_at: fetched_at.clone(),
            etag: None,
            last_modified: None,
            content_type: content_type.clone(),
        };
        write_cache(plan.body_path, plan.meta_path, &body, &meta)?;
        self.count_capture(plan, status, bytes).await;
        Ok(FetchOutcome {
            url: plan.url.to_string(),
            method: plan.method.to_string(),
            status,
            sha256: key_hex,
            bytes,
            fetched_at,
            from_cache: false,
            content_type,
            body,
        })
    }

    /// Handle a 404 capture: cache the evidence, then return `Ok` or `Err` depending on
    /// `allow_not_found`.
    ///
    /// The body and metadata are always cached — a 404 is a real answer the run should remember.
    /// When `allow_not_found` is `true`, the caller expects the 404 as a normal outcome.
    /// When `false`, the caller wants a 404 to propagate as an error.
    pub(super) async fn handle_404_capture(
        &self,
        plan: &FetchPlan<'_>,
        capture: BrowserCapture,
    ) -> Result<FetchOutcome, FetchError> {
        let status = 404u16;
        let body = decode_capture_body(plan, &capture)?;
        let mut hasher = Sha256::new();
        hasher.update(&body);
        let digest = hasher.finalize();
        let key_hex: String = digest
            .get(..16)
            .unwrap_or(digest.as_slice())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let content_hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        let bytes = body.len();
        let content_type = content_type(&capture.response.headers);
        let fetched_at = capture
            .fetched_at_ms
            .and_then(instant_iso8601)
            .unwrap_or_else(now_iso8601);
        let meta = CacheMeta {
            url: plan.url.to_string(),
            method: plan.method.to_string(),
            status,
            key_prefix: key_hex.clone(),
            content_digest: content_hex,
            bytes,
            fetched_at: fetched_at.clone(),
            etag: None,
            last_modified: None,
            content_type: content_type.clone(),
        };
        write_cache(plan.body_path, plan.meta_path, &body, &meta)?;
        self.count_capture(plan, status, bytes).await;
        if plan.options.allow_not_found {
            Ok(FetchOutcome {
                url: plan.url.to_string(),
                method: plan.method.to_string(),
                status,
                sha256: key_hex,
                bytes,
                fetched_at,
                from_cache: false,
                content_type,
                body,
            })
        } else {
            Err(FetchError::Http {
                status,
                url: plan.url.to_string(),
            })
        }
    }

    /// Count one accepted capture: the request that carried it, the bytes it carried, and the
    /// status it arrived under.
    ///
    /// The browser lane's mirror of `record_request_stats`, and the reason `count_request` skips a
    /// `200` or `404`: a body is counted where it is written down, once. One lock covers the whole
    /// update, and the per-provider row (§45) is keyed by the plan's host, so a browser-transported
    /// source's traffic lands in the same table an HTTP source's does.
    async fn count_capture(&self, plan: &FetchPlan<'_>, status: u16, bytes: usize) {
        let downloaded = u64::try_from(bytes).unwrap_or(u64::MAX);
        let failed = status >= 400 && !(status == 404 && plan.options.allow_not_found);
        {
            let mut stats = self.stats.lock().await;
            stats.requests = stats.requests.saturating_add(1);
            stats.bytes_downloaded = stats.bytes_downloaded.saturating_add(downloaded);
            let entry = stats.per_host.entry(plan.host.to_string()).or_default();
            entry.requests = entry.requests.saturating_add(1);
            entry.bytes = entry.bytes.saturating_add(downloaded);
            if failed {
                stats.errors = stats.errors.saturating_add(1);
            }
        }
        if failed {
            warn!(status, url = plan.url, "non-success response");
        }
    }
}
