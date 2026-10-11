#![forbid(unsafe_code)]
use crate::net::{now_iso8601, FetchError, Fetcher};
use census_domain::model::{AccessBlockKind, SourceAccessCondition};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClosePriority {
    HumanRequired,
    BrowserUnavailable,
    RateLimited,
}

impl ClosePriority {
    fn of(kind: AccessBlockKind) -> Option<Self> {
        match kind {
            AccessBlockKind::HumanRequired => Some(Self::HumanRequired),
            AccessBlockKind::BrowserUnavailable => Some(Self::BrowserUnavailable),
            AccessBlockKind::RateLimited => Some(Self::RateLimited),
            AccessBlockKind::Forbidden
            | AccessBlockKind::RobotsDisallowed
            | AccessBlockKind::Timeout
            | AccessBlockKind::Unavailable => None,
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::HumanRequired => 0,
            Self::BrowserUnavailable => 1,
            Self::RateLimited => 2,
        }
    }
}

impl Fetcher {
    pub(super) async fn refused_admission(&self, host: &str, url: &str) -> Option<FetchError> {
        let closing = self.closing_condition(host).await?;
        Some(Self::refusal_for(url, &closing))
    }

    async fn closing_condition(&self, host: &str) -> Option<SourceAccessCondition> {
        let now = now_iso8601();
        let blocks = self.pacing.blocks.lock().await;
        let mut best: Option<(u8, SourceAccessCondition)> = None;
        for condition in blocks.values() {
            if condition.host != host {
                continue;
            }
            if !condition.is_blocking(&now) {
                continue;
            }
            let Some(priority) = ClosePriority::of(condition.kind) else {
                continue;
            };
            let rank = priority.rank();
            let replace = best.as_ref().is_none_or(|(held, _)| rank < *held);
            if replace {
                best = Some((rank, condition.clone()));
            }
        }
        best.map(|(_, condition)| condition)
    }

    fn refusal_for(url: &str, closing: &SourceAccessCondition) -> FetchError {
        match closing.kind {
            AccessBlockKind::RateLimited => FetchError::Cooldown {
                host: closing.host.clone(),
            },
            AccessBlockKind::HumanRequired | AccessBlockKind::BrowserUnavailable => {
                FetchError::BrowserLane {
                    url: url.to_string(),
                    detail: Self::closure_detail(closing),
                    retryable: false,
                }
            }
            AccessBlockKind::Forbidden
            | AccessBlockKind::RobotsDisallowed
            | AccessBlockKind::Timeout
            | AccessBlockKind::Unavailable => FetchError::Policy {
                detail: Self::closure_detail(closing),
            },
        }
    }

