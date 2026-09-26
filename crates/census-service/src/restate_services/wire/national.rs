//! The national-run family: the request that fans a run out over the jurisdiction set, and the
//! report it produces.
//!
//! Split from the wire root for the reason its sibling families are — the root declares the
//! protocol's surface, and each family declares its own types. [`NationalRequest`] takes its
//! concurrency default from the root, where that function sits next to the request that names it.

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_reconcile::identity::Revision;
use serde::{Deserialize, Serialize};

use super::default_concurrency;

/// The national run's request. `jurisdictions` empty means all fifty states and the District of
/// Columbia, in declaration order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NationalRequest {
    pub season: SchoolYear,
    pub revision: Revision,
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
    /// Operator-authorized hosts for the run, fanned out to every jurisdiction: their robots.txt
    /// rules are recorded as `robots_authorized` instead of blocking requests. Absent means none,
    /// which is the current behavior.
    #[serde(default)]
    pub authorized_hosts: Vec<String>,
}

/// One jurisdiction's row in the national report.
///
/// The walk's three outcomes are separate on purpose. `rosters_done` is what this traversal
/// fetched, `rosters_skipped` is what the journal already held, and `rosters_owed` is what the run
/// left unfinished — a state whose host refused requests (§69) stops the walk with most of its
/// index still owed. Without the last two an operator reading a blocked state sees a small state.
///
/// Both are `Option` because a report written by an earlier revision does not carry them: a missing
/// denominator must read as unknown, never as zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionSummary {
    pub jurisdiction: UsJurisdiction,
    pub identity: String,
    pub teams: usize,
    pub rosters_done: usize,
    pub rosters_skipped: usize,
    /// Rosters this run left unfinished: no journal entry, no fetched page.
    #[serde(default)]
    pub rosters_owed: Option<usize>,
    /// The host refused at least one roster with HTTP 403/429, which ends the state's requests.
    #[serde(default)]
    pub blocked: Option<bool>,
    pub athletes: usize,
    pub class_of_2027: usize,
}

/// A jurisdiction whose run failed. The national run keeps going: one jurisdiction's source outage
/// is a row in this list, never a failed national run (§69).
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
    pub teams_total: usize,
    pub athletes_total: usize,
    pub class_of_2027_total: usize,
    pub today: String,
}
