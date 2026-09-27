
use serde::{Deserialize, Serialize};

use super::super::SourceNamespace;
use crate::UsJurisdiction;

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
    pub const ALL: [Self; 7] = [
        Self::Athletes,
        Self::Schools,
        Self::Coaches,
        Self::Teams,
        Self::Meets,
        Self::Events,
        Self::Performances,
    ];

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceObjectIdentity {
    pub id: String,
    pub namespace: SourceNamespace,
    pub entity: SourceEntityKind,
    pub source_id: String,
    pub canonical_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl SourceObjectIdentity {
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

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMeetRef {
    pub id: String,
    pub source: String,
    pub source_meet_id: String,
    pub jurisdiction: UsJurisdiction,
    pub season: String,
    pub year: u16,
    pub name: String,
    pub date: Option<String>,
    pub venue: String,
    pub results_url: String,
    pub observed_on: String,
}

impl SourceMeetRef {
    pub fn row_id(source: &str, source_meet_id: &str) -> String {
        format!("{source}:{source_meet_id}")
    }
}
