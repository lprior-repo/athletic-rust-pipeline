
use super::FetchPlan;
use crate::net::bridge::{BrowserCapture, BrowserError, Verdict};
use crate::net::{FetchError, FetchOutcome, Fetcher};
use census_domain::model::AccessBlockKind;

impl Fetcher {
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
