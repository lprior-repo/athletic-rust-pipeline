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

use census_crawl::net::FetchStats;
use census_domain::model::SchoolYear;
use census_domain::model::SourceAccessCondition;
use census_domain::UsJurisdiction;
use serde::{Deserialize, Serialize};

mod aggregate;
mod meets;
mod scope;
pub mod seal;
mod state;
mod sweep;

#[cfg(test)]
mod tests;

pub use aggregate::consolidate;
pub use meets::{
    collect_state_meets, select_meets, MeetCensus, MeetSourceRows, SeasonScope, SOURCE,
};
pub use state::{
    owed_cohort_decisions, owed_identity_candidates, owed_jurisdictions, owed_source_objects,
    silent_source_objects, AcceptanceItem, CensusState, GapTally, JurisdictionStages, OpenWork,
    Phase, RetainedFindings, SealCounts, SealError, SealEvidence, SealedCensus, SourceObject,
    WorkbookCheck,
};
pub use sweep::{collect_milesplit, collect_state_rosters, collect_state_teams};

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
            school_year: SchoolYear::DEFAULT,
            observed_on: census_crawl::net::today_iso(),
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
    /// Walk failures: a state that could not complete, plus the access failures it recorded. This is
    /// not the transport's error count — that lives in [`TransportReport::errors`] — because a walk
    /// can fail on a payload it received, and a request can fail for a walk that then succeeded
    /// elsewhere.
    pub errors: u64,
    pub elapsed_seconds: f64,
    /// §45's transport account: per-source traffic, latency and the run's efficiency metric.
    pub transport: TransportReport,
    /// The access conditions the sources imposed on this run, sorted by row id (§69). Empty means
    /// no source refused this client.
    pub access_conditions: Vec<SourceAccessCondition>,
    /// Hosts whose condition still blocks work: a non-empty list is what tells a blocked run from a
    /// complete one.
    pub blocked_hosts: Vec<String>,
}

/// §45's per-source row: what one source was asked for, and what it answered itself.
#[derive(Debug, Clone, Serialize)]
pub struct SourceTraffic {
    /// The source's host, as the client keyed it.
    pub host: String,
    pub requests: u64,
    /// Requests the source itself answered: cache hits excluded.
    pub physical_requests: u64,
    pub cache_hits: u64,
    pub bytes: u64,
}

/// §45's transport account for a run: what the sources saw, what it cost, and what it produced.
///
/// Read from the fetcher's own counters at the end of a walk, so a report is a reading of the client
/// that made the requests rather than a second ledger kept beside it. Latencies are histogram
/// edges: an upper bound at the client's resolution, never a claim about one request's timing.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TransportReport {
    pub requests: u64,
    pub cache_hits: u64,
    pub physical_requests: u64,
    pub bytes: u64,
    /// Mean transport latency in milliseconds; `None` when nothing was measured.
    pub avg_latency_ms: Option<u64>,
    pub p50_latency_ms: Option<u64>,
    pub p95_latency_ms: Option<u64>,
    pub p99_latency_ms: Option<u64>,
    pub rate_limited: u64,
    pub timeouts: u64,
    pub errors: u64,
    /// The records this run verified as useful, and their ratio to what the sources physically saw.
    pub verified_records: u64,
    pub verified_records_per_physical_request: Option<f64>,
    /// One row per source, busiest first, the host breaking ties.
    pub sources: Vec<SourceTraffic>,
}

impl TransportReport {
    /// Read a run's §45 account from the client's counters.
    ///
    /// `verified_records` is passed in rather than derived here: what a run usefully produced is the
    /// walk's own count, and this type knows only the traffic that produced it.
    pub fn from_stats(stats: &FetchStats, verified_records: u64) -> Self {
        let mut sources: Vec<SourceTraffic> = stats
            .per_host
            .iter()
            .map(|(host, traffic)| SourceTraffic {
                host: host.clone(),
                requests: traffic.requests,
                physical_requests: traffic.physical_requests(),
                cache_hits: traffic.cache_hits,
                bytes: traffic.bytes,
            })
            .collect();
        // Busiest first, host as the tie-break: the report is a projection that must render the same
        // way twice, so equal traffic cannot be left in hash order.
        sources.sort_by(|left, right| {
            right
                .requests
                .cmp(&left.requests)
                .then_with(|| left.host.cmp(&right.host))
        });
        Self {
            requests: stats.requests,
            cache_hits: stats.cache_hits,
            physical_requests: stats.physical_requests(),
            bytes: stats.bytes_downloaded,
            avg_latency_ms: stats.latency_ms_avg(),
            p50_latency_ms: stats.latency_percentile_ms(50),
            p95_latency_ms: stats.latency_percentile_ms(95),
            p99_latency_ms: stats.latency_percentile_ms(99),
            rate_limited: stats.rate_limited,
            timeouts: stats.timeouts,
            errors: stats.errors,
            verified_records,
            verified_records_per_physical_request: stats
                .useful_records_per_physical_request(verified_records),
            sources,
        }
    }
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
