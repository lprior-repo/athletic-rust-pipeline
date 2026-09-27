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

#[derive(Default)]
pub(in crate::tfrrs) struct Accumulator {
    pub(in crate::tfrrs) schools: HashMap<String, CanonicalSchool>,
    pub(in crate::tfrrs) meets: HashMap<String, CanonicalMeet>,
    pub(in crate::tfrrs) teams: HashMap<String, CanonicalTeam>,
    pub(in crate::tfrrs) athletes: HashMap<String, CanonicalAthlete>,
    pub(in crate::tfrrs) events: HashMap<String, CanonicalEvent>,
    pub(in crate::tfrrs) performances: HashMap<String, CanonicalPerformance>,
}

pub(in crate::tfrrs) struct Absorb<'a> {
    pub(super) index: &'a SchoolIndex,
    pub(super) resolved: HashMap<String, SchoolId>,
    pub(in crate::tfrrs) accumulator: Accumulator,
    pub(in crate::tfrrs) stats: Stats,
}

#[derive(Clone, Copy)]
pub(in crate::tfrrs) struct Page<'a> {
    pub(in crate::tfrrs) source: &'a SourceRef,
    pub(in crate::tfrrs) observed_on: &'a str,
    pub(in crate::tfrrs) jurisdiction: UsJurisdiction,
}

pub(in crate::tfrrs) struct ListContext<'a> {
    pub(in crate::tfrrs) page: Page<'a>,
    pub(in crate::tfrrs) list: &'a ListPath,
    pub(in crate::tfrrs) filter: Option<YearToken>,
}

pub(in crate::tfrrs) struct RosterContext<'a> {
    pub(in crate::tfrrs) page: Page<'a>,
    pub(in crate::tfrrs) team: &'a TeamPath,
}

pub(super) struct AthleteFacts<'a> {
    pub(in crate::tfrrs) school: &'a SchoolId,
    pub(in crate::tfrrs) name: &'a str,
    pub(in crate::tfrrs) grad_year: GradYear,
    pub(in crate::tfrrs) gender: Gender,
    pub(in crate::tfrrs) sport: Sport,
    pub(in crate::tfrrs) tfrrs_id: Option<u64>,
    pub(in crate::tfrrs) url: Option<String>,
    pub(in crate::tfrrs) observed_grade: Option<ObservedGrade>,
    pub(in crate::tfrrs) source_key: String,
}

pub(super) struct TeamFacts<'a> {
    pub(in crate::tfrrs) school: &'a SchoolId,
    pub(in crate::tfrrs) sport: Sport,
    pub(in crate::tfrrs) gender: Gender,
    pub(in crate::tfrrs) school_year: SchoolYear,
    pub(in crate::tfrrs) slug: Option<&'a str>,
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
