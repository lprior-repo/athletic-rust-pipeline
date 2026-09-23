//! What a pass measured and what stopped it: coverage, collection snapshots, access conditions.
//!
//! These rows answer "how much is in the store, as of when, and what is in the way". They are states
//! rather than histories — one row per subject or per host, replaced by the next pass — so a reader
//! asking what a lane can do next does not have to replay every pass that ran before it.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What one coverage row measures: a place or a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageScope {
    /// A jurisdiction: the USPS code, or the unplaced label.
    Jurisdiction,
    /// A source namespace (`milesplit_athlete`, `timer_meet:wayzata`, …).
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

/// One coverage row: the §31 key `scope | subject -> measurements`, with named metrics rather than
/// positional columns, so a later pass can add a denominator without rewriting the readers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageRow {
    /// The store row id: one row per subject, because coverage is a state rather than a history.
    pub id: String,
    pub scope: CoverageScope,
    pub subject: String,
    pub metrics: BTreeMap<String, u64>,
}

impl CoverageRow {
    /// Mint a coverage row for one subject.
    pub fn new(scope: CoverageScope, subject: impl Into<String>) -> Self {
        let subject = subject.into();
        Self {
            id: format!("{}:{subject}", scope.slug()),
            scope,
            subject,
            metrics: BTreeMap::new(),
        }
    }

    /// Set one measurement.
    pub fn with(mut self, metric: &str, value: u64) -> Self {
        self.metrics.insert(metric.to_string(), value);
        self
    }
}

/// One finished pass over the store: what it saw and when it stopped (§29 collection snapshots).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionSnapshot {
    pub id: String,
    /// The pass that wrote the row (`derive`, `collect:<source>`, …).
    pub phase: String,
    /// The offset the pass finished at.
    pub finished_at: String,
    /// Appended-observation count per canonical table at that moment, keyed by table name. The
    /// counters track *appends*: the derived tables (`source_identities`, `conflicts`,
    /// `review_cases`, `coverage`, `snapshots`) replace their rows instead, so they stay at zero
    /// however many derivation passes have run. `consolidate` reports the merged row counts.
    pub observations: BTreeMap<String, u64>,
}

impl CollectionSnapshot {
    /// Mint the snapshot of one pass.
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

    /// Record one table's appended-observation count.
    pub fn with_observations(mut self, table: &str, observations: u64) -> Self {
        self.observations.insert(table.to_string(), observations);
        self
    }
}

/// What kind of access condition a source imposed on this client (§69).
///
/// Most kinds are the source's own answer. The last two are the browser lane's: there the answer is a
/// fact about this machine's runtime — no lane deployed, or a profile that needs a person — and a
/// reader that only knows the source's vocabulary would look for it in the wrong place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessBlockKind {
    /// The source answered 403 for a path its own robots rules allow.
    Forbidden,
    /// The source asked this client to slow down: 429, or a published limit it enforced.
    RateLimited,
    /// The source's robots rules disallow the path and the host is not operator-authorized.
    RobotsDisallowed,
    /// Requests to the host timed out often enough that the lane stopped.
    Timeout,
    /// The host could not be reached at all.
    Unavailable,
    /// The browser lane that acquires this source is not deployed or not configured on this machine,
    /// so the source cannot be fetched here at all. The unit stays owed; a later run on a machine
    /// with the lane serves it.
    BrowserUnavailable,
    /// The profile behind the browser lane needs a person before it serves this source again: a
    /// challenge page instead of the answer. No delay ends this one, so the row carries no cooldown
    /// and only an operator clears it.
    HumanRequired,
}

impl AccessBlockKind {
    /// The slug the row id is keyed by and the report prints.
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

/// One access condition a source imposed: the row that stops a lane paying for the same block twice.
///
/// The id binds the kind and the host, so a second observation refreshes the row instead of minting
/// a second one. `cooldown_until` is supplied by the caller — the collection layer is the only place
/// a clock exists — so the domain stores an instant without ever reading one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceAccessCondition {
    /// The store row id: `{kind.slug()}:{host}`.
    pub id: String,
    /// The adapter slug that observed the condition (`milesplit`, `wiaa_results`, …).
    pub source: String,
    pub host: String,
    pub kind: AccessBlockKind,
    /// The HTTP status that produced the row; `0` when the condition is not an HTTP status.
    pub status: u16,
    /// When the condition was observed (RFC 3339 UTC, `Z`).
    pub observed_at: String,
    pub detail: String,
    /// The source's own `Retry-After`, when it published one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
    /// When the block stops applying (RFC 3339 UTC, `Z`). `None` means it does not expire on its own,
    /// so the row keeps blocking until an operator re-derives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_until: Option<String>,
}

impl SourceAccessCondition {
    /// Mint the condition for one host.
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

    /// Carry the `Retry-After` the source published, when it published one.
    pub fn with_retry_after(mut self, seconds: Option<u64>) -> Self {
        self.retry_after_seconds = seconds;
        self
    }

    /// Carry the instant the block stops applying.
    pub fn with_cooldown_until(mut self, until: Option<String>) -> Self {
        self.cooldown_until = until;
        self
    }

    /// Whether this condition still blocks work at `now_iso8601`.
    ///
    /// Both sides are RFC 3339 UTC with a `Z` suffix, so the comparison is lexicographic on purpose:
    /// same shape, same zone, most-significant field first. A condition with no cooldown never
    /// expires on its own and therefore always blocks.
    pub fn is_blocking(&self, now_iso8601: &str) -> bool {
        match self.cooldown_until.as_deref() {
            Some(until) => until > now_iso8601,
            None => true,
        }
    }
}
