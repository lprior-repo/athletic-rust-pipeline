use crate::net::{FetchError, FetchOutcome};
use chrono::{DateTime, Utc};
use std::time::Duration;
use url::Url;

pub(super) const MAX_POLICY_AGE_SECONDS: i64 = 86_400;
const MAX_POLICY_BYTES: usize = 500 * 1024;
const MAX_POLICY_LINES: usize = 512;
const MAX_LINE_BYTES: usize = 2048;
const MAX_DELAY_SECONDS: f64 = 60.0;

pub(in crate::net) struct Policy {
    origin: url::Origin,
    rules: texting_robots::Robot,
    delay: Duration,
    locator: String,
    digest: String,
    fetched_at: DateTime<Utc>,
}

impl Policy {
    pub(in crate::net) fn from_capture(
        target: &Url,
        capture: &FetchOutcome,
        user_agent: &str,
    ) -> Result<Self, FetchError> {
        let product = user_agent
            .split_ascii_whitespace()
            .next()
            .and_then(|value| value.split('/').next())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| refusal("configured user-agent has no product token"))?;
        let (body, delay) = match capture.status {
            404 => ("", Duration::ZERO),
            200 => admitted_document(capture)?,
            _ => return Err(refusal("robots capture has an unsupported response status")),
        };
        let rules = texting_robots::Robot::new(product, body.as_bytes())
            .map_err(|error| refusal(&format!("robots parser refused policy: {error}")))?;
        Ok(Self {
            origin: target.origin(),
            rules,
            delay,
            locator: capture.url.clone(),
            digest: capture.content_digest.clone(),
            fetched_at: retrieval_time(capture)?,
        })
    }

    pub(in crate::net) fn check(&self, raw: &str) -> Result<(), FetchError> {
        let url = parse_url(raw)?;
        if url.origin() != self.origin {
            return Err(refusal(&format!(
                "school research incomplete: {raw} redirects beyond the origin covered by {}",
                self.locator
            )));
        }
        let relative = &url[url::Position::BeforePath..url::Position::AfterQuery];
        if relative.contains('%') {
            return Err(refusal(
                "percent-encoded school paths require unsupported robots normalization",
            ));
        }
        if !self.rules.allowed(relative) {
            return Err(refusal(&format!(
                "robots disallows {raw}; policy={} sha256={}",
                self.locator, self.digest
            )));
        }
        Ok(())
    }

    pub(in crate::net) fn check_dispatch(
        &self,
        url: &str,
        dispatched_at: &str,
    ) -> Result<(), FetchError> {
        self.check(url)?;
        let dispatched_at = DateTime::parse_from_rfc3339(dispatched_at)
            .map_err(|_| refusal("robots dispatch has an invalid timestamp"))?
            .with_timezone(&Utc);
        if !current_at(self.fetched_at, dispatched_at) {
            return Err(refusal("robots policy is no longer current at dispatch"));
        }
        Ok(())
    }

    pub(in crate::net) fn delay(&self) -> Duration {
        self.delay
    }
}

fn admitted_document(capture: &FetchOutcome) -> Result<(&str, Duration), FetchError> {
    if capture.body.len() > MAX_POLICY_BYTES {
        return Err(refusal("robots policy exceeds the 500 KiB parsing budget"));
    }
    let text =
        std::str::from_utf8(&capture.body).map_err(|_| refusal("robots policy is not UTF-8"))?;
    if text.contains('<')
        || text.contains('\0')
        || capture
            .content_type
            .as_deref()
            .is_some_and(|value| value.to_ascii_lowercase().contains("html"))
    {
        return Err(refusal(
            "robots endpoint returned HTML or an unsupported document",
        ));
    }
    if text.lines().count() > MAX_POLICY_LINES {
        return Err(refusal("robots policy exceeds the 512-line parsing budget"));
    }
    text.lines()
        .try_fold(None, inspect_line)
        .map(|delay| (text, delay.map_or(Duration::ZERO, |value| value)))
}

fn inspect_line(delay: Option<Duration>, raw: &str) -> Result<Option<Duration>, FetchError> {
    if raw.len() > MAX_LINE_BYTES {
        return Err(refusal("robots policy line exceeds 2048 bytes"));
    }
    let line = raw.split('#').next().map_or("", |value| value).trim();
    if line.is_empty() {
        return Ok(delay);
    }
    let (field, value) = line
        .split_once(':')
        .ok_or_else(|| refusal("robots policy contains an unsupported malformed line"))?;
    let value = value.trim();
    if field.trim().eq_ignore_ascii_case("user-agent") && value.is_empty() {
        return Err(refusal("robots user-agent is empty"));
    }
    if matches!(
        field.trim().to_ascii_lowercase().as_str(),
        "allow" | "disallow"
    ) && !value.is_empty()
        && !value.starts_with('/')
        && !value.starts_with('*')
    {
        return Err(refusal("robots rule has an unsupported non-path value"));
    }
    if matches!(
        field.trim().to_ascii_lowercase().as_str(),
        "allow" | "disallow"
    ) && (value.contains('%')
        || value.matches('$').count() > 1
        || (value.contains('$') && !value.ends_with('$')))
    {
        return Err(refusal(
            "robots rule requires unsupported path normalization or wildcard syntax",
        ));
    }
    if !field.trim().eq_ignore_ascii_case("crawl-delay") {
        return Ok(delay);
    }
    if delay.is_some() {
        return Err(refusal(
            "multiple crawl-delay directives require unsupported group resolution",
        ));
    }
    let seconds = value
        .parse::<f64>()
        .map_err(|_| refusal("robots crawl-delay is not numeric"))?;
    if !seconds.is_finite() || !(0.0..=MAX_DELAY_SECONDS).contains(&seconds) {
        return Err(refusal(
            "robots crawl-delay is negative, nonfinite or exceeds the 60-second budget",
        ));
    }
    Duration::try_from_secs_f64(seconds)
        .map(Some)
        .map_err(|_| refusal("robots crawl-delay is not representable"))
}

pub(in crate::net) fn capture_is_current(
    capture: &FetchOutcome,
    now: DateTime<Utc>,
) -> Result<bool, FetchError> {
    Ok(current_at(retrieval_time(capture)?, now))
}

fn retrieval_time(capture: &FetchOutcome) -> Result<DateTime<Utc>, FetchError> {
    DateTime::parse_from_rfc3339(&capture.fetched_at)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| refusal("robots capture has an invalid retrieval timestamp"))
}

fn current_at(observed: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    let age = now.signed_duration_since(observed);
    age >= chrono::Duration::zero() && age <= chrono::Duration::seconds(MAX_POLICY_AGE_SECONDS)
}

pub(in crate::net) fn parse_url(raw: &str) -> Result<Url, FetchError> {
    Url::parse(raw).map_err(|source| FetchError::InvalidUrl {
        url: raw.to_owned(),
        source,
    })
}

pub(in crate::net) fn refusal(detail: &str) -> FetchError {
    FetchError::Policy {
        detail: detail.to_owned(),
    }
}
