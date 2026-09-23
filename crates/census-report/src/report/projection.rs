//! The JSON projection: the rollup passes that turn the store's merged entity tables into one
//! `Census` document.
//!
//! Two filters shape every published row, and their order is the contract the rest of the report
//! reads. The **run scope** — the jurisdictions [`UsJurisdiction::CENSUS_SCOPE`] names, plus the
//! unplaced row — is applied first, to the rows as scanned: outside it, a row publishes in no row
//! and counts in no total, and is named in the notes rather than dropped. The evidence [`Scope`] is
//! applied second, to the rows the run scope kept, so `core` and `all-sources` publish exactly the
//! same jurisdictions and differ only in the evidence they admit.

use super::coverage::{jurisdiction_of, school_state_index};
use super::notes::{bump, census_notes, state_entry};
use super::rows::{
    coach_sport, school_coach_index, tally_co2027, AthleteRollup, CoachRollup, RowCounts,
};
use super::tables::{duplicate_school_names, meet_coverage, schools_by_state, totals_of};
use super::{retain_core, Census, ProviderCoverage, ReportResult, Scope, StateCensus};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalSchool, GradYear,
};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use census_store::clock::{Clock, SystemClock};
use census_store::{Store, Table};
use std::collections::{BTreeMap, HashMap};

/// One pass over the merged athlete rows.
fn rollup_athletes(
    athletes: &[CanonicalAthlete],
    school_state: &HashMap<&str, Option<UsJurisdiction>>,
    school_coach: &HashMap<&str, (&CanonicalCoach, bool)>,
) -> AthleteRollup {
    let mut rollup = AthleteRollup::default();
    for athlete in athletes {
        let state = jurisdiction_of(school_state, athlete.school.as_str());
        bump(
            rollup
                .by_grad_year
                .entry(athlete.grad_year.to_string())
                .or_default(),
        );
        let entry = state_entry(&mut rollup.by_state, state);
        bump(&mut entry.athletes);
        if athlete.grad_year == GradYear::CO2027 {
            tally_co2027(entry, &mut rollup.co2027, athlete, school_coach);
        }
    }
    rollup
}

