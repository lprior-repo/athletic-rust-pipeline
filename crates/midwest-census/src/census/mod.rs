//! Collection orchestration: state → teams → rosters → canonical entities, resumable at every step.
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

use crate::sources::milesplit;
use census_domain::model::SchoolYear;
use serde::Serialize;

mod aggregate;
mod scope;
mod sweep;

pub use aggregate::consolidate;
pub use sweep::{collect_milesplit, collect_state_rosters, collect_state_teams};

#[derive(Debug, Clone)]
pub struct CollectOptions {
    pub states: Vec<String>,
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
            states: milesplit::SITES
                .iter()
                .map(|site| site.state.to_string())
                .collect(),
            limit_per_state: None,
            concurrency: 4,
            state_concurrency: 4,
            refresh: false,
            school_year: SchoolYear(2026),
            observed_on: crate::net::today_iso(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StateProgress {
    pub state: String,
    pub teams: usize,
    pub rosters_done: usize,
    pub rosters_skipped: usize,
    pub athletes: usize,
    pub class_of_2027: usize,
    pub class_of_2027_boys: usize,
    pub class_of_2027_girls: usize,
    pub empty_rosters: usize,
    pub errors: Vec<String>,
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
}

/// Team index phase key.
fn teams_phase(state: &str) -> String {
    format!("milesplit_teams_{}", state.to_ascii_lowercase())
}

/// Roster phase key.
fn rosters_phase(state: &str) -> String {
    format!("milesplit_rosters_{}", state.to_ascii_lowercase())
}
