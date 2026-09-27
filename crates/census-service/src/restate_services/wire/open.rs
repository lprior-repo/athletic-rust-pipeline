
use census_domain::UsJurisdiction;
use serde::{Deserialize, Serialize};

use crate::census::JurisdictionStages;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenWorkRequest {
    pub season: i16,
    #[serde(default = "first_revision")]
    pub revision: u32,
    #[serde(default)]
    pub source_objects: Vec<String>,
}

fn first_revision() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionOpen {
    pub jurisdiction: UsJurisdiction,
    pub identity: String,
    pub stages: JurisdictionStages,
    pub unreadable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceObjectOpen {
    pub endpoint: String,
    pub observations: u64,
    pub windows: u64,
    pub unreadable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenWorkReply {
    pub season: String,
    pub revision: u32,
    pub jurisdiction_sweeps: Option<u64>,
    pub source_objects: Option<u64>,
    #[serde(default)]
    pub silent_sources: Vec<String>,
    pub jurisdictions: Vec<JurisdictionOpen>,
    pub endpoints: Vec<SourceObjectOpen>,
}
