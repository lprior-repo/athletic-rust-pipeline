//! The JSON projection: the rollup passes that turn the store's merged entity tables into one
//! `Census` document.

use super::notes::{bump, census_notes, state_entry};
use super::rows::{
    coach_sport, school_coach_index, school_state_index, state_of, tally_co2027, AthleteRollup,
    CoachRollup, RowCounts,
};
use super::tables::{duplicate_school_names, meet_coverage, schools_by_state, totals_of};
use super::{
    retain_core, Census, ProviderCoverage, ReportResult, Scope, StateCensus, UNKNOWN_JURISDICTION,
};
use crate::clock::{Clock, SystemClock};
use crate::store::{Store, Table};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalSchool, GradYear,
};
use census_domain::UsJurisdiction;
use std::collections::{BTreeMap, HashMap};

/// One pass over the merged athlete rows.
fn rollup_athletes(
    athletes: &[CanonicalAthlete],
    school_state: &HashMap<&str, &str>,
    school_coach: &HashMap<&str, (&CanonicalCoach, bool)>,
) -> AthleteRollup {
    let mut rollup = AthleteRollup::default();
    for athlete in athletes {
        let state = state_of(school_state, athlete.school.as_str());
        bump(
            rollup
                .by_grad_year
                .entry(athlete.grad_year.to_string())
                .or_default(),
        );
        let entry = state_entry(&mut rollup.by_state, &state);
        bump(&mut entry.athletes);
        if athlete.grad_year == GradYear::CO2027 {
            tally_co2027(entry, &mut rollup.co2027, athlete, school_coach);
        }
    }
    rollup
}

/// One pass over the merged coach rows.
fn rollup_coaches(coaches: &[CanonicalCoach], school_state: &HashMap<&str, &str>) -> CoachRollup {
    let mut rollup = CoachRollup::default();
    for coach in coaches {
        let state = state_of(school_state, coach.school.as_str());
        let slot = rollup.by_state.entry(state).or_insert((0, 0));
        slot.0 = slot.0.saturating_add(1);
        if coach.professional_email.is_some() {
            slot.1 = slot.1.saturating_add(1);
        }
        let role = format!("{:?}", coach.role).to_lowercase();
        bump(rollup.roles.entry(role).or_default());
        bump(rollup.sports.entry(coach_sport(coach)).or_default());
        for identity in &coach.source_identities {
            bump(
                rollup
                    .sources
                    .entry(identity.namespace.to_string())
                    .or_default(),
            );
        }
    }
    rollup
}

/// Merge per-state coach counts into the athlete-derived buckets, creating a state that only a coach
/// mentions.
fn apply_coach_states(
    by_state: &mut BTreeMap<String, StateCensus>,
    coach_states: &BTreeMap<String, (usize, usize)>,
) {
    for (state, (total, with_email)) in coach_states {
        let entry = state_entry(by_state, state);
        entry.coaches = *total;
        entry.coaches_with_email = *with_email;
    }
}

/// Give every configured jurisdiction a row before any rollup touches the map.
///
/// A state that holds schools but no athletes — or nothing at all — must publish zeros rather than
/// disappear from `by_state`, the per-state CSV and the "By state" sheets: §49 reads an omitted state
/// as one nobody looked at, when the measured answer is "covered, empty". The seeded rows are the
/// same labels the rollups already mint, so a state with data overwrites its own zero row instead of
/// gaining a second one.
fn seed_states(by_state: &mut BTreeMap<String, StateCensus>) {
    for code in UsJurisdiction::ALL
        .iter()
        .copied()
        .map(UsJurisdiction::code)
    {
        state_entry(by_state, code);
    }
    state_entry(by_state, UNKNOWN_JURISDICTION);
}

/// Build the census from the store's merged entity tables.
///
/// Every row comes from [`Store::scan`], which merges the append-only observations of a table into
/// one entity per id, so the report never depends on a materialized `out/*.jsonl` export. A scan
/// bounds a table at [`crate::store::MAX_ROWS_PER_TABLE`] observations, which is the bound every
/// loop below runs under.
pub fn build_census(store: &Store, scope: Scope) -> ReportResult<Census> {
    let out = store.out_dir();
    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    let mut athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
    let mut meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    let dropped = match scope {
        Scope::AllSources => 0,
        Scope::Core => retain_core(&mut athletes).saturating_add(retain_core(&mut meets)),
    };
    let counts = RowCounts::of(&schools, &athletes, &coaches, dropped);
    let school_state = school_state_index(&schools);
    let school_coach = school_coach_index(&coaches);
    let athlete_rollup = rollup_athletes(&athletes, &school_state, &school_coach);
    let coach_rollup = rollup_coaches(&coaches, &school_state);
    let coach_sources_empty = coach_rollup.sources.is_empty();
    let mut by_state = athlete_rollup.by_state;
    // Seed before the rollups land: after this line every configured jurisdiction has a row, and
    // `totals_of` still sums exactly the values the rows ended up carrying.
    seed_states(&mut by_state);
    apply_coach_states(&mut by_state, &coach_rollup.by_state);

    // Both the workbook and the CSV read a state's school count off its `by_state` row, so it is
    // filled once here from the school table rather than re-derived by each writer.
    let school_counts = schools_by_state(&schools);
    for (state, entry) in by_state.iter_mut() {
        entry.schools = school_counts.get(state).copied().unwrap_or(0);
    }

    Ok(Census {
        generated_on: SystemClock.today(),
        store_dir: store.root().display().to_string(),
        scope: scope.as_str().to_string(),
        totals: totals_of(&by_state, &counts),
        by_state,
        athletes_by_grad_year: athlete_rollup.by_grad_year,
        class_of_2027_sports: athlete_rollup.co2027.sports,
        providers: ProviderCoverage {
            namespaces: athlete_rollup.co2027.namespaces,
            multisource_athletes: athlete_rollup.co2027.multisource,
            athletic_net_urls_known: athlete_rollup.co2027.athletic_net_urls,
            grade_evidence_sources: athlete_rollup.co2027.grade_evidence_sources,
        },
        coach_roles: coach_rollup.roles,
        coach_sports: coach_rollup.sports,
        coach_sources: coach_rollup.sources,
        meets: meet_coverage(&meets),
        duplicate_school_names: duplicate_school_names(&schools),
        notes: census_notes(scope, &counts, coach_sources_empty, &out),
    })
}
