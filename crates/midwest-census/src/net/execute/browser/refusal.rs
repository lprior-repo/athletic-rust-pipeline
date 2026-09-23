//! The seat's refusals: the two answers a lane call can carry that are not the source's answer.
//!
//! A challenge capture and a failed call are one refusal read from two sides — the verification page
//! arrived, or the call never produced one — and neither is ever minted as evidence: the row is
//! `human_required`, which carries no cooldown, because a profile that wants a person is cleared by a
//! person and by nothing else, and a timer would only invite the run to try the same wall again. The
//! capture's own status travels in the detail; the row's `status` column stays `0`, the audit
//! schema's reading of "not an HTTP status".
//!
//! Nothing here re-derives a verdict either. The verdict is the transport's, read rather than
//! classified again: `Retryable` leaves as a retryable error for the durable layer to replay,
//! `HumanRequired` becomes the same row a challenge does, and a terminal failure that names the lane
//! itself is the deployment row. The remaining terminal failures are the request's own or the
//! transport's, and neither is an observation about the host, so no row is recorded for them.

use super::FetchPlan;
use crate::net::bridge::{BrowserCapture, BrowserError, Verdict};
use crate::net::{FetchError, FetchOutcome, Fetcher};
use census_domain::model::AccessBlockKind;

impl Fetcher {
    /// Record a challenge capture as the refusal it is.
    ///
    /// A challenged capture is the verification page, not the source's answer, so it is never minted
    /// as evidence: the row is `human_required`, which carries no cooldown — a profile that wants a
    /// person is cleared by a person and by nothing else, and a timer would only invite the run to
    /// try the same wall again. The capture's own status travels in the detail; the row's `status`
    /// column stays `0`, the audit schema's reading of "not an HTTP status".
    pub(super) async fn refuse_challenge(
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
    pub(super) async fn refuse_failure(
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
}
