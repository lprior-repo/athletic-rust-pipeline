use super::attempt::{blocking_kind, FetchPlan};
use super::attempt_helper::recorded_millis;
use crate::net::bridge::{Action, BrowserCapture, BrowserOutcome, RequestSpec};
use crate::net::representation::USER_AGENT_NAME;
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
        self.validate_browser_request(plan)?;
        let _permit = gate.lock().await;
        self.wait_turn(plan.host).await;
        let spec = RequestSpec {
            url: plan.url.to_string(),
            semantic_url: plan.url.to_string(),
            action: Action::Fetch {},
            headers: plan.representation.clone(),
        };
        let Some(lane) = self.lane.as_ref() else {
            return self.refuse_without_lane(plan).await;
        };
        let result = self.execute_browser_request(lane, &spec, plan).await?;
        self.handle_browser_outcome(plan, result).await
    }

    fn validate_browser_request(&self, plan: &FetchPlan<'_>) -> Result<(), FetchError> {
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
        if plan
            .options
            .headers
            .iter()
            .any(|(name, _)| name.trim().eq_ignore_ascii_case(USER_AGENT_NAME))
        {
            return Err(FetchError::Policy {
                detail: format!(
                    "browser requests cannot carry a declared User-Agent ({}): the browser engine sends its own identity, so a declared value would silently diverge",
                    plan.host
                ),
            });
        }
        Ok(())
    }

    async fn execute_browser_request(
        &self,
        lane: &crate::net::bridge::BrowserLane,
        spec: &RequestSpec,
        plan: &FetchPlan<'_>,
    ) -> Result<BrowserOutcome, FetchError> {
        self.count_attempt(plan.host).await;
        match lane.answer(spec).await {
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
            Ok(outcome) => Ok(outcome),
        }
    }

    async fn handle_browser_outcome(
        &self,
        plan: &FetchPlan<'_>,
        outcome: BrowserOutcome,
    ) -> Result<FetchOutcome, FetchError> {
        match outcome {
            BrowserOutcome::Captured(capture) if capture.challenge => {
                self.refuse_challenge(plan, &capture).await
            }
            BrowserOutcome::Captured(capture) => self.accept_capture(plan, capture).await,
            BrowserOutcome::Failed(failure) => {
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
        if let Some(kind) = blocking_kind(status) {
            let retry_after = capture.retry_after_ms.and_then(recorded_millis);
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
            _ => {
                self.archive_refusal_capture(plan, &capture, status).await;
                Err(self.status_error(status, plan).await)
            }
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

#[cfg(test)]
mod accounting_tests;

#[cfg(test)]
mod user_agent_policy_tests {
    use super::*;
    use crate::net::{FetchOptions, RepresentationHeaders};
    use std::collections::HashMap;
    use std::path::Path;
    use std::time::Duration;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    const URL: &str =
        "https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=1&sport=tf";
    const HOST: &str = "www.athletic.net";

    fn fetcher_in(dir: &Path) -> TestResult<Fetcher> {
        Ok(Fetcher::new(
            dir.join("http"),
            None,
            Duration::from_millis(1),
            HashMap::new(),
            Vec::new(),
        )?)
    }

    #[test]
    fn a_declared_user_agent_is_refused_before_the_lane_is_asked() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let dir = tempfile::tempdir()?;
                let fetcher = fetcher_in(dir.path())?;
                let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", URL, ""));
                let options = FetchOptions {
                    refresh: false,
                    allow_not_found: false,
                    headers: vec![(
                        USER_AGENT_NAME.to_string(),
                        "Mozilla/5.0 Chrome/126.0 census-service/0.1".to_string(),
                    )],
                };
                let representation = RepresentationHeaders::canonical(&options.headers)?;
                let plan = FetchPlan {
                    method: "GET",
                    url: URL,
                    payload: None,
                    host: HOST,
                    family: None,
                    body_path: &body_path,
                    meta_path: &meta_path,
                    cached: None,
                    options: &options,
                    representation: &representation,
                    timeout_secs: 45,
                };
                match fetcher.fetch_browser(Arc::new(Mutex::new(())), &plan).await {
                    Err(FetchError::Policy { detail }) => {
                        check!(
                            detail.contains("User-Agent"),
                            "the refusal names the header it cannot carry: {detail}"
                        );
                        check!(
                            detail.contains(HOST),
                            "the refusal names the host whose identity would diverge: {detail}"
                        );
                    }
                    other => {
                        return Err(format!(
                            "a process with no lane answers BrowserLane, so any other result means the policy refusal did not run first: {other:?}"
                        )
                        .into())
                    }
                }
                check!(
                    fetcher.access_conditions().await.is_empty(),
                    "a policy refusal is not an observation about the host"
                );
                check!(eq;
                    fetcher.stats.lock().await.bytes_downloaded,
                    0,
                    "no bytes move on a policy refusal"
                );
                Ok(())
            })
    }
}
