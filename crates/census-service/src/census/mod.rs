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
#[cfg(feature = "native-fault-injection")]
pub use sweep::install_boundary_hook;
pub use sweep::{collect_milesplit, collect_state_rosters, collect_state_teams};

pub const DEFAULT_ORIGIN_LOCK_ROOT: &str = "var/locks";

#[derive(Debug, Clone)]
pub struct CollectOptions {
    pub jurisdictions: Vec<UsJurisdiction>,
    pub limit_per_state: Option<usize>,
    pub concurrency: usize,
    pub state_concurrency: usize,
    pub refresh: bool,
    pub school_year: SchoolYear,
    pub observed_on: String,
    pub revision: std::num::NonZeroU32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateProgress {
    #[serde(rename = "state")]
    pub jurisdiction: UsJurisdiction,
    pub rosters_total: usize,
    pub rosters_committed: usize,
    pub rosters_remaining: usize,
    pub rosters_skipped: usize,
    pub athletes: usize,
    pub class_of_2027: usize,
    pub class_of_2027_boys: usize,
    pub class_of_2027_girls: usize,
    pub errors: Vec<String>,
    pub blocked: bool,
    pub blocked_skipped: usize,
    pub teams: usize,
}

impl StateProgress {
    pub fn is_terminal(&self) -> bool {
        self.rosters_remaining == 0 && self.blocked_skipped == 0 && !self.blocked
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CollectReport {
    pub states: Vec<StateProgress>,
    pub teams_total: usize,
    pub rosters_fetched: usize,
    pub athletes_total: usize,
    pub class_of_2027_total: usize,
    pub errors: u64,
    pub elapsed_seconds: f64,
    pub transport: TransportReport,
    pub access_conditions: Vec<SourceAccessCondition>,
    pub blocked_hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceTraffic {
    pub host: String,
    pub requests: u64,
    pub physical_requests: u64,
    pub cache_hits: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TransportReport {
    pub requests: u64,
    pub cache_hits: u64,
    pub physical_requests: u64,
    pub bytes: u64,
    pub avg_latency_ms: Option<u64>,
    pub p50_latency_ms: Option<u64>,
    pub p95_latency_ms: Option<u64>,
    pub p99_latency_ms: Option<u64>,
    pub rate_limited: u64,
    pub timeouts: u64,
    pub errors: u64,
    pub verified_records: u64,
    pub verified_records_per_physical_request: Option<f64>,
    pub sources: Vec<SourceTraffic>,
}

impl TransportReport {
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

pub fn teams_phase(jurisdiction: UsJurisdiction) -> String {
    format!(
        "milesplit_teams_{}",
        jurisdiction.code().to_ascii_lowercase()
    )
}

pub fn rosters_phase(
    jurisdiction: UsJurisdiction,
    school_year: SchoolYear,
    revision: std::num::NonZeroU32,
) -> String {
    format!(
        "milesplit_rosters_{}_{}_{}",
        jurisdiction.code().to_ascii_lowercase(),
        school_year.get(),
        revision
    )
}
