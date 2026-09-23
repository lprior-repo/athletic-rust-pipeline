use crate::domain::identity::EvidenceDigest;
use athleticnet_browser::protocol::RankingsCapture;
use serde::{Deserialize, Serialize};

/// A complete rankings collection plan covering all requested
/// event families for a single division scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingsPlan {
    pub collection: EvidenceDigest,
    pub list_id: u64,
    pub gender: String,
    pub grade: u8,
    pub events: Vec<RankedEvent>,
}

/// One event family within a RankingsPlan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedEvent {
    pub short: String,
    pub family: String,
    pub group: String,
    pub event_id: u64,
    pub page: u32,
    pub capture: RankingsCapture,
    pub is_relay: bool,
}

impl RankingsPlan {
    pub fn for_event(&self, short: &str) -> Option<&RankedEvent> {
        self.events.iter().find(|e| e.short == short)
    }
}
