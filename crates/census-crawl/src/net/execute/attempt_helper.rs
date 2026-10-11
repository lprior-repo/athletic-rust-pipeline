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
    let value = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?;
    retry_after_delay(value.trim(), chrono::Utc::now())
}

pub(super) fn retry_after_delay(value: &str, now: chrono::DateTime<chrono::Utc>) -> Option<u64> {
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(seconds);
    }
    let deadline = parse_http_date(value)?;
    let remaining = deadline.signed_duration_since(now).num_seconds().max(0);
    u64::try_from(remaining).ok()
}

fn parse_http_date(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    [
        "%a, %d %b %Y %H:%M:%S GMT",
        "%A, %d-%b-%y %H:%M:%S GMT",
        "%a %b %e %H:%M:%S %Y",
    ]
    .iter()
    .find_map(|format| {
        chrono::NaiveDateTime::parse_from_str(value, format)
            .ok()
            .map(|parsed| parsed.and_utc())
    })
}

use crate::net::execute::attempt::FetchPlan;

impl Fetcher {
    pub(super) async fn dispatch_at(
        &self,
        url: &str,
        plan: &FetchPlan<'_>,
    ) -> Result<reqwest::Response, FetchError> {
        let mut request = build_request(
            &self.client,
            plan.method,
            url,
            plan.payload,
            plan.representation.entries(),
            plan.cached,
            plan.options.refresh,
        )?;
        for (name, value) in &plan.options.headers {
            if super::representation::is_validator(name) {
                request = request.header(name.as_str(), value.as_str());
            }
        }
        let response = tokio::time::timeout(Duration::from_secs(plan.timeout_secs), request.send())
            .await
            .map_err(|_| FetchError::Timeout {
                url: url.to_string(),
                timeout_secs: plan.timeout_secs,
            })?
            .map_err(|source| FetchError::Transport {
                url: url.to_string(),
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
