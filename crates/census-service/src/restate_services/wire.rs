use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::census::{MeetCensus, StateProgress};
use census_reconcile::identity::Revision;

use super::results_arms::ResultsStageOutcome;

pub(super) mod ingest;

pub use ingest::{
    EndpointObservation, IngestReply, IngestRequest, IngestState, SweepReport, SweepRequest,
    WindowRequest,
};

pub(super) mod open;

pub use open::{JurisdictionOpen, OpenWorkReply, OpenWorkRequest, SourceObjectOpen};

pub(super) mod plan;

pub use plan::{RefusedSource, SourcePlan};

pub(super) mod seal;

pub use seal::{BindRunReply, BindRunRequest, SealItem, SealRef, SealReply, SealRequest};

pub(super) mod school_address_join;

pub use school_address_join::{SchoolAddressJoinReply, SchoolAddressJoinRequest};

pub(super) mod national;

pub use national::{JurisdictionSummary, NationalFailure, NationalReport, NationalRequest};

mod teams;
pub use teams::{CompletedTeams, IncompleteTeams, TeamsFailure, TeamsStage};
mod teams_sources;
pub use teams_sources::{
    TeamsAttemptProgress, TeamsSourceFailure, TeamsSourceInspection, TeamsSourceOutcome,
    TeamsSourceRequest,
};

#[cfg(test)]
mod teams_tests;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCount {
    pub table: String,
    pub rows: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReply {
    pub tables: Vec<TableCount>,
    pub observations: u64,
    pub bytes_on_disk: u64,
    pub today: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsolidateRequest {
    #[serde(default)]
    pub tables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidatedTable {
    pub table: String,
    pub rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidateReply {
    pub tables: Vec<ConsolidatedTable>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReportRequest {
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportReply {
    pub scope: String,
    pub generated_on: String,
    pub totals: Value,
    pub json_path: String,
    pub csv_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BestsRequest {
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub grad_year: Option<i16>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestsReply {
    pub cohort: String,
    pub rows: usize,
    pub jsonl: String,
    pub csv: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkbookRequest {
    #[serde(default)]
    pub grad_year: Option<i16>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub out: Option<String>,
    #[serde(default)]
    pub school_year: Option<i16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbookReply {
    pub path: String,
    pub grad_year: Option<i16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionRequest {
    pub jurisdiction: UsJurisdiction,
    pub season: SchoolYear,
    pub revision: Revision,
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
}

fn default_concurrency() -> usize {
    4
}

pub(super) fn default_source_parallelism() -> usize {
    census_crawl::net::DEFAULT_FAMILY_PARALLELISM
}

impl NationalRequest {
    pub fn for_jurisdiction(&self, jurisdiction: UsJurisdiction) -> JurisdictionRequest {
        JurisdictionRequest {
            jurisdiction,
            season: self.season,
            revision: self.revision,
            refresh: self.refresh,
            limit_per_state: self.limit_per_state,
            concurrency: self.concurrency,
            observed_on: self.observed_on.clone(),
            authorized_hosts: self.authorized_hosts.clone(),
            source_parallelism: self.source_parallelism,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageOutcome {
    pub records: usize,
    pub at: String,
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JurisdictionState {
    #[serde(default)]
    pub identity: String,
    #[serde(default)]
    pub plan: Option<SourcePlan>,
    pub teams: TeamsStage,
    #[serde(default)]
    pub rosters: Option<StateProgress>,
    #[serde(default)]
    pub consolidated: Option<Vec<ConsolidatedTable>>,
    #[serde(default)]
    pub meets: Option<MeetCensus>,
    #[serde(default)]
    pub meets_complete: bool,
    #[serde(default)]
    pub results: Option<ResultsStageOutcome>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionReport {
    pub identity: String,
    pub jurisdiction: UsJurisdiction,
    #[serde(default)]
    pub plan: SourcePlan,
    pub stages_run: Vec<String>,
    pub teams: usize,
    pub rosters: StateProgress,
    pub consolidated: Vec<ConsolidatedTable>,
    pub meets: MeetCensus,
    #[serde(default)]
    pub results: ResultsStageOutcome,
    pub completed_at: String,
}
