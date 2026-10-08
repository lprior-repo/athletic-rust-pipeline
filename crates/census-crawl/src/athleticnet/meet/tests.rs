use super::super::map::{Accumulator, Stats};
use super::super::parse::gender_of;
use super::super::{
    grade_of, jurisdiction_of, AllResults, EventDivisions, EventMetadata, MeetData,
};
use super::map::absorb_meet;
use census_domain::model::{
    CompetitionLevel, EvidenceMethod, Gender, Mark, PerformanceId, SchoolYear, SourceNamespace,
    SourceRef, Sport, TimingMethod,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use std::collections::{BTreeMap, BTreeSet, HashMap};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod captured;
mod performance_specifications;
mod qualification;
mod semantics;
mod specifications;

const MEET_DATA: &str =
    include_str!("../../../tests/fixtures/athleticnet/meet_634313_meetdata.json");
const ALL_RESULTS: &str =
    include_str!("../../../tests/fixtures/athleticnet/meet_634313_allresults.json");
const EVENT_DIV: &str =
    include_str!("../../../tests/fixtures/athleticnet/meet_634313_eventdiv.json");
const MEET_DATA_WITHOUT_STATE: &str =
    include_str!("../../../tests/fixtures/athleticnet/meet_634313_meetdata_nostate.derived.json");
const OBSERVED_ON: &str = "2026-09-22";

struct Walk {
    meet: MeetData,
    counts: super::count::MeetStats,
    accumulated: Accumulator,
    stats: Stats,
}

fn walk(metadata: bool) -> TestResult<Walk> {
    let meet: MeetData = serde_json::from_str(MEET_DATA)?;
    let results: AllResults = serde_json::from_str(ALL_RESULTS)?;
    let document: EventDivisions = serde_json::from_str(EVENT_DIV)?;
    let metadata = metadata.then(|| EventMetadata::new(&document));
    absorb(&meet, &results, metadata.as_ref())
}

fn absorb(
    meet: &MeetData,
    results: &AllResults,
    metadata: Option<&EventMetadata>,
) -> TestResult<Walk> {
    let mut accumulated = Accumulator::default();
    let mut stats = Stats::default();
    let source = SourceRef::new("athleticnet", None);
    let (_, counts) = absorb_meet(
        meet,
        results,
        metadata,
        &source,
        (
            OBSERVED_ON,
            chrono::NaiveDate::from_ymd_opt(2026, 9, 30).ok_or("snapshot date")?,
        ),
        &SchoolIndex::from_schools(&[]),
        &mut HashMap::new(),
        &mut stats,
        &mut accumulated,
    )?;
    Ok(Walk {
        meet: meet.clone(),
        counts,
        accumulated,
        stats,
    })
}

fn performances(walk: &Walk) -> Vec<&census_domain::model::CanonicalPerformance> {
    let mut rows: Vec<_> = walk.accumulated.performances.values().collect();
    rows.sort_by(|left, right| left.source_key.cmp(&right.source_key));
    rows
}

fn leg_position(row: &census_domain::model::CanonicalPerformance) -> Option<u8> {
    let note = row.evidence.first()?.note.as_deref()?;
    note.strip_prefix("relay leg ")?
        .split(';')
        .next()?
        .trim()
        .parse()
        .ok()
}
