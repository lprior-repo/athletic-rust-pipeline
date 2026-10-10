use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_reconcile::identity::Revision;
use serde::{Deserialize, Serialize};

use super::school_address_join::{SchoolAddressJoinReply, SchoolAddressJoinRequest};
use super::{default_concurrency, default_source_parallelism};
use super::{HistoryWindow, JurisdictionOwed};

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
pub struct JurisdictionSummary {
    pub jurisdiction: UsJurisdiction,
    pub identity: String,
    pub rosters_total: usize,
    pub rosters_committed: usize,
    pub rosters_remaining: usize,
    pub rosters_skipped: usize,
    pub blocked: bool,
    pub athletes: usize,
    pub class_of_2027: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NationalFailure {
    pub jurisdiction: UsJurisdiction,
    pub identity: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NationalReport {
    pub season: SchoolYear,
    pub revision: Revision,
    pub jurisdictions: Vec<JurisdictionSummary>,
    pub failures: Vec<NationalFailure>,
    #[serde(default)]
    pub owed: Vec<JurisdictionOwed>,
    pub rosters_total: usize,
    pub athletes_total: usize,
    pub class_of_2027_total: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub school_address: Option<SchoolAddressJoinReply>,
    pub today: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactSummary {
    pub jurisdiction: UsJurisdiction,
    pub identity: String,
    pub status: ContactStatus,
    pub source_rows: Option<usize>,
    pub errors: Option<usize>,
    pub unfinished: Option<usize>,
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
