//! Derived rows the census keeps beside its canonical entities: source-object identities, the
//! retained conflict and review queues, coverage measurements, and one row per finished pass.
//!
//! These are [values](crate::model), not tables. They carry no clock, no store handle and no JSON
//! value, so the domain crate stays pure and the store decides how a row is keyed and durable. Every
//! id here is a deterministic function of the row's own facts, which is what lets the same finding be
//! re-derived on a later run without minting a second case.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::SourceNamespace;
use crate::UsJurisdiction;

/// The canonical table a source object resolved to.
///
/// Half of the §31 join key `SourceSystem | SourceObjectId -> CanonicalId`; the other half is the
/// provider's own id, which rides verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceEntityKind {
    Athletes,
    Schools,
    Coaches,
    Teams,
    Meets,
    Events,
    Performances,
}

impl SourceEntityKind {
    /// Every kind, in table order.
    pub const ALL: [Self; 7] = [
        Self::Athletes,
        Self::Schools,
        Self::Coaches,
        Self::Teams,
        Self::Meets,
        Self::Events,
        Self::Performances,
    ];

    /// The kind's slug: the name of the canonical table it points at.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Athletes => "athletes",
            Self::Schools => "schools",
            Self::Coaches => "coaches",
            Self::Teams => "teams",
            Self::Meets => "meets",
            Self::Events => "events",
            Self::Performances => "performances",
        }
    }
}

/// One source object observed for one canonical row.
///
/// The row is evidence, not an index into a source: it records that a provider's own id resolved to
/// a canonical id, with the URL it was read from when one exists. A provider that renumbers or
/// re-points an id writes a *new* row, so the earlier join stays visible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceObjectIdentity {
    /// The store row id: one row per source object, whatever canonical row it joined.
    pub id: String,
    pub namespace: SourceNamespace,
    pub entity: SourceEntityKind,
    pub source_id: String,
    pub canonical_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl SourceObjectIdentity {
    /// Mint the identity of one source object.
    pub fn new(
        namespace: SourceNamespace,
        entity: SourceEntityKind,
        source_id: impl Into<String>,
        canonical_id: impl Into<String>,
    ) -> Self {
        let source_id = source_id.into();
        Self {
            id: format!("{}:{}:{source_id}", namespace, entity.slug()),
            namespace,
            entity,
            source_id,
            canonical_id: canonical_id.into(),
            url: None,
        }
    }

    /// Carry the URL the object was read from.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
}

/// A conflict the merge kept: two canonical rows a stored key says are the same subject.
///
/// Retained rather than resolved, because resolving it is a decision the evidence does not make — the
/// row exists so an operator or the review lane can act on a subject instead of on a count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedConflict {
    pub id: String,
    /// The family label the published queue prints.
    pub family: String,
    /// The canonical row the finding is about.
    pub subject_id: String,
    /// The subject as a human reads it (name, school, meet).
    pub subject: String,
    /// Why the row is unresolved.
    pub detail: String,
}

impl RetainedConflict {
    /// Mint a conflict row. The id binds family and subject, so one subject yields one row per family
    /// however often the derivation runs.
    pub fn new(
        family: &str,
        subject_id: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        let subject_id = subject_id.into();
        Self {
            id: format!("{family}:{subject_id}"),
            family: family.to_string(),
            subject_id,
            subject: subject.into(),
            detail: detail.into(),
        }
    }
}

/// Where a review case stands.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    /// No decision yet: the case is what the review lane is for.
    #[default]
    Pending,
    /// The lane adjudicated it (for example the identity model returned a verdict the merge applied).
    Resolved,
    /// The lane looked and left it: the evidence does not decide, so the row stays visible.
    Retained,
}

/// One case the review lane owns (§32).
///
/// The id binds the finding, not the run: identical evidence reuses one case, so a repeated
/// derivation never asks the model about the same package twice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCase {
    pub id: String,
    pub family: String,
    pub subject_id: String,
    pub subject: String,
    pub detail: String,
    pub state: ReviewState,
}

impl ReviewCase {
    /// Mint a pending case for one finding.
    pub fn pending(
        family: &str,
        subject_id: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        let subject_id = subject_id.into();
        Self {
            id: format!("{family}:{subject_id}"),
            family: family.to_string(),
            subject_id,
            subject: subject.into(),
            detail: detail.into(),
            state: ReviewState::Pending,
        }
    }
}

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

#[cfg(test)]
#[path = "records_tests.rs"]
mod tests;

/// One meet a source enumerated but whose results this program has not necessarily read: the §30
/// `source_meets` row, and the meet census's durable output.
///
/// Every field but `id` is what the enumerating page said. `observed_on` is the day the row was
/// first seen, and [`Entity::merge`] keeps the earliest sighting while filling any field a later
/// sighting publishes and this one does not — a meet's name and venue are properties of the meet, so
/// a re-crawl may complete the row but may not rewrite its identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMeetRef {
    /// `{source}:{source_meet_id}` — the §31 key from the provider's own id to this meet.
    pub id: String,
    pub source: String,
    pub source_meet_id: String,
    pub jurisdiction: UsJurisdiction,
    /// The season selector the enumeration used: `cc`, `indoor` or `outdoor`.
    pub season: String,
    /// The season's start year, matching [`AcademicYear`]'s convention.
    pub year: u16,
    pub name: String,
    /// `yyyy-mm-dd` when the enumerating row published both a month and a day, else `None`.
    pub date: Option<String>,
    pub venue: String,
    /// The source page a results pull would read.
    pub results_url: String,
    /// The day this row was first observed, `yyyy-mm-dd`.
    pub observed_on: String,
}

impl SourceMeetRef {
    /// The row id for one source object.
    pub fn row_id(source: &str, source_meet_id: &str) -> String {
        format!("{source}:{source_meet_id}")
    }

    /// The season as the typed value, when the row's own string names one.
    pub fn season_of(&self) -> Option<&str> {
        match self.season.as_str() {
            "cc" | "indoor" | "outdoor" => Some(self.season.as_str()),
            _ => None,
        }
    }
}
