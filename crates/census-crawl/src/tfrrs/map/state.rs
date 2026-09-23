//! The run's writing half: what one page shape accumulates, and what it counts.

use crate::tfrrs::parse::{ListPath, TeamPath, YearToken};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, Gender, GradYear, ObservedGrade, SchoolId, SchoolYear, SourceRef, Sport,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub(in crate::tfrrs) struct Stats {
    pub(in crate::tfrrs) sections: u64,
    pub(in crate::tfrrs) rows_seen: u64,
    pub(in crate::tfrrs) rows_absorbed: u64,
    pub(in crate::tfrrs) rows_relay: u64,
    pub(in crate::tfrrs) relay_members: u64,
    pub(in crate::tfrrs) rows_without_team: u64,
    pub(in crate::tfrrs) rows_without_season: u64,
    pub(in crate::tfrrs) rows_without_athlete: u64,
    pub(in crate::tfrrs) rows_without_grade: u64,
    pub(in crate::tfrrs) rows_below_high_school: u64,
    pub(in crate::tfrrs) rows_without_meet: u64,
    pub(in crate::tfrrs) rows_without_date: u64,
    pub(in crate::tfrrs) rows_without_mark: u64,
    pub(in crate::tfrrs) marks_unconverted: u64,
    pub(in crate::tfrrs) marks_converted: u64,
    pub(in crate::tfrrs) grades_from_filter: u64,
    pub(in crate::tfrrs) grades_conflicting_filter: u64,
    pub(in crate::tfrrs) genders_unknown: u64,
    pub(in crate::tfrrs) events_unmapped: u64,
    pub(in crate::tfrrs) rosters_seen: u64,
    pub(in crate::tfrrs) roster_rows_seen: u64,
    pub(in crate::tfrrs) roster_rows_absorbed: u64,
    pub(in crate::tfrrs) roster_rows_without_year: u64,
    pub(in crate::tfrrs) roster_rows_without_name: u64,
    pub(in crate::tfrrs) rosters_without_season: u64,
    pub(in crate::tfrrs) fetches_failed: u64,
    pub(in crate::tfrrs) schools_resolved: u64,
    pub(in crate::tfrrs) schools_minted: u64,
}

/// Everything one run accumulated, keyed by canonical id so a second view of the same page upserts
/// the same entity instead of duplicating it.
#[derive(Default)]
pub(in crate::tfrrs) struct Accumulator {
    pub(in crate::tfrrs) schools: HashMap<String, CanonicalSchool>,
    pub(in crate::tfrrs) meets: HashMap<String, CanonicalMeet>,
    pub(in crate::tfrrs) teams: HashMap<String, CanonicalTeam>,
    pub(in crate::tfrrs) athletes: HashMap<String, CanonicalAthlete>,
    pub(in crate::tfrrs) events: HashMap<String, CanonicalEvent>,
    pub(in crate::tfrrs) performances: HashMap<String, CanonicalPerformance>,
}

/// The writing half of one collection run.
pub(in crate::tfrrs) struct Absorb<'a> {
    pub(super) index: &'a SchoolIndex,
    /// Schools already resolved or minted, memoized per run: a list names its schools once per row.
    pub(super) resolved: HashMap<String, SchoolId>,
    pub(in crate::tfrrs) accumulator: Accumulator,
    pub(in crate::tfrrs) stats: Stats,
}

/// Where one page's observations come from.
#[derive(Clone, Copy)]
pub(in crate::tfrrs) struct Page<'a> {
    pub(in crate::tfrrs) source: &'a SourceRef,
    pub(in crate::tfrrs) observed_on: &'a str,
    /// The state the page's own host serves: TFRRS publishes one instance per state
    /// (`indiana.tfrrs.org`, `nh.tfrrs.org`), so the host is the only jurisdiction a page
    /// states and every school it names is minted in it.
    pub(in crate::tfrrs) jurisdiction: UsJurisdiction,
}

/// What every row of one list page shares.
pub(in crate::tfrrs) struct ListContext<'a> {
    pub(in crate::tfrrs) page: Page<'a>,
    pub(in crate::tfrrs) list: &'a ListPath,
    /// The `?year=` the page was requested with, when it was.
    pub(in crate::tfrrs) filter: Option<YearToken>,
}

/// What every athlete of one team page shares.
pub(in crate::tfrrs) struct RosterContext<'a> {
    pub(in crate::tfrrs) page: Page<'a>,
    pub(in crate::tfrrs) team: &'a TeamPath,
}

/// The facts one athlete observation carries.
pub(super) struct AthleteFacts<'a> {
    pub(in crate::tfrrs) school: &'a SchoolId,
    pub(in crate::tfrrs) name: &'a str,
    pub(in crate::tfrrs) grad_year: GradYear,
    pub(in crate::tfrrs) gender: Gender,
    pub(in crate::tfrrs) sport: Sport,
    pub(in crate::tfrrs) tfrrs_id: Option<u64>,
    pub(in crate::tfrrs) url: Option<String>,
    pub(in crate::tfrrs) observed_grade: Option<ObservedGrade>,
}

/// The facts one team observation carries.
pub(super) struct TeamFacts<'a> {
    pub(in crate::tfrrs) school: &'a SchoolId,
    pub(in crate::tfrrs) sport: Sport,
    pub(in crate::tfrrs) gender: Gender,
    pub(in crate::tfrrs) school_year: SchoolYear,
    /// The team route's own slug (`Lawrence_Central`), the channel a `TfrrsTeam` identity holds.
    pub(in crate::tfrrs) slug: Option<&'a str>,
    /// The team route as published.
    pub(in crate::tfrrs) path: Option<&'a str>,
}

impl<'a> Absorb<'a> {
    pub(in crate::tfrrs) fn new(index: &'a SchoolIndex) -> Self {
        Self {
            index,
            resolved: HashMap::new(),
            accumulator: Accumulator::default(),
            stats: Stats::default(),
        }
    }
}
