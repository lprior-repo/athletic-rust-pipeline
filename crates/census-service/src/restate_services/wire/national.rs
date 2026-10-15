use census_domain::model::{Revision, SchoolYear};
use census_domain::UsJurisdiction;
use serde::{Deserialize, Serialize};

use super::school_address_join::SchoolAddressJoinRequest;
use super::{default_concurrency, default_source_parallelism};
use super::{HistoryWindow, PublicationContract};

fn default_pass_budget() -> usize {
    96
}

fn default_pass_delay_seconds() -> u64 {
    900
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NationalRequest {
    pub season: SchoolYear,
    pub revision: Revision,
    pub history: HistoryWindow,
    #[serde(default, skip_serializing_if = "PublicationContract::is_legacy")]
    pub publication_contract: PublicationContract,
    #[serde(default)]
    pub jurisdictions: Vec<UsJurisdiction>,
    #[serde(default)]
    pub refresh: bool,
    #[serde(default)]
    pub limit_per_state: Option<usize>,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default)]
    pub observed_on: Option<String>,
    #[serde(default)]
    pub authorized_hosts: Vec<String>,
    #[serde(default = "default_source_parallelism")]
    pub source_parallelism: usize,
    #[serde(default)]
    pub school_address: Option<SchoolAddressJoinRequest>,
    #[serde(default = "default_pass_budget")]
    pub pass_budget: usize,
    #[serde(default = "default_pass_delay_seconds")]
    pub pass_delay_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactSummary {
    pub jurisdiction: UsJurisdiction,
    pub identity: String,
    pub status: ContactStatus,
    pub source_rows: Option<usize>,
    pub errors: Option<usize>,
    pub unfinished: Option<usize>,
    #[serde(default)]
    pub sources: Vec<SourceStageOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStageOutcome {
    pub slug: String,
    pub outcome: super::StageOutcome,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactStatus {
    Complete,
    Partial,
    Terminal,
    Exhausted,
    Interrupted,
    HandlerFailed,
}
