//! Collection orchestration: jurisdiction → teams → rosters → canonical entities, resumable at
//! every step.
//!
//! Resume model: the entity logs are append-only and the journal records each completed unit of work
//! (`<state>:<team_id>`). A run that is interrupted — or an operator who stops one deliberately —
//! re-invokes with the same arguments and only the unfinished units are fetched again. HTTP bodies
//! are additionally cached on disk, so even a re-fetch costs no network traffic unless `--refresh`.
//!
//! # Layout
//!
//! `sweep` walks a state's team index and then its rosters, `scope` decides which units a resumed
//! run still owes and which athletes the cohort counts, and `aggregate` folds the outcomes into
//! the run report and the merged snapshots. This file holds the options, the progress and report
//! types, and the journal phase keys the three share.

use census_domain::model::SchoolYear;
use census_domain::model::SourceAccessCondition;
use census_domain::UsJurisdiction;
use serde::{Deserialize, Serialize};

mod aggregate;
mod identity;
mod meets;
mod scope;
mod state;
mod sweep;
pub mod verify;

pub use aggregate::consolidate;
pub use identity::{Revision, WorkflowIdentity};
pub use meets::{collect_state_meets, select_meets, MeetCensus};
pub use state::{
    owed_jurisdictions, owed_source_objects, AcceptanceItem, CensusState, GapTally,
    JurisdictionStages, OpenWork, Phase, RetainedFindings, SealCounts, SealError, SealEvidence,
    SealedCensus, SourceObject, WorkbookCheck,
};
pub use sweep::{collect_milesplit, collect_state_rosters, collect_state_teams};
pub use verify::{
    missing_columns, sample_indices, sheets_matching_prefix, verify_athletes, verify_performances,
    ATHLETES_REQUIRED, PERFORMANCES_REQUIRED,
};

#[derive(Debug, Clone)]
pub struct CollectOptions {
    /// The jurisdictions this walk covers, in the caller's order.
    pub jurisdictions: Vec<UsJurisdiction>,
    pub limit_per_state: Option<usize>,
    pub concurrency: usize,
    /// How many state hosts to walk at once. Each host is still limited to one request at a time by
    /// the fetcher's per-host gate, so this only removes idle time between states.
    pub state_concurrency: usize,
    pub refresh: bool,
    pub school_year: SchoolYear,
    pub observed_on: String,
}

impl Default for CollectOptions {
    fn default() -> Self {
        Self {
            jurisdictions: UsJurisdiction::CENSUS_SCOPE.to_vec(),
            limit_per_state: None,
            concurrency: 4,
            state_concurrency: 4,
            refresh: false,
            school_year: SchoolYear(2026),
            observed_on: crate::net::today_iso(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateProgress {
    /// The jurisdiction this row summarizes; serializes as its USPS code under the historic
    /// `state` key, so stored reports keep their shape while the type stays validated.
    #[serde(rename = "state")]
    pub jurisdiction: UsJurisdiction,
    pub teams: usize,
    pub rosters_done: usize,
    pub rosters_skipped: usize,
    pub athletes: usize,
    pub class_of_2027: usize,
    pub class_of_2027_boys: usize,
    pub class_of_2027_girls: usize,
    pub empty_rosters: usize,
    pub errors: Vec<String>,
    /// §69: whether a hard access block (HTTP 403/429) ended this state's walk before its rosters
    /// were done. Defaulted on read so a report stored before the stop existed still decodes.
    #[serde(default)]
    pub blocked: bool,
    /// Rosters the walk dropped unfetched after that block: they stay owed, so a re-run resumes.
    #[serde(default)]
    pub blocked_skipped: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CollectReport {
    pub states: Vec<StateProgress>,
    pub teams_total: usize,
    pub rosters_fetched: usize,
    pub athletes_total: usize,
    pub class_of_2027_total: usize,
    pub requests: u64,
    pub cache_hits: u64,
    pub errors: u64,
    pub elapsed_seconds: f64,
    /// The access conditions the sources imposed on this run, sorted by row id (§69). Empty means
    /// no source refused this client.
    pub access_conditions: Vec<SourceAccessCondition>,
    /// Hosts whose condition still blocks work: a non-empty list is what tells a blocked run from a
    /// complete one.
    pub blocked_hosts: Vec<String>,
}

/// Team index phase key: `milesplit_teams_wi`.
fn teams_phase(jurisdiction: UsJurisdiction) -> String {
    format!(
        "milesplit_teams_{}",
        jurisdiction.code().to_ascii_lowercase()
    )
}

/// Roster phase key: `milesplit_rosters_wi`.
fn rosters_phase(jurisdiction: UsJurisdiction) -> String {
    format!(
        "milesplit_rosters_{}",
        jurisdiction.code().to_ascii_lowercase()
    )
}
#[cfg(test)]
mod verify_tests;
