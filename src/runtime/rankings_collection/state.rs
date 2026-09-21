use crate::domain::identity::EvidenceDigest;
use crate::runtime::protocol::FetchOutcome;
use crate::runtime::rankings::RankingsScope;
use crate::runtime::Runtime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct RankingsCollectionState {
    pub runtime: Arc<Runtime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionRequest {
    pub source_snapshot: EvidenceDigest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventProgress {
    pub event_short: String,
    pub event_id: u64,
    pub is_relay: bool,
    pub next_page: u32,
    pub head_checkpoint: Option<EvidenceDigest>,
    pub page_count: u32,
    pub terminal: bool,
}

/// Deterministic collection fingerprint from revision + source snapshot.
pub fn collection_fingerprint(
    revision: &str,
    source: &EvidenceDigest,
) -> anyhow::Result<EvidenceDigest> {
    use crate::runtime::identity::fingerprint;
    let collection = (revision, source);
    fingerprint(&collection)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionState {
    pub source_snapshot: EvidenceDigest,
    pub scope: RankingsScope,
    pub phase: CollectionPhase,
    pub generation: u64,
    pub events: Vec<EventProgress>,
    pub current_event_index: usize,
    pub final_snapshot: Option<EvidenceDigest>,
    pub catalog_ref: Option<EvidenceDigest>,
    pub plan_ref: Option<EvidenceDigest>,
    pub absent_families: Vec<String>,
    pub catalog_outcome: Option<FetchOutcome>,
    pub pause_reason: Option<CollectionPauseReason>,
    pub last_outcome: Option<FetchOutcome>,
    pub pause_detail: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum CollectionPhase {
    CatalogStep,
    PageStep,
    Complete,
    Paused(CollectionPauseReason),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CollectionPauseReason {
    Manual,
    SourceFailure,
    InvalidEvidence,
    PageLimit,
    ParserError,
}
