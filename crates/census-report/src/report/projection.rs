use super::coverage::jurisdiction_of;
use super::derivation::Derivation;
use super::notes::{bump, census_notes, state_entry};
use super::rows::{
    coach_sport, school_coach_index, tally_co2027, AthleteRollup, CoachRollup, RowCounts,
};
use super::tables::{duplicate_school_names, meet_coverage, schools_by_state, totals_of};
use super::{Census, ProviderCoverage, StateCensus};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalSchool, GradYear,
};
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

pub fn build_census(derivation: &Derivation<'_>, out: &Path) -> Census {
    let schools = derivation.schools();
    let athletes = derivation.athletes();
    let coaches = derivation.coaches();
    let school_state = derivation.school_state();
    let counts = RowCounts::of(schools, athletes, coaches, derivation.dropped_rows());
    let school_coach = school_coach_index(coaches);
    let athlete_rollup = rollup_athletes(athletes, school_state, &school_coach);
    let coach_rollup = rollup_coaches(coaches, school_state);
    let coach_sources_empty = coach_rollup.sources.is_empty();
    let mut by_state = athlete_rollup.by_state;
    seed_states(&mut by_state);
    apply_coach_states(&mut by_state, &coach_rollup.by_state);
    fill_school_counts(&mut by_state, schools);
    let mut notes = census_notes(derivation.scope(), &counts, coach_sources_empty, out);
    notes.extend(outside_scope_notes(
        derivation.outside_schools(),
        derivation.outside_athletes(),
        derivation.outside_coaches(),
        derivation.outside_meets(),
        school_state,
    ));
    Census {
        generated_on: derivation.dataset().lineage.generated_on.clone(),
        store_dir: derivation.dataset().lineage.store_root.clone(),
        scope: derivation.scope().as_str().to_string(),
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
        meets: meet_coverage(derivation.meets()),
        duplicate_school_names: duplicate_school_names(schools),
        notes,
    }
}
