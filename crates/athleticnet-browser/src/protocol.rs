//! Capture shapes the transport publishes and the pipeline records.
//!
//! A browser response carries what the page said; a rankings capture carries what the in-page
//! request asked for. Both cross the crate boundary: the pipeline stores them as receipts, and the
//! census reads the same observations when it acquires Athletic.net through this crate.

use serde::{Deserialize, Serialize};

pub const MAX_SOURCE_RESPONSE_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RankingsCapture {
    Navigation,
    Results,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankingPageObservation {
    pub capture: RankingsCapture,
    pub request_method: String,
    pub request_url: String,
    pub request_body: Option<String>,
    pub next_page: Option<u32>,
}
