use super::coverage::jurisdiction_of;
use super::derivation::Derivation;
use super::notes::{bump, census_notes, state_entry};
use super::rows::{
    coach_sport, school_coach_index, tally_co2027, AthleteRollup, CoachRollup, RowCounts,
};
use super::tables::{duplicate_school_names, meet_coverage, schools_by_state, totals_of};
use super::{Census, ProviderCoverage, StateCensus};
use census_domain::model::{CanonicalAthlete, CanonicalCoach, CanonicalSchool, GradYear};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

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

fn rollup_coaches(
    coaches: &[CanonicalCoach],
    school_state: &HashMap<&str, Option<UsJurisdiction>>,
) -> CoachRollup {
    let mut rollup = CoachRollup::default();
    for coach in coaches {
        let state = jurisdiction_of(school_state, coach.school.as_str());
        let slot = rollup.by_state.entry(state).or_insert((0, 0));
        slot.0 = slot.0.saturating_add(1);
        if coach.has_published_email() {
            slot.1 = slot.1.saturating_add(1);
        }
        let role = coach.role.stable_key().to_lowercase();
        bump(rollup.roles.entry(role).or_default());
        bump(rollup.sports.entry(coach_sport(coach)).or_default());
        tally_coach_sources(&mut rollup, coach);
    }
    rollup
}

fn tally_coach_sources(rollup: &mut CoachRollup, coach: &CanonicalCoach) {
    for identity in &coach.source_identities {
        bump(
            rollup
                .sources
                .entry(identity.namespace.to_string())
                .or_default(),
        );
    }
}

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

fn seed_states(by_state: &mut BTreeMap<JurisdictionBucket, StateCensus>) {
    for jurisdiction in UsJurisdiction::CENSUS_SCOPE {
        state_entry(by_state, jurisdiction.into());
    }
    state_entry(by_state, JurisdictionBucket::Unplaced);
}

#[derive(Debug, Default)]
struct OutsideRow {
    schools: usize,
    athletes: usize,
    class_of_2027: usize,
    coaches: usize,
    meets: usize,
}

fn outside_scope_notes(derivation: &Derivation<'_>) -> Vec<String> {
    let school_state = derivation.school_state();
    let mut outside: BTreeMap<JurisdictionBucket, OutsideRow> = BTreeMap::new();
    for school in derivation.outside_schools() {
        bump(&mut outside.entry(school.state.into()).or_default().schools);
    }
    tally_outside_athletes(&mut outside, derivation);
    for coach in derivation.outside_coaches() {
        let bucket = jurisdiction_of(school_state, coach.school.as_str());
        bump(&mut outside.entry(bucket).or_default().coaches);
    }
    for meet in derivation.outside_meets() {
        bump(&mut outside.entry(meet.state.into()).or_default().meets);
    }
    outside
        .into_iter()
        .map(|(bucket, row)| outside_scope_note(bucket, &row))
        .collect()
}

fn tally_outside_athletes(
    outside: &mut BTreeMap<JurisdictionBucket, OutsideRow>,
    derivation: &Derivation<'_>,
) {
    for athlete in derivation.outside_athletes() {
        let row = outside
            .entry(jurisdiction_of(
                derivation.school_state(),
                athlete.school.as_str(),
            ))
            .or_default();
        bump(&mut row.athletes);
        if athlete.grad_year == GradYear::CO2027 {
            bump(&mut row.class_of_2027);
        }
    }
}

fn outside_scope_note(bucket: JurisdictionBucket, row: &OutsideRow) -> String {
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

fn fill_school_counts(
    by_state: &mut BTreeMap<JurisdictionBucket, StateCensus>,
    schools: &[CanonicalSchool],
) {
    let counts = schools_by_state(schools);
    for (state, entry) in by_state.iter_mut() {
        entry.schools = counts.get(state).copied().map_or(0, std::convert::identity);
    }
}

struct CensusRows {
    counts: RowCounts,
    athletes: AthleteRollup,
    coaches: CoachRollup,
    notes: Vec<String>,
}

pub fn build_census(derivation: &Derivation<'_>, out: &Path) -> Census {
    let schools = derivation.schools();
    let athletes = derivation.athletes();
    let coaches = derivation.coaches();
    let school_state = derivation.school_state();
    let counts = RowCounts::of(schools, athletes, coaches, derivation.dropped_rows());
    let school_coach = school_coach_index(coaches);
    let athlete_rollup = rollup_athletes(athletes, school_state, &school_coach);
    let coach_rollup = rollup_coaches(coaches, school_state);
    assemble_census(derivation, out, counts, athlete_rollup, coach_rollup)
}

fn assemble_census(
    derivation: &Derivation<'_>,
    out: &Path,
    counts: RowCounts,
    athlete_rollup: AthleteRollup,
    coach_rollup: CoachRollup,
) -> Census {
    let mut rows = CensusRows {
        counts,
        athletes: athlete_rollup,
        coaches: coach_rollup,
        notes: Vec::new(),
    };
    fill_states(&mut rows, derivation);
    rows.notes = census_row_notes(&rows, derivation, out);
    census_value(derivation, rows)
}

fn census_value(derivation: &Derivation<'_>, rows: CensusRows) -> Census {
    let (sports, providers) = cohort_parts(rows.athletes.co2027);
    Census {
        generated_on: derivation.dataset().lineage.generated_on.clone(),
        store_dir: derivation.dataset().lineage.store_root.clone(),
        scope: derivation.scope().as_str().to_string(),
        totals: totals_of(&rows.athletes.by_state, &rows.counts),
        by_state: rows.athletes.by_state,
        athletes_by_grad_year: rows.athletes.by_grad_year,
        class_of_2027_sports: sports,
        providers,
        coach_roles: rows.coaches.roles,
        coach_sports: rows.coaches.sports,
        coach_sources: rows.coaches.sources,
        meets: meet_coverage(derivation.meets()),
        duplicate_school_names: duplicate_school_names(derivation.schools()),
        notes: rows.notes,
    }
}

fn cohort_parts(cohort: super::rows::Co2027Rollup) -> (super::SportsBreakdown, ProviderCoverage) {
    let providers = ProviderCoverage {
        namespaces: cohort.namespaces,
        multisource_athletes: cohort.multisource,
        athletic_net_urls_known: cohort.athletic_net_urls,
        grade_evidence_sources: cohort.grade_evidence_sources,
    };
    (cohort.sports, providers)
}

fn fill_states(rows: &mut CensusRows, derivation: &Derivation<'_>) {
    let by_state = &mut rows.athletes.by_state;
    seed_states(by_state);
    apply_coach_states(by_state, &rows.coaches.by_state);
    fill_school_counts(by_state, derivation.schools());
}

fn census_row_notes(rows: &CensusRows, derivation: &Derivation<'_>, out: &Path) -> Vec<String> {
    let mut notes = census_notes(
        derivation.scope(),
        &rows.counts,
        rows.coaches.sources.is_empty(),
        out,
    );
    notes.extend(outside_scope_notes(derivation));
    notes
}
