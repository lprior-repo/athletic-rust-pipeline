mod archive;
mod classify;
mod map;
mod parse;
mod run;

use crate::AdapterContext;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam, Sport,
};
use census_domain::UsJurisdiction;
use std::collections::HashMap;

pub use archive::{archive_artifacts, ArchiveArtifact};
pub use classify::{artifact_format, level_of, school_year_for, ArtifactFormat};
pub use parse::parse_result_body;
pub use run::collect;

pub const ARCHIVES: [(&str, Sport); 4] = [
    (
        "https://www.wiaawi.org/sports/boys-track-field/boys-track-field-state-archive",
        Sport::OutdoorTrack,
    ),
    (
        "https://www.wiaawi.org/sports/girls-track-field/girls-track-field-state-archive",
        Sport::OutdoorTrack,
    ),
    (
        "https://www.wiaawi.org/sports/boys-cross-country/boys-cross-country-state-archive",
        Sport::CrossCountry,
    ),
    (
        "https://www.wiaawi.org/sports/girls-cross-country/girls-cross-country-state-archive",
        Sport::CrossCountry,
    ),
];

pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub seasons: Vec<i16>,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

const PARSE_VERSION: u32 = 7;

#[derive(Debug, Default)]
struct Accumulator {
    meets: HashMap<String, CanonicalMeet>,
    events: HashMap<String, CanonicalEvent>,
    teams: HashMap<String, CanonicalTeam>,
    athletes: HashMap<String, CanonicalAthlete>,
    performances: HashMap<String, CanonicalPerformance>,
    unsupported: crate::cohort::UnsupportedCohortRows,
}

#[derive(Debug, Default)]
struct Stats {
    artifacts_seen: usize,
    artifacts_parsed: usize,
    artifacts_unparsed: usize,
    artifacts_unsupported: usize,
    artifacts_parse_failed: usize,
    artifacts_failed: usize,
    pdf_parsed: usize,
    pdf_unparsed: usize,
    pdf_tool_failures: usize,
    pdf_layouts: std::collections::BTreeMap<String, usize>,
    rows: usize,
    rows_with_grade: usize,
    rows_without_school: usize,
    relay_legs: usize,
    events: usize,
    school_resolved: HashMap<&'static str, usize>,
    unresolved: HashMap<String, usize>,
    formats: HashMap<String, usize>,
    seasons: HashMap<i16, usize>,
}

async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.requests, stats.cache_hits)
}

#[cfg(test)]
mod tests;
