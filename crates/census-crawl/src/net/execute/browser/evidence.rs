//! Evidence building for browser captures: decoding, hashing, caching, and outcome construction.
//!
//! This module holds the body of `mint_capture`, `handle_404_capture`, and `count_body` — the part
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
            // The transport's own codec writes this field by construction, so a body that does
            // not decode did not come from the lane's codec: fail loudly, mint nothing.
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
        // The transport stamps the capture; a stamp that is not an instant falls back to this
        // process's clock rather than dropping the receipt.
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
            // The lane sends no conditional headers, so it has neither validator to carry forward.
            etag: None,
            last_modified: None,
            content_type: content_type.clone(),
        };
        write_cache(plan.body_path, plan.meta_path, &body, &meta)?;
        self.count_body(plan, status, bytes).await;
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
        self.count_body(plan, status, bytes).await;
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

    /// Count the body one accepted capture carried, and flag the status it arrived under.
    ///
    /// The accounting is the one `process_response` keeps for an HTTP body: the bytes a source
    /// served, and an error for a status the run must stop on — a `404` the run allowed is an
    /// answer, not an error.
    async fn count_body(&self, plan: &FetchPlan<'_>, status: u16, bytes: usize) {
        {
            let mut stats = self.stats.lock().await;
            stats.bytes_downloaded = stats
                .bytes_downloaded
                .saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
        }
        if status >= 400 && !(status == 404 && plan.options.allow_not_found) {
            let mut stats = self.stats.lock().await;
            stats.errors = stats.errors.saturating_add(1);
            warn!(status, url = plan.url, "non-success response");
        }
    }
}
