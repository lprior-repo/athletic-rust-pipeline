//! The §31 join: one row per source object, saying which canonical row it resolved to.
//!
//! A source object is an id a provider issued — an athlete, a school, a team — and the join records
//! that this provider's object became this canonical row. The row is evidence of a resolution, not an
//! index into a source: a provider that renumbers or re-points an id writes a *new* row, so the earlier
//! join stays visible and a decision can be read back.

use serde::{Deserialize, Serialize};

use super::super::SourceNamespace;
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

/// One meet a source enumerated but whose results this program has not necessarily read: the §30
/// `source_meets` row, and the meet census's durable output.
///
/// Every field but `id` is what the enumerating page said. `observed_on` is the day the row was
/// first seen, and the merge rule keeps the earliest sighting while filling any field a later
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
    /// The season's start year, matching [`SchoolYear`](crate::model::SchoolYear)'s convention.
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