/// One pass over the merged coach rows.
fn rollup_coaches(
    coaches: &[CanonicalCoach],
    school_state: &HashMap<&str, Option<UsJurisdiction>>,
) -> CoachRollup {
    let mut rollup = CoachRollup::default();
    for coach in coaches {
        let state = jurisdiction_of(school_state, coach.school.as_str());
        let slot = rollup.by_state.entry(state).or_insert((0, 0));
        slot.0 = slot.0.saturating_add(1);
        if coach.professional_email.is_some() {
            slot.1 = slot.1.saturating_add(1);
        }
        let role = coach.role.stable_key().to_lowercase();
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
    by_state: &mut BTreeMap<JurisdictionBucket, StateCensus>,
    coach_states: &BTreeMap<JurisdictionBucket, (usize, usize)>,
) {
    for (state, (total, with_email)) in coach_states {
        let entry = state_entry(by_state, *state);
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
fn seed_states(by_state: &mut BTreeMap<JurisdictionBucket, StateCensus>) {
    for jurisdiction in UsJurisdiction::CENSUS_SCOPE {
        state_entry(by_state, jurisdiction.into());
    }
    state_entry(by_state, JurisdictionBucket::Unplaced);
}

/// Whether a census run publishes a row for `bucket`: the jurisdictions
/// [`UsJurisdiction::CENSUS_SCOPE`] names, plus the unplaced row. The same universe the coverage
/// report publishes, off the same domain constant.
pub(crate) fn in_run_scope(bucket: JurisdictionBucket) -> bool {
    bucket
        .jurisdiction()
        .is_none_or(UsJurisdiction::is_in_census_scope)
}

/// Split one scanned table by the run scope: `rows` keeps what a run publishes, and the rows the
/// scope leaves out are returned in the order the table held them.
///
/// The excluded rows stay reachable because the run-scope split of one table is what *places* a row
/// of another: schools are split here so their ids can name a jurisdiction for the athletes, coaches
/// and meets that point at them, whether or not the school itself publishes.
pub(crate) fn exclude_out_of_scope<T>(
    rows: &mut Vec<T>,
    place: impl Fn(&T) -> JurisdictionBucket,
) -> Vec<T> {
    let (kept, excluded): (Vec<T>, Vec<T>) =
        rows.drain(..).partition(|row| in_run_scope(place(row)));
    *rows = kept;
    excluded
}

/// The counts one jurisdiction the run scope leaves out would have contributed, so a note can name
/// the work the store holds for it without that work entering a row or a total.
#[derive(Debug, Default)]
struct OutsideRow {
    schools: usize,
    athletes: usize,
    class_of_2027: usize,
    coaches: usize,
    meets: usize,
}

/// One note per jurisdiction the run scope leaves outside every published row: the row the census
/// would have published for it, named rather than silently dropped, in the field the coverage report
/// writes its own notes to.
fn outside_scope_notes(
    outside_schools: &[CanonicalSchool],
    outside_athletes: &[CanonicalAthlete],
    outside_coaches: &[CanonicalCoach],
    outside_meets: &[CanonicalMeet],
    school_state: &HashMap<&str, Option<UsJurisdiction>>,
) -> Vec<String> {
    let mut outside: BTreeMap<JurisdictionBucket, OutsideRow> = BTreeMap::new();
    for school in outside_schools {
        bump(&mut outside.entry(school.state.into()).or_default().schools);
    }
    for athlete in outside_athletes {
        let row = outside
            .entry(jurisdiction_of(school_state, athlete.school.as_str()))
            .or_default();
        bump(&mut row.athletes);
        if athlete.grad_year == GradYear::CO2027 {
            bump(&mut row.class_of_2027);
        }
    }
    for coach in outside_coaches {
        let bucket = jurisdiction_of(school_state, coach.school.as_str());
        bump(&mut outside.entry(bucket).or_default().coaches);
    }
    for meet in outside_meets {
        bump(&mut outside.entry(meet.state.into()).or_default().meets);
    }
    outside
        .into_iter()
        .map(|(bucket, row)| outside_scope_note(bucket, &row))
        .collect()
}

/// One note for one excluded jurisdiction: the counts its published row would have carried.
fn outside_scope_note(bucket: JurisdictionBucket, row: &OutsideRow) -> String {
    // Name and code, so the line reads without a lookup. Every excluded bucket is a state — the
    // unplaced row is inside the run scope — so a bare code is only the total fallback.
    let jurisdiction = bucket.jurisdiction().map_or_else(
        || bucket.code().to_string(),
        |state| format!("{} ({})", state.name(), state.code()),
    );
    format!(
        "{jurisdiction} is outside the census run scope: its stored rows publish in no row and \
         count in no total — schools={} athletes={} class_of_2027={} coaches={} meets={}",
        row.schools, row.athletes, row.class_of_2027, row.coaches, row.meets
    )
}

/// The four merged entity tables one census pass reads, in scan order — schools, athletes, coaches,
/// meets — named so a signature that hands them on says what it hands back.
type ScannedTables = (
    Vec<CanonicalSchool>,
    Vec<CanonicalAthlete>,
    Vec<CanonicalCoach>,
    Vec<CanonicalMeet>,
);

/// Scan every merged table a census reads: [`Store::scan`] merges a table's append-only observations
/// into one entity per id, so the report never needs the materialized export, and bounds the table at
/// [`census_store::MAX_ROWS_PER_TABLE`] rows, which bounds every loop over them.
fn scan_tables(store: &Store) -> ReportResult<ScannedTables> {
    Ok((
        store.scan(Table::Schools)?,
        store.scan(Table::Athletes)?,
        store.scan(Table::Coaches)?,
        store.scan(Table::Meets)?,
    ))
}

/// Apply the requested [`Scope`] to the rows the run scope kept, and report how many it dropped.
fn apply_evidence_scope(
    scope: Scope,
    athletes: &mut Vec<CanonicalAthlete>,
    meets: &mut Vec<CanonicalMeet>,
) -> usize {
    match scope {
        Scope::AllSources => 0,
        Scope::Core => retain_core(athletes).saturating_add(retain_core(meets)),
    }
}

/// Fill every published row's school count from the school table, where the workbook and the CSV
/// read it from rather than re-deriving it per writer.
fn fill_school_counts(
    by_state: &mut BTreeMap<JurisdictionBucket, StateCensus>,
    schools: &[CanonicalSchool],
) {
    let counts = schools_by_state(schools);
    for (state, entry) in by_state.iter_mut() {
        entry.schools = counts.get(state).copied().unwrap_or(0);
    }
}

/// Build the census from the store's merged entity tables.
///
/// The run scope decides which jurisdictions a run has at all, and the evidence [`Scope`] decides
/// which of their rows count; the order the two are applied in, and why, is documented at the top of
/// this module. Both filters run before any rollup, so every number below — rows, the `ALL` row the
/// seal reads, and the notes — is measured over the same cohort.
pub fn build_census(store: &Store, scope: Scope) -> ReportResult<Census> {
    let out = store.out_dir();
    let (mut schools, mut athletes, mut coaches, mut meets) = scan_tables(store)?;
    // The index spans both sides of the split: a school the run scope leaves out still places its
    // own athletes and coaches, so they are excluded with it instead of landing in the unplaced row
    // as though nothing had placed them.
    let outside_schools = exclude_out_of_scope(&mut schools, |school| school.state.into());
    let mut school_state = school_state_index(&schools);
    school_state.extend(school_state_index(&outside_schools));
    let outside_athletes = exclude_out_of_scope(&mut athletes, |athlete| {
        jurisdiction_of(&school_state, athlete.school.as_str())
    });
    let outside_coaches = exclude_out_of_scope(&mut coaches, |coach| {
        jurisdiction_of(&school_state, coach.school.as_str())
    });
    let outside_meets = exclude_out_of_scope(&mut meets, |meet| meet.state.into());
    let dropped = apply_evidence_scope(scope, &mut athletes, &mut meets);
    let counts = RowCounts::of(&schools, &athletes, &coaches, dropped);
    let school_coach = school_coach_index(&coaches);
    let athlete_rollup = rollup_athletes(&athletes, &school_state, &school_coach);
    let coach_rollup = rollup_coaches(&coaches, &school_state);
    let coach_sources_empty = coach_rollup.sources.is_empty();
    let mut by_state = athlete_rollup.by_state;
    // Seed before the rollups land: after this line every run-scope jurisdiction has a row, and
    // `totals_of` still sums exactly the values the rows ended up carrying.
    seed_states(&mut by_state);
    apply_coach_states(&mut by_state, &coach_rollup.by_state);
    fill_school_counts(&mut by_state, &schools);
    let mut notes = census_notes(scope, &counts, coach_sources_empty, &out);
    notes.extend(outside_scope_notes(
        &outside_schools,
        &outside_athletes,
        &outside_coaches,
        &outside_meets,
        &school_state,
    ));
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
        notes,
    })
}
