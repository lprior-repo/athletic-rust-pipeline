//! The run's writing half: what one page shape accumulates, and what it counts.

use crate::school_index::SchoolIndex;
use crate::sources::tfrrs::parse::{ListPath, TeamPath, YearToken};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, Gender, GradYear, ObservedGrade, SchoolId, SchoolYear, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub(in crate::sources::tfrrs) struct Stats {
    pub(in crate::sources::tfrrs) sections: u64,
    pub(in crate::sources::tfrrs) rows_seen: u64,
    pub(in crate::sources::tfrrs) rows_absorbed: u64,
    pub(in crate::sources::tfrrs) rows_relay: u64,
    pub(in crate::sources::tfrrs) relay_members: u64,
    pub(in crate::sources::tfrrs) rows_without_team: u64,
    pub(in crate::sources::tfrrs) rows_without_season: u64,
    pub(in crate::sources::tfrrs) rows_without_athlete: u64,
    pub(in crate::sources::tfrrs) rows_without_grade: u64,
    pub(in crate::sources::tfrrs) rows_below_high_school: u64,
    pub(in crate::sources::tfrrs) rows_without_meet: u64,
    pub(in crate::sources::tfrrs) rows_without_date: u64,
    pub(in crate::sources::tfrrs) rows_without_mark: u64,
    pub(in crate::sources::tfrrs) marks_unconverted: u64,
    pub(in crate::sources::tfrrs) marks_converted: u64,
    pub(in crate::sources::tfrrs) grades_from_filter: u64,
    pub(in crate::sources::tfrrs) grades_conflicting_filter: u64,
    pub(in crate::sources::tfrrs) genders_unknown: u64,
    pub(in crate::sources::tfrrs) events_unmapped: u64,
    pub(in crate::sources::tfrrs) rosters_seen: u64,
    pub(in crate::sources::tfrrs) roster_rows_seen: u64,
    pub(in crate::sources::tfrrs) roster_rows_absorbed: u64,
    pub(in crate::sources::tfrrs) roster_rows_without_year: u64,
    pub(in crate::sources::tfrrs) roster_rows_without_name: u64,
    pub(in crate::sources::tfrrs) rosters_without_season: u64,
    pub(in crate::sources::tfrrs) fetches_failed: u64,
    pub(in crate::sources::tfrrs) schools_resolved: u64,
    pub(in crate::sources::tfrrs) schools_minted: u64,
}

/// Everything one run accumulated, keyed by canonical id so a second view of the same page upserts
/// the same entity instead of duplicating it.
#[derive(Default)]
pub(in crate::sources::tfrrs) struct Accumulator {
    pub(in crate::sources::tfrrs) schools: HashMap<String, CanonicalSchool>,
    pub(in crate::sources::tfrrs) meets: HashMap<String, CanonicalMeet>,
    pub(in crate::sources::tfrrs) teams: HashMap<String, CanonicalTeam>,
    pub(in crate::sources::tfrrs) athletes: HashMap<String, CanonicalAthlete>,
    pub(in crate::sources::tfrrs) events: HashMap<String, CanonicalEvent>,
    pub(in crate::sources::tfrrs) performances: HashMap<String, CanonicalPerformance>,
}

/// The writing half of one collection run.
pub(in crate::sources::tfrrs) struct Absorb<'a> {
    pub(super) index: &'a SchoolIndex,
    /// Schools already resolved or minted, memoized per run: a list names its schools once per row.
    pub(super) resolved: HashMap<String, SchoolId>,
    pub(in crate::sources::tfrrs) accumulator: Accumulator,
    pub(in crate::sources::tfrrs) stats: Stats,
}

/// Where one page's observations come from.
#[derive(Clone, Copy)]
pub(in crate::sources::tfrrs) struct Page<'a> {
    pub(in crate::sources::tfrrs) source: &'a SourceRef,
    pub(in crate::sources::tfrrs) observed_on: &'a str,
    /// The state the page's own host serves: TFRRS publishes one instance per state
    /// (`indiana.tfrrs.org`, `nh.tfrrs.org`), so the host is the only jurisdiction a page
    /// states and every school it names is minted in it.
    pub(in crate::sources::tfrrs) jurisdiction: UsJurisdiction,
}

/// What every row of one list page shares.
pub(in crate::sources::tfrrs) struct ListContext<'a> {
    pub(in crate::sources::tfrrs) page: Page<'a>,
    pub(in crate::sources::tfrrs) list: &'a ListPath,
    /// The `?year=` the page was requested with, when it was.
    pub(in crate::sources::tfrrs) filter: Option<YearToken>,
}

/// What every athlete of one team page shares.
pub(in crate::sources::tfrrs) struct RosterContext<'a> {
    pub(in crate::sources::tfrrs) page: Page<'a>,
    pub(in crate::sources::tfrrs) team: &'a TeamPath,
}

/// The facts one athlete observation carries.
pub(super) struct AthleteFacts<'a> {
    pub(in crate::sources::tfrrs) school: &'a SchoolId,
    pub(in crate::sources::tfrrs) name: &'a str,
    pub(in crate::sources::tfrrs) grad_year: GradYear,
    pub(in crate::sources::tfrrs) gender: Gender,
    pub(in crate::sources::tfrrs) sport: Sport,
    pub(in crate::sources::tfrrs) tfrrs_id: Option<u64>,
    pub(in crate::sources::tfrrs) url: Option<String>,
    pub(in crate::sources::tfrrs) observed_grade: Option<ObservedGrade>,
}

/// The facts one team observation carries.
pub(super) struct TeamFacts<'a> {
    pub(in crate::sources::tfrrs) school: &'a SchoolId,
    pub(in crate::sources::tfrrs) sport: Sport,
    pub(in crate::sources::tfrrs) gender: Gender,
    pub(in crate::sources::tfrrs) school_year: SchoolYear,
    /// The team route's own slug (`Lawrence_Central`), the channel a `TfrrsTeam` identity holds.
    pub(in crate::sources::tfrrs) slug: Option<&'a str>,
    /// The team route as published.
    pub(in crate::sources::tfrrs) path: Option<&'a str>,
}

impl<'a> Absorb<'a> {
    pub(in crate::sources::tfrrs) fn new(index: &'a SchoolIndex) -> Self {
        Self {
            index,
            resolved: HashMap::new(),
            accumulator: Accumulator::default(),
            stats: Stats::default(),
        }
    }
}
