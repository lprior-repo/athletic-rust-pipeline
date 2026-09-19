use crate::domain::identity::{AthleteId, EvidenceDigest};
use crate::domain::name::CanonicalName;
use serde::{Deserialize, Serialize};

/// Candidate classification for ranking index entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RankingCandidateKind {
    Individual,
    RelayMember,
}

/// A single source ranking row with provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankingSourceRow {
    pub result_id: u64,
    pub row_number: u64,
}

/// Reference to a ranking roster observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingRosterObservation {
    pub result_id: u64,
    pub present: bool,
}

/// A ranking page index entry referencing a candidate row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingCandidateEntry {
    pub name: CanonicalName,
    pub athlete_id: AthleteId,
    pub kind: RankingCandidateKind,
    pub record_index: u32,
    pub result_id: u64,
}

/// A ranking page index with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingPageIndex {
    pub collection: EvidenceDigest,
    pub event_short: String,
    pub page: u32,
    pub checkpoint: EvidenceDigest,
    pub rows: Vec<RankingSourceRow>,
    pub candidates: Vec<RankingCandidateEntry>,
    pub rosters: Vec<RankingRosterObservation>,
}

/// A ranking lookup result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingLookup {
    pub records: Vec<RankingRecordRef>,
    pub truncated: bool,
}

/// A ranking record reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingRecordRef {
    pub athlete_id: AthleteId,
    pub checkpoint: EvidenceDigest,
    pub kind: RankingCandidateKind,
    pub record_index: u32,
}

/// Collection-level ranking stats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingCollectionStats {
    pub unique_athletes: u64,
}

/// Event-level ranking stats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingEventStats {
    pub pages: u64,
    pub source_results: u64,
    pub row_positions: u64,
    pub max_row_position: u64,
    pub grade11_individual_results: u64,
    pub grade11_relay_member_results: u64,
    pub unique_athletes: u64,
    pub unresolved_roster_results: u64,
}
