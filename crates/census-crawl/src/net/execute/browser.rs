use super::attempt::{blocking_kind, FetchPlan};
use crate::net::bridge::{Action, BrowserCapture, BrowserOutcome, RequestSpec};
use crate::net::{FetchError, FetchOutcome, Fetcher};
use census_domain::model::AccessBlockKind;
use std::sync::Arc;
use tokio::sync::Mutex;

mod evidence;
mod refusal;

#[cfg(test)]
use base64::engine::general_purpose::STANDARD as BASE64;

impl Fetcher {
    pub(super) async fn fetch_browser(
        &self,
        gate: Arc<Mutex<()>>,
        plan: &FetchPlan<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        if !plan.method.eq_ignore_ascii_case("GET") {
            return Err(FetchError::Invariant {
                detail: format!(
                    "browser lane carries GET only, and {} was routed to it",
                    plan.method
                ),
            });
        }
        if plan
            .options
            .headers
            .iter()
            .any(|(name, _)| super::representation::is_validator(name))
        {
            return Err(FetchError::Policy {
                detail: "browser requests cannot carry conditional validators".to_string(),
            });
        }
        let _permit = gate.lock().await;
        self.wait_turn(plan.host).await;
        let spec = RequestSpec {
            url: plan.url.to_string(),
            semantic_url: plan.url.to_string(),
            action: Action::Fetch { body: None },
            headers: plan.representation.clone(),
        };
        let Some(lane) = self.lane.as_ref() else {
            return self.refuse_without_lane(plan).await;
        };
        match lane.answer(&spec).await {
            Err(error) => {
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

    async fn accept_capture(
        &self,
        plan: &FetchPlan<'_>,
        capture: BrowserCapture,
    ) -> Result<FetchOutcome, FetchError> {
        let status = capture.response.status;
        self.check_capture_response_url(plan, &capture)?;
        self.count_request(plan.host, status).await;
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
                detail: format!(
                    "browser lane answered 304 for {}: a capture is a take, not a revalidation",
                    plan.url
                ),
            }),
            _ => Err(self.status_error(status, plan).await),
        }
    }

    fn check_capture_response_url(
        &self,
        plan: &FetchPlan<'_>,
        capture: &BrowserCapture,
    ) -> Result<(), FetchError> {
        let Some(response_url) = capture.response.response_url.as_deref() else {
            return Ok(());
        };
        if response_url == plan.url {
            return Ok(());
        }
        let final_url = url::Url::parse(response_url).map_err(|source| FetchError::Policy {
            detail: format!("cannot parse browser capture response URL for admission: {source}"),
        })?;
        let original = url::Url::parse(plan.url).map_err(|source| FetchError::Policy {
            detail: format!("cannot parse original browser URL for admission: {source}"),
        })?;
        if !self.destination.permits_redirect(&original, &final_url) {
            return Err(FetchError::Policy {
                detail: format!(
                    "browser capture final URL {} is not admitted for {}",
                    response_url, plan.url
                ),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
pub(super) use crate::net::MAX_BODY_BYTES;

#[cfg(test)]
mod tests;
