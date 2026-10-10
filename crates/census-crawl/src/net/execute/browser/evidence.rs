use super::FetchPlan;
use crate::net::bridge::BrowserCapture;
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::{instant_iso8601, now_iso8601, FetchError, FetchOutcome, Fetcher, MAX_BODY_BYTES};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use tracing::warn;

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

pub(super) fn content_type(headers: &[(String, String)]) -> Option<String> {
    headers
        .iter()
        .rev()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.clone())
}

impl Fetcher {
    pub(super) async fn mint_capture(
        &self,
        plan: &FetchPlan<'_>,
        capture: BrowserCapture,
    ) -> Result<FetchOutcome, FetchError> {
        let status = capture.response.status;
        let body = decode_capture_body(plan, &capture)?;
        let content_hex = content_digest(&body);
        let bytes = body.len();
        let content_type = content_type(&capture.response.headers);
        let fetched_at = match capture.fetched_at_ms.and_then(instant_iso8601) {
            Some(value) => value,
            None => now_iso8601(),
        };
        let meta = CacheMeta {
        redirects: Vec::new(),
            url: plan.url.to_string(),
            response_url: capture.response.response_url.map(|url| url.into_string()),
            method: plan.method.to_string(),
            representation: plan.representation.clone(),
            status,
            content_digest: content_hex,
            bytes,
            fetched_at,
            etag: None,
            last_modified: None,
            content_type,
        };
        write_cache(plan.body_path, plan.meta_path, &body, &meta)?;
        self.count_capture(plan, status, bytes).await;
        Ok(FetchOutcome {
            url: meta.url,
            response_url: meta.response_url,
            method: meta.method,
            status,
            content_digest: meta.content_digest,
            bytes,
            fetched_at: meta.fetched_at,
            from_cache: false,
            content_type: meta.content_type,
            body,
        })
    }

    pub(super) async fn handle_404_capture(
        &self,
        plan: &FetchPlan<'_>,
        capture: BrowserCapture,
    ) -> Result<FetchOutcome, FetchError> {
        let status = 404u16;
        let body = decode_capture_body(plan, &capture)?;
        let content_hex = content_digest(&body);
        let bytes = body.len();
        let content_type = content_type(&capture.response.headers);
        let fetched_at = match capture.fetched_at_ms.and_then(instant_iso8601) {
            Some(value) => value,
            None => now_iso8601(),
        };
        let meta = CacheMeta {
        redirects: Vec::new(),
            url: plan.url.to_string(),
            response_url: capture.response.response_url.map(|url| url.into_string()),
            method: plan.method.to_string(),
            representation: plan.representation.clone(),
            status,
            content_digest: content_hex,
            bytes,
            fetched_at,
            etag: None,
            last_modified: None,
            content_type,
        };
        crate::net::cache::write_archive(plan.body_path, plan.meta_path, &body, &meta)?;
        self.count_capture(plan, status, bytes).await;
        if plan.options.allow_not_found {
            Ok(FetchOutcome {
                url: meta.url,
                response_url: meta.response_url,
                method: meta.method,
                status,
                content_digest: meta.content_digest,
                bytes,
                fetched_at: meta.fetched_at,
                from_cache: false,
                content_type: meta.content_type,
                body,
            })
        } else {
            Err(FetchError::Http {
                status,
                url: plan.url.to_string(),
            })
        }
    }

    async fn count_capture(&self, plan: &FetchPlan<'_>, status: u16, bytes: usize) {
        let downloaded = u64::try_from(bytes).map_or(u64::MAX, |value| value);
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
