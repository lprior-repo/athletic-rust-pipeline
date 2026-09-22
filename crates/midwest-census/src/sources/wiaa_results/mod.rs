//! WIAA state result archive (Tier D — official result artifacts).
//!
//! The WIAA publishes every state, sectional and regional result file it has ever released on four
//! year-partitioned archive pages (boys/girls track & field, boys/girls cross country). The files are
//! not one format:
//!
//! | format | who publishes it | parsed |
//! |---|---|---|
//! | Hy-Tek HTML (Cocoa-converted `<p>` report) | PrimeTime Timing and other Hy-Tek timers | yes |
//! | Hy-Tek plain text (`.txt`) | HS timing systems from the 2000s | yes |
//! | RaceDay Scoring HTML tables | cross-country sectionals 2010s-2020s | yes |
//! | PDF / RTF | newer state finals | indexed, not parsed |
//!
//! The archive is the cheapest place in the platform to obtain **grade-bearing official results**:
//! every Hy-Tek and RaceDay row carries the athlete's grade at the time of the meet, which is exactly
//! the class-of-2027 evidence the census needs, and none of it costs an Athletic.net request.
//!
//! Entities are minted on the same deterministic keys the roster and association adapters use —
//! athletes from `(school, name, grad year, gender)`, teams from `(school, sport, gender, school
//! year)`, meets from `(state, date, name)` — so a WIAA result file reconciles with an existing
//! canonical athlete instead of creating a parallel one.
//!
//! Layout: `run` walks the archive and dispatches each artifact to its reader, `archive` extracts
//! the artifact links, `classify` decides an artifact's format, season and level, `parse` reads a
//! PDF release, and `map` assembles canonical meets, events and performances. This file holds the
//! published constants, the options, and the run state those pieces share.

mod archive;
mod classify;
mod map;
mod parse;
mod run;

use crate::sources::AdapterContext;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam, Sport,
};
use census_domain::UsJurisdiction;
use std::collections::HashMap;

pub use archive::{archive_artifacts, ArchiveArtifact};
pub use classify::{artifact_format, level_of, school_year_for, ArtifactFormat};
pub use parse::parse_result_body;
pub use run::collect;

/// The four WIAA archive pages, with the sport each one publishes.
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
    /// Stop after this many artifacts (smoke runs).
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    /// Restrict to these archive years; empty means every year on the archive pages.
    pub seasons: Vec<i16>,
    /// Unused: the archive is a single state. Kept for the uniform provider CLI shape.
    pub states: Vec<UsJurisdiction>,
    /// Unused: schools come from the consolidated school snapshot.
    pub school_names: Vec<String>,
}

/// Bump when a parser change alters what an already-journaled artifact yields: resume entries are
/// only honoured for the current version, so a format fix re-reads the affected files.
const PARSE_VERSION: u32 = 6;

#[derive(Debug, Default)]
struct Accumulator {
    meets: HashMap<String, CanonicalMeet>,
    events: HashMap<String, CanonicalEvent>,
    teams: HashMap<String, CanonicalTeam>,
    athletes: HashMap<String, CanonicalAthlete>,
    performances: HashMap<String, CanonicalPerformance>,
}

/// Run counters. Every field saturates at `usize::MAX` instead of wrapping: the counts are published
/// in the run notes, and a wrap would silently turn a large run into a small number there.
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
