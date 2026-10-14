use super::attempt_helper::{blocking_kind, retry_after_secs};
use crate::net::execute::attempt::FetchPlan;
use crate::net::request::build_request;
use crate::net::{FetchError, Fetcher};
use std::time::Duration;
use tracing::warn;

impl Fetcher {
    pub(super) async fn dispatch_at(
        &self,
        url: &str,
        host: &str,
        plan: &FetchPlan<'_>,
        session: &mut crate::net::acquisition::CaptureSession<'_>,
    ) -> Result<reqwest::Response, FetchError> {
        let request = self.planned_request(url, plan)?;
        if session.is_adopted() {
            self.admit_adopted_dispatch(url, host).await?;
        }
        #[cfg(test)]
        session.prearm_timeout(url)?;
        let armed = session.arm(&request)?;
        #[cfg(test)]
        session.interrupt(crate::net::acquisition::CaptureInterruption::AfterArm)?;
        self.count_attempt(host).await;
        let lease = armed.into_observation();
        let outcome = tokio::time::timeout(
            Duration::from_secs(plan.timeout_secs),
            self.client.execute(request),
        )
        .await;
        let response = match outcome {
            Ok(Ok(response)) => response,
            Ok(Err(source)) => {
                let error = FetchError::Transport {
                    url: url.to_string(),
                    source,
                };
                lease.fail(&error)?;
                return Err(error);
            }
            Err(_) => {
                let error = FetchError::Timeout {
                    url: url.to_string(),
                    timeout_secs: plan.timeout_secs,
                };
                lease.fail(&error)?;
                return Err(error);
            }
        };
        if let Err(error) = session.observed(lease, &response, &self.capture_clock.observed_at()) {
            if let Some(kind) = blocking_kind(response.status().as_u16()) {
                self.record_access_condition(
                    host,
                    kind,
                    response.status().as_u16(),
                    retry_after_secs(&response),
                    response.url().as_str().to_owned(),
                )
                .await;
            }
            return Err(error);
        }
        #[cfg(test)]
        session.interrupt(crate::net::acquisition::CaptureInterruption::AfterObserved)?;
        Ok(response)
    }
    async fn admit_adopted_dispatch(&self, url: &str, host: &str) -> Result<(), FetchError> {
        self.destination.validate_resolved_url(url).await?;
        if let Some(refusal) = self.refused_admission(host, url).await {
            return Err(refusal);
        }
        if self.host_blocked(host, &crate::net::now_iso8601()).await {
            return Err(FetchError::Cooldown {
                host: host.to_owned(),
            });
        }
        Ok(())
    }

    fn planned_request(
        &self,
        url: &str,
        plan: &FetchPlan<'_>,
    ) -> Result<reqwest::Request, FetchError> {
        super::representation::canonical_request(url, &plan.options.headers)?;
        let request = build_request(
            &self.client,
            plan.method,
            url,
            plan.payload,
            &plan.options.headers,
            plan.cached,
            plan.options.refresh,
        )?
        .build()
        .map_err(|source| FetchError::Transport {
            url: url.to_string(),
            source,
        })?;
        Ok(request)
    }
    pub(super) async fn status_error(&self, status: u16, plan: &FetchPlan<'_>) -> FetchError {
        {
            let mut stats = self.stats.lock().await;
            stats.errors = stats.errors.saturating_add(1);
        }
        warn!(status, url = plan.url, "non-success response");
        FetchError::Http {
            status,
            url: plan.url.to_string(),
        }
    }
}
