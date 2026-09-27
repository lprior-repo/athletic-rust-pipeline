
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageScope {
    Jurisdiction,
    Source,
}

impl CoverageScope {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Jurisdiction => "jurisdiction",
            Self::Source => "source",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageRow {
    pub id: String,
    pub scope: CoverageScope,
    pub subject: String,
    pub metrics: BTreeMap<String, u64>,
}

impl CoverageRow {
    pub fn new(scope: CoverageScope, subject: impl Into<String>) -> Self {
        let subject = subject.into();
        Self {
            id: format!("{}:{subject}", scope.slug()),
            scope,
            subject,
            metrics: BTreeMap::new(),
        }
    }

    pub fn with(mut self, metric: &str, value: u64) -> Self {
        self.metrics.insert(metric.to_string(), value);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionSnapshot {
    pub id: String,
    pub phase: String,
    pub finished_at: String,
    pub observations: BTreeMap<String, u64>,
}

impl CollectionSnapshot {
    pub fn new(phase: impl Into<String>, finished_at: impl Into<String>) -> Self {
        let phase = phase.into();
        let finished_at = finished_at.into();
        Self {
            id: format!("{phase}:{finished_at}"),
            phase,
            finished_at,
            observations: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessBlockKind {
    Forbidden,
    RateLimited,
    RobotsDisallowed,
    Timeout,
    Unavailable,
    BrowserUnavailable,
    HumanRequired,
}

impl AccessBlockKind {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Forbidden => "forbidden",
            Self::RateLimited => "rate_limited",
            Self::RobotsDisallowed => "robots_disallowed",
            Self::Timeout => "timeout",
            Self::Unavailable => "unavailable",
            Self::BrowserUnavailable => "browser_unavailable",
            Self::HumanRequired => "human_required",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceAccessCondition {
    pub id: String,
    pub source: String,
    pub host: String,
    pub kind: AccessBlockKind,
    pub status: u16,
    pub observed_at: String,
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_until: Option<String>,
}

impl SourceAccessCondition {
    pub fn new(
        source: impl Into<String>,
        host: impl Into<String>,
        kind: AccessBlockKind,
        status: u16,
        observed_at: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        let host = host.into();
        Self {
            id: format!("{}:{host}", kind.slug()),
            source: source.into(),
            host,
            kind,
            status,
            observed_at: observed_at.into(),
            detail: detail.into(),
            retry_after_seconds: None,
            cooldown_until: None,
        }
    }

    pub fn with_retry_after(mut self, seconds: Option<u64>) -> Self {
        self.retry_after_seconds = seconds;
        self
    }

    pub fn with_cooldown_until(mut self, until: Option<String>) -> Self {
        self.cooldown_until = until;
        self
    }

    pub fn is_blocking(&self, now_iso8601: &str) -> bool {
        match self.cooldown_until.as_deref() {
            Some(until) => until > now_iso8601,
            None => true,
        }
    }
}
