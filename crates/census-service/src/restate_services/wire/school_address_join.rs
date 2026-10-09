use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::school_address::{Counters, JoinReport};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchoolAddressJoinRequest {
    #[serde(default)]
    pub generation: Option<String>,
    #[serde(default)]
    pub urls: BTreeMap<String, String>,
    #[serde(default)]
    pub dates: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchoolAddressJoinReply {
    pub mode: String,
    pub generation: String,
    pub report: String,
    pub outcomes: String,
    pub counters: Counters,
}

impl From<JoinReport> for SchoolAddressJoinReply {
    fn from(report: JoinReport) -> Self {
        Self {
            mode: report.mode,
            generation: report.generation,
            report: report.report,
            outcomes: report.outcomes,
            counters: report.counters,
        }
    }
}