    fn human_detail(closing: &SourceAccessCondition) -> String {
        format!(
            "admission closed for {} after human verification; {}",
            closing.host, closing.detail
        )
    }
    fn lane_detail(closing: &SourceAccessCondition) -> String {
        format!(
            "admission closed for {} while the browser lane is unavailable; {}",
            closing.host, closing.detail
        )
    }
    fn cooldown_detail(closing: &SourceAccessCondition) -> String {
        format!(
            "admission closed for {} until {}",
            closing.host,
            closing
                .cooldown_until
                .as_deref()
                .map_or("cooldown", |value| value)
        )
    }
    fn fallback_detail(closing: &SourceAccessCondition) -> String {
        format!(
            "admission closed for {} ({}); {}",
            closing.host,
            closing.kind.slug(),
            closing.detail
        )
    }
    fn closure_detail(closing: &SourceAccessCondition) -> String {
        match closing.kind {
            AccessBlockKind::HumanRequired => Self::human_detail(closing),
            AccessBlockKind::BrowserUnavailable => Self::lane_detail(closing),
            AccessBlockKind::RateLimited => Self::cooldown_detail(closing),
            AccessBlockKind::Forbidden
            | AccessBlockKind::RobotsDisallowed
            | AccessBlockKind::Timeout
            | AccessBlockKind::Unavailable => Self::fallback_detail(closing),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    fn fetcher_in(dir: &std::path::Path) -> TestResult<Fetcher> {
        Fetcher::new(
            dir.join("http"),
            None,
            Duration::from_millis(1),
            HashMap::new(),
            Vec::new(),
        )
        .map_err(Into::into)
    }

    const HOST: &str = "www.example.test";
    const URL: &str = "https://www.example.test/teams/1/roster";

    #[tokio::test]
    async fn a_human_required_row_closes_admission_without_a_physical_request() -> TestResult {
        let dir = tempfile::tempdir()?;
        let fetcher = fetcher_in(dir.path())?;
        fetcher
            .record_access_condition(
                HOST,
                AccessBlockKind::HumanRequired,
                0,
                None,
                "challenge page",
            )
            .await;
        let before = fetcher.stats().await;
        let refusal = fetcher
            .refused_admission(HOST, URL)
            .await
            .ok_or("human verification closes admission")?;
        check!(!refusal.retryable());
        check!(matches!(refusal, FetchError::BrowserLane { .. }));
        let after = fetcher.stats().await;
        check!(eq; before.requests, after.requests);
        check!(eq; before.cache_hits, after.cache_hits);
        check!(eq; fetcher.access_conditions().await.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn a_rate_limited_row_refuses_with_the_retryable_verdict() -> TestResult {
        let dir = tempfile::tempdir()?;
        let fetcher = fetcher_in(dir.path())?;
        fetcher
            .record_access_condition(HOST, AccessBlockKind::RateLimited, 429, Some(120), "slow")
            .await;
        let refusal = fetcher
            .refused_admission(HOST, URL)
            .await
            .ok_or("a cooldown closes admission")?;
        check!(refusal.retryable());
        check!(matches!(
            refusal,
            FetchError::Cooldown { ref host } if host == HOST
        ));
        Ok(())
    }

    #[tokio::test]
    async fn a_forbidden_page_never_closes_the_host() -> TestResult {
        let dir = tempfile::tempdir()?;
        let fetcher = fetcher_in(dir.path())?;
        fetcher
            .record_access_condition(HOST, AccessBlockKind::Forbidden, 403, None, "one page")
            .await;
        check!(fetcher.refused_admission(HOST, URL).await.is_none());
        check!(fetcher
            .refused_admission("other.example.test", URL)
            .await
            .is_none());
        Ok(())
    }

    #[tokio::test]
    async fn a_missing_lane_row_closes_admission_for_its_host_only() -> TestResult {
        let dir = tempfile::tempdir()?;
        let fetcher = fetcher_in(dir.path())?;
        fetcher
            .record_access_condition(
                HOST,
                AccessBlockKind::BrowserUnavailable,
                0,
                None,
                "no lane installed",
            )
            .await;
        let refusal = fetcher
            .refused_admission(HOST, URL)
            .await
            .ok_or("a missing lane closes admission")?;
        check!(!refusal.retryable());
        check!(fetcher
            .refused_admission("other.example.test", URL)
            .await
            .is_none());
        Ok(())
    }

    #[tokio::test]
    async fn human_verification_outranks_a_cooldown() -> TestResult {
        let dir = tempfile::tempdir()?;
        let fetcher = fetcher_in(dir.path())?;
        fetcher
            .record_access_condition(HOST, AccessBlockKind::RateLimited, 429, Some(60), "slow")
            .await;
        fetcher
            .record_access_condition(
                HOST,
                AccessBlockKind::HumanRequired,
                0,
                None,
                "challenge page",
            )
            .await;
        let refusal = fetcher
            .refused_admission(HOST, URL)
            .await
            .ok_or("a closing row exists")?;
        check!(matches!(refusal, FetchError::BrowserLane { .. }));
        check!(!refusal.retryable());
        Ok(())
    }
}
