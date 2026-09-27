
use crate::net::request::build_request;
use crate::net::{FetchError, Fetcher};
use census_domain::model::AccessBlockKind;
use std::time::Duration;
use tracing::warn;

pub(super) fn blocking_kind(status: u16) -> Option<AccessBlockKind> {
    match status {
        403 => Some(AccessBlockKind::Forbidden),
        429 => Some(AccessBlockKind::RateLimited),
        _ => None,
    }
}

pub(super) fn retry_after_secs(response: &reqwest::Response) -> Option<u64> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
}

use crate::net::execute::attempt::FetchPlan;

impl Fetcher {
    pub(super) async fn dispatch(
        &self,
        plan: &FetchPlan<'_>,
    ) -> Result<reqwest::Response, FetchError> {
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
