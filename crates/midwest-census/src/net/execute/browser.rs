//! The browser lane's seat in the fetch loop: the hosts it takes, and what its answer becomes here.
//!
//! The lane is a transport, not a side channel: a browser-transported host reaches this seat after
//! the same cache lookup, the same robots gate and the same per-host pacing an HTTP request does, and
//! the evidence written here has the shape [`crate::net::decode::process_response`] writes for a
//! response body. What is deliberately *not* shared is the transport's own status decision: the
//! pipeline's browser manager already made one inside the crate, so a capture that is not the
//! source's answer is refused by name here, never minted as evidence.
//!
//! Nothing in this module re-derives a verdict. The transport classifies; this seat records what the
//! classification means for the census — a retryable failure stays retryable, a human requirement
//! becomes a row — and hands the answer on.

use super::attempt::{blocking_kind, FetchPlan};
use crate::net::bridge::{Action, BrowserCapture, BrowserError, BrowserOutcome, RequestSpec, Verdict};
use crate::net::cache::{sha256_prefix16, write_cache, CacheMeta};
use crate::net::{
    instant_iso8601, now_iso8601, FetchError, FetchOutcome, Fetcher, MAX_BODY_BYTES,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use census_domain::model::AccessBlockKind;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::warn;

impl Fetcher {
    /// Fetch one browser-transported target through the lane.
    ///
    /// The seat sits where [`Fetcher::fetch_once`] sits, and takes the same turn: the cache, the
    /// robots gate and the host's turn have all been settled by the caller before either runs.
    pub(super) async fn fetch_browser(
        &self,
        gate: Arc<Mutex<()>>,
        plan: &FetchPlan<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        if !plan.method.eq_ignore_ascii_case("GET") {
            // The lane carries GETs and nothing else: its mirrored action has no body to post. A
            // non-GET reaching this seat is a routing fault, not a source observation, so it fails
            // loudly rather than being refused as if the host had said something.
            return Err(FetchError::Invariant {
                detail: format!(
                    "browser lane carries GET only, and {} was routed to it",
                    plan.method
                ),
            });
        }
        let _permit = gate.lock().await;
        self.wait_turn(plan.host).await;
        let spec = RequestSpec {
            url: plan.url.to_string(),
            semantic_url: plan.url.to_string(),
            action: Action::Fetch { body: None },
        };
        let Some(lane) = self.lane.as_ref() else {
            let detail = format!(
                "no browser lane is installed in this process, so {} cannot be fetched here",
                plan.host
            );
            self.record_access_condition(
                plan.host,
                AccessBlockKind::BrowserUnavailable,
                0,
                None,
                detail.clone(),
            )
            .await;
            return Err(FetchError::BrowserLane {
                url: plan.url.to_string(),
                detail,
                retryable: false,
            });
        };
        match lane.answer(&spec).await {
            Err(error) => {
                // The call never reached a verdict, so nothing was classified: what is known is that
                // the deployment's lane did not answer, which is the applicability row.
                if let FetchError::BrowserLane { detail, .. } = &error {
                    self.record_access_condition(
                        plan.host,
                        AccessBlockKind::BrowserUnavailable,
                        0,
                        None,
                        detail.clone(),
                    )
                    .await;
                }
                Err(error)
            }
            Ok(BrowserOutcome::Captured(capture)) if capture.challenge => {
                self.refuse_challenge(plan, &capture).await
            }
            Ok(BrowserOutcome::Captured(capture)) => {
                let status = capture.response.status;
                self.count_request(plan.host, status).await;
                if let Some(kind) = blocking_kind(status) {
                    let retry_after = retry_after_secs(&capture.response.headers);
                    self.record_access_condition(
                        plan.host,
                        kind,
                        status,
                        retry_after,
                        plan.url.to_string(),
                    )
                    .await;
                }
                match status {
                    200 | 404 => self.mint_capture(plan, capture).await,
                    304 => Err(FetchError::Invariant {
                        // A capture that reports 304 came from a transport that sent conditional
                        // headers; a capture that was taken from the source cannot be a revalidation.
                        detail: format!(
                            "browser lane answered 304 for {}: a capture is a take, not a revalidation",
                            plan.url
                        ),
                    }),
                    _ => Err(self.status_error(status, plan).await),
                }
            }
            Ok(BrowserOutcome::Failed(failure)) => {
                self.refuse_failure(plan, failure.error, failure.verdict).await
            }
        }
    }

    /// Record a challenge capture as the refusal it is.
    ///
    /// A challenged capture is the verification page, not the source's answer, so it is never minted
    /// as evidence: the row is `human_required`, which carries no cooldown — a profile that wants a
    /// person is cleared by a person and by nothing else, and a timer would only invite the run to
    /// try the same wall again. The capture's own status travels in the detail; the row's `status`
    /// column stays `0`, the audit schema's reading of "not an HTTP status".
    async fn refuse_challenge(
        &self,
        plan: &FetchPlan<'_>,
        capture: &BrowserCapture,
    ) -> Result<FetchOutcome, FetchError> {
        let waited = capture.retry_after_ms.map(|ms| ms / 1_000);
        let detail = match waited {
            Some(seconds) => format!(
                "the source answered {} with human verification, asking for {seconds}s",
                capture.response.status
            ),
            None => format!(
                "the source answered {} with human verification",
                capture.response.status
            ),
        };
        self.record_access_condition(
            plan.host,
            AccessBlockKind::HumanRequired,
            0,
            waited,
            detail.clone(),
        )
        .await;
        Err(FetchError::BrowserLane {
            url: plan.url.to_string(),
            detail,
            retryable: false,
        })
    }

    /// Grade one failed lane call, and hand it on.
    ///
    /// The verdict is the transport's, read rather than re-derived: `Retryable` leaves as a retryable
    /// error for the durable layer to replay, `HumanRequired` becomes the same row a challenge does,
    /// and a terminal failure that names the lane itself is the deployment row. The remaining
    /// terminal failures are the request's own or the transport's, and neither is an observation
    /// about the host, so no row is recorded for them.
    async fn refuse_failure(
        &self,
        plan: &FetchPlan<'_>,
        error: BrowserError,
        verdict: Verdict,
    ) -> Result<FetchOutcome, FetchError> {
        match (verdict, error) {
            (Verdict::HumanRequired, error) => {
                let detail = format!("the source answered with human verification ({error})");
                self.record_access_condition(
                    plan.host,
                    AccessBlockKind::HumanRequired,
                    0,
                    None,
                    detail.clone(),
                )
                .await;
                Err(FetchError::BrowserLane {
                    url: plan.url.to_string(),
                    detail,
                    retryable: false,
                })
            }
            (Verdict::Retryable, error) => Err(FetchError::BrowserLane {
                url: plan.url.to_string(),
                detail: format!("the browser lane reported {error}, which it grades retryable"),
                retryable: true,
            }),
            (Verdict::Terminal, BrowserError::Unavailable) => {
                let detail = format!("the browser lane reported {error}");
                self.record_access_condition(
                    plan.host,
                    AccessBlockKind::BrowserUnavailable,
                    0,
                    None,
                    detail.clone(),
                )
                .await;
                Err(FetchError::BrowserLane {
                    url: plan.url.to_string(),
                    detail,
                    retryable: false,
                })
            }
            (Verdict::Terminal, error) => Err(FetchError::BrowserLane {
                url: plan.url.to_string(),
                detail: format!("the browser lane reported {error}, which it grades terminal"),
                retryable: false,
            }),
        }
    }

    /// Mint the evidence one capture carries, in the shape a response body gets.
    ///
    /// The cache write comes first, exactly as it does for HTTP: a body the source served is worth
    /// keeping even when the status that carried it is one the run must stop on.
    async fn mint_capture(
        &self,
        plan: &FetchPlan<'_>,
        capture: BrowserCapture,
    ) -> Result<FetchOutcome, FetchError> {
        let status = capture.response.status;
        // Base64 is four characters per three bytes, plus padding: refusing on the encoded length
        // keeps a body over the ceiling from being allocated only to be thrown away.
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
                detail: format!(
                    "browser lane body for {} is not base64 ({error})",
                    plan.url
                ),
            })?;
        if body.len() > MAX_BODY_BYTES {
            return Err(FetchError::TooLarge {
                url: plan.url.to_string(),
            });
        }
        let mut hasher = Sha256::new();
        hasher.update(&body);
        let sha256 = sha256_prefix16(hasher);
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
            sha256: sha256.clone(),
            bytes,
            fetched_at: fetched_at.clone(),
            // The lane sends no conditional headers, so it has neither validator to carry forward.
            etag: None,
            last_modified: None,
            content_type: content_type.clone(),
        };
        write_cache(plan.body_path, plan.meta_path, &body, &meta)?;
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
        Ok(FetchOutcome {
            url: plan.url.to_string(),
            method: plan.method.to_string(),
            status,
            sha256,
            bytes,
            fetched_at,
            from_cache: false,
            content_type,
            body,
        })
    }
}

/// The `Retry-After` a capture published, when it published one in the delta-seconds form.
///
/// The HTTP path reads the same header from a `reqwest::Response`; a capture carries the pairs
/// themselves. The HTTP-date form is deliberately not parsed in either place: no source in this
/// corpus has used it, and a wrong guess at an instant is worse than no instant.
fn retry_after_secs(headers: &[(String, String)]) -> Option<u64> {
    headers
        .iter()
        .rev()
        .find(|(name, _)| name.eq_ignore_ascii_case("retry-after"))
        .and_then(|(_, value)| value.trim().parse::<u64>().ok())
}

/// The content type a capture published, when it published one.
fn content_type(headers: &[(String, String)]) -> Option<String> {
    headers
        .iter()
        .rev()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.clone())
}

#[cfg(test)]
mod tests;
