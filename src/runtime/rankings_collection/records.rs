use crate::domain::identity::EvidenceDigest;
use crate::runtime::protocol::FetchOutcome;
use crate::runtime::rankings::RankingsScope;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionFinalSnapshot {
    pub collection: EvidenceDigest,
    pub source_snapshot: EvidenceDigest,
    pub scope: RankingsScope,
    pub catalog_outcome: FetchOutcome,
    pub catalog_ref: Option<EvidenceDigest>,
    pub plan_ref: Option<EvidenceDigest>,
    pub event_heads: Vec<EventHeadRef>,
    pub coverage: CoverageSummary,
    pub unique_athletes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHeadRef {
    pub event_short: String,
    pub head_checkpoint: Option<EvidenceDigest>,
    pub page_count: u32,
    pub terminal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageSummary {
    pub total_requested: u64,
    pub completed: u64,
    pub absent_families: Vec<String>,
}

/// Immutable checkpoint with validated observation and provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingsPageCheckpoint {
    pub revision: String,
    pub collection: EvidenceDigest,
    pub event_short: String,
    pub page: u32,
    pub previous_checkpoint: Option<EvidenceDigest>,
    pub outcome: FetchOutcome,
    pub observation_digest: EvidenceDigest,
}

/// Public reference to a completed ranking collection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RankingCollectionRef {
    pub collection: EvidenceDigest,
    pub snapshot: EvidenceDigest,
}
