use crate::net::request::build_request;
use crate::net::{FetchError, Fetcher};
use census_domain::model::AccessBlockKind;
use chrono::{DateTime, Utc};
use std::time::Duration;
use tracing::warn;

pub(super) const MAX_RETRY_AFTER_SECONDS: u64 = 86_400;

const MILLIS_PER_SECOND: u64 = 1_000;

const SKEW_TOLERANCE_SECONDS: i64 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RetryAfter {
    Absent,
    Delay(u64),
    Deadline(DateTime<Utc>),
    Refused(RetryAfterRefusal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RetryAfterRefusal {
    Unparsable,
    BeyondBound,
}

impl RetryAfter {
    pub(super) fn seconds_at(self, now: DateTime<Utc>) -> Option<u64> {
        match self {
            Self::Absent | Self::Refused(_) => None,
            Self::Delay(seconds) => Some(seconds),
            Self::Deadline(deadline) => Some(remaining_seconds(now, deadline)),
        }
    }
}

impl RetryAfterRefusal {
    fn label(self) -> &'static str {
        match self {
            Self::Unparsable => "unparsable",
            Self::BeyondBound => "beyond-bound",
        }
    }
}

pub(super) fn blocking_kind(status: u16) -> Option<AccessBlockKind> {
    match status {
        403 => Some(AccessBlockKind::Forbidden),
        429 => Some(AccessBlockKind::RateLimited),
        _ => None,
    }
}

fn remaining_seconds(now: DateTime<Utc>, deadline: DateTime<Utc>) -> u64 {
    deadline
        .signed_duration_since(now)
        .num_seconds()
        .max(SKEW_TOLERANCE_SECONDS)
        .try_into()
        .map_or(0, |seconds| seconds)
}

fn within_bound(now: DateTime<Utc>, deadline: DateTime<Utc>) -> bool {
    deadline.signed_duration_since(now).num_seconds() <= max_retry_after_i64()
}

fn max_retry_after_i64() -> i64 {
    i64::try_from(MAX_RETRY_AFTER_SECONDS).map_or(i64::MAX, |seconds| seconds)
}

pub(super) fn recorded_seconds(delay: RetryAfter, now: DateTime<Utc>) -> Option<u64> {
    if let RetryAfter::Refused(reason) = delay {
        warn!(refusal = reason.label(), "refused the Retry-After value");
    }
    delay.seconds_at(now)
}

pub(super) fn retry_after_secs(response: &reqwest::Response) -> Option<u64> {
    let now = Utc::now();
    let value = response.headers().get(reqwest::header::RETRY_AFTER);
    recorded_seconds(retry_after_header(value, now), now)
}

pub(super) fn recorded_millis(millis: u64) -> Option<u64> {
    let now = Utc::now();
    recorded_seconds(retry_after_millis(millis, now), now)
}

pub(super) fn retry_after_header(
    value: Option<&reqwest::header::HeaderValue>,
    now: DateTime<Utc>,
) -> RetryAfter {
    let Some(value) = value else {
        return RetryAfter::Absent;
    };
    match value.to_str() {
        Ok(text) => retry_after_delay(text.trim(), now),
        Err(_) => RetryAfter::Refused(RetryAfterRefusal::Unparsable),
    }
}

pub(super) fn retry_after_delay(value: &str, now: DateTime<Utc>) -> RetryAfter {
    if is_delta_seconds(value) {
        return match value.parse::<u64>() {
            Ok(seconds) if seconds <= MAX_RETRY_AFTER_SECONDS => RetryAfter::Delay(seconds),
            _ => RetryAfter::Refused(RetryAfterRefusal::BeyondBound),
        };
    }
    match parse_http_date(value) {
        Some(deadline) if within_bound(now, deadline) => RetryAfter::Deadline(deadline),
        Some(_) => RetryAfter::Refused(RetryAfterRefusal::BeyondBound),
        None => RetryAfter::Refused(RetryAfterRefusal::Unparsable),
    }
}

pub(super) fn retry_after_millis(millis: u64, now: DateTime<Utc>) -> RetryAfter {
    let seconds = millis.saturating_div(MILLIS_PER_SECOND);
    if seconds > MAX_RETRY_AFTER_SECONDS {
        return RetryAfter::Refused(RetryAfterRefusal::BeyondBound);
    }
    match deadline_of(now, seconds) {
        Some(deadline) => RetryAfter::Deadline(deadline),
        None => RetryAfter::Refused(RetryAfterRefusal::BeyondBound),
    }
}

fn is_delta_seconds(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn deadline_of(now: DateTime<Utc>, seconds: u64) -> Option<DateTime<Utc>> {
    let seconds = i64::try_from(seconds).ok()?;
    chrono::TimeDelta::try_seconds(seconds).and_then(|delta| now.checked_add_signed(delta))
}

fn parse_http_date(value: &str) -> Option<DateTime<Utc>> {
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
        host: &str,
        plan: &FetchPlan<'_>,
    ) -> Result<reqwest::Response, FetchError> {
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
        self.count_attempt(host).await;
        let response = tokio::time::timeout(
            Duration::from_secs(plan.timeout_secs),
            self.client.execute(request),
        )
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
