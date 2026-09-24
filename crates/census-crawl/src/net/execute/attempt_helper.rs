//! Shared utilities and Fetcher impl methods extracted from `attempt.rs`.
//!
//! This module contains pure status classification helpers and the HTTP dispatch / error-building
//! impl methods that are logically separate from the core attempt orchestration in `attempt.rs`.

use crate::net::request::build_request;
use crate::net::{FetchError, Fetcher};
use census_domain::model::AccessBlockKind;
use tracing::warn;
use std::time::Duration;

/// The access condition one HTTP status states, when it states one.
///
/// `403` is the source refusing a path its own robots rules allow, and `429` is the source asking
/// for less traffic. Both are observations about the host, which is why they are recorded against
/// it rather than against the URL that happened to be in flight.
///
/// A capture the browser lane carries states the same two the same way: the status travels on the
/// capture, so the reading is shared rather than restated.
pub(super) fn blocking_kind(status: u16) -> Option<AccessBlockKind> {
    match status {
        403 => Some(AccessBlockKind::Forbidden),
        429 => Some(AccessBlockKind::RateLimited),
        _ => None,
    }
}

/// The `Retry-After` one response published, when it published one in the delta-seconds form.
///
/// The HTTP-date form is deliberately not parsed: no source in this corpus has used it, and a wrong
/// guess at an instant is worse than no instant, so an unparsed value stays `None` and the policy
/// default applies.
pub(super) fn retry_after_secs(response: &reqwest::Response) -> Option<u64> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
}

// ---------------------------------------------------------------------------
// Fetcher impl methods
// ---------------------------------------------------------------------------

use crate::net::execute::attempt::FetchPlan;

impl Fetcher {
    /// Build the HTTP request and send it under the per-request timeout.
pub(super) async fn dispatch(&self, plan: &FetchPlan<'_>) -> Result<reqwest::Response, FetchError> {
        let request = build_request(
            &self.client,
            plan.method,
            plan.url,
            plan.payload,
            &plan.options.headers,
            plan.cached,
            plan.options.refresh,
        )?;
        let response = tokio::time::timeout(Duration::from_secs(plan.timeout_secs), request.send())
            .await
            .map_err(|_| FetchError::Timeout {
                url: plan.url.to_string(),
                timeout_secs: plan.timeout_secs,
            })?
            .map_err(|source| FetchError::Transport {
                url: plan.url.to_string(),
                source,
            })?;
        Ok(response)
    }

    /// Non-200/404/304: record the error and return it for the durable layer to classify.
    ///
    /// The browser lane's seat reaches this for exactly the same statuses, so a refusal that
    /// arrives inside a capture (403, 5xx) is graded once, by this policy, whichever transport
    /// carried it.
    pub(super) async fn status_error(&self, status: u16, plan: &FetchPlan<'_>) -> FetchError {
        {
            let mut stats = self.stats.lock().await;
            stats.errors = stats.errors.saturating_add(1);
        }
        warn!(status, url = plan.url, "non-success response");
        // Every status here is either a host observation (403/429, already recorded against the host)
        // or a fault the durable retry policy exists to absorb. The transport's job is to report it;
        // grading it as retryable or terminal belonged to the loop that no longer runs here.
        FetchError::Http {
            status,
            url: plan.url.to_string(),
        }
    }
}
