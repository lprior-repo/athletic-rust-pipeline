//! The browser lane's seat in the fetch loop: the hosts it takes, and what its answer becomes here.
//!
//! The lane is a transport, not a side channel: a browser-transported host reaches this seat after
//! the same cache lookup, the same robots gate and the same per-host pacing an HTTP request does, and
//! the evidence written here has the shape the HTTP lane's `cache_and_record` writes for a
//! response body. What is deliberately *not* shared is the transport's own status decision: the
//! pipeline's browser manager already made one inside the crate, so a capture that is not the
//! source's answer is refused by name here, never minted as evidence.
//!
//! Nothing in this module re-derives a verdict. The transport classifies; this seat records what the
//! classification means for the census — a retryable failure stays retryable, a human requirement
//! becomes a row — and hands the answer on. What stays here is the entry, the accept path and the
//! evidence; the two verdicts a lane call can carry that are not an answer live next door, in
//! [`refusal`].
//!
//! Evidence building is extracted to [`evidence`]: decoding, hashing, caching, and outcome construction.

use super::attempt::{blocking_kind, FetchPlan};
use crate::net::bridge::{Action, BrowserCapture, BrowserOutcome, RequestSpec};
use crate::net::{FetchError, FetchOutcome, Fetcher};
use census_domain::model::AccessBlockKind;
use std::sync::Arc;
use tokio::sync::Mutex;

mod evidence;
mod refusal;

// This seat's tests drive [`refusal`]'s methods and read the transport's verdict vocabulary back
// through here; the seat's own code never names a verdict, because naming one is the first step
// towards re-deriving it.
#[cfg(test)]
use crate::net::bridge::{BrowserError, Verdict};

// Re-exports needed by browser/tests.rs via `super::*`.
#[cfg(test)]
use base64::engine::general_purpose::STANDARD as BASE64;
#[cfg(test)]
use sha2::Sha256;

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
            return self.refuse_without_lane(plan).await;
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
            Ok(BrowserOutcome::Captured(capture)) => self.accept_capture(plan, capture).await,
            Ok(BrowserOutcome::Failed(failure)) => {
                self.refuse_failure(plan, failure.error, failure.verdict)
                    .await
            }
        }
    }

    /// Refuse a target the registry routes to a lane this process does not have installed.
    ///
    /// `None` is not "fetch it over HTTP instead": attempting the other transport quietly is the
    /// thing §69 exists to prevent, so the refusal names what is missing and records the
    /// applicability row — a lane that is not there yet may be there later, which is why this row
    /// expires where the human-requirement row does not.
    async fn refuse_without_lane(&self, plan: &FetchPlan<'_>) -> Result<FetchOutcome, FetchError> {
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
        Err(FetchError::BrowserLane {
            url: plan.url.to_string(),
            detail,
            retryable: false,
        })
    }

    /// Accept one capture that is the source's answer, and read the status it carries.
    ///
    /// The statuses are read as the HTTP seat reads them: `200` and an allowed `404` are evidence, a
    /// `304` cannot come from a transport that sends no conditional headers, and every other status
    /// is graded once, by [`Fetcher::status_error`].
    async fn accept_capture(
        &self,
        plan: &FetchPlan<'_>,
        capture: BrowserCapture,
    ) -> Result<FetchOutcome, FetchError> {
        let status = capture.response.status;
        self.count_request(plan.host, status).await;
        // A 403 or 429 is an observation about the *host*, not about this URL, so it is recorded
        // once per run here exactly as it is on the HTTP path.
        if let Some(kind) = blocking_kind(status) {
            let retry_after = capture.retry_after_ms.map(|ms| ms / 1_000);
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
            200 => self.mint_capture(plan, capture).await,
            404 => self.handle_404_capture(plan, capture).await,
            304 => Err(FetchError::Invariant {
                // A capture that reports 304 came from a transport that sent conditional headers; a
                // capture that was taken from the source cannot be a revalidation.
                detail: format!(
                    "browser lane answered 304 for {}: a capture is a take, not a revalidation",
                    plan.url
                ),
            }),
            _ => Err(self.status_error(status, plan).await),
        }
    }
}

// Re-export for browser/tests.rs.
#[cfg(test)]
pub(super) use crate::net::MAX_BODY_BYTES;

#[cfg(test)]
mod tests;
