use super::*;
use census_domain::model::QueryProfileV1;

pub(super) fn performances<'a>(
    dataset: &'a ExportDataset,
    derivation: &Derivation<'_>,
    admission: &EventAdmission<'_>,
    profile: &QueryProfileV1,
    parents: &Parents<'a>,
) -> Vec<&'a CanonicalPerformance> {
    dataset
        .performances
        .iter()
        .filter(|performance| {
            parents
                .athlete(performance.athlete.as_str())
                .is_some_and(|athlete| {
                    in_cohort(athlete, derivation, profile)
                        && admission.event(&performance.event).is_some_and(|event| {
                            profile
                                .input()
                                .competition_categories
                                .contains(&event.gender)
                        })
                        && admission
                            .sport(&performance.event)
                            .is_some_and(|sport| profile.input().sports.contains(&sport))
                        && dataset
                            .teams
                            .get(&performance.team)
                            .and_then(|team| dataset.schools.get(&team.school))
                            .and_then(|school| school.state)
                            .is_some_and(|state| in_state(state, derivation, profile))
                })
        })
        .collect()
}

pub(super) fn athletes<'a>(
    dataset: &'a ExportDataset,
    derivation: &Derivation<'_>,
    profile: &QueryProfileV1,
    performances: &HashMap<&str, Vec<&CanonicalPerformance>>,
) -> Vec<&'a CanonicalAthlete> {
    let verified = derivation
        .athletes()
        .iter()
        .map(|athlete| &athlete.id)
        .collect::<BTreeSet<_>>();
    dataset
        .athletes
        .iter()
        .filter(|athlete| {
            if !in_cohort(athlete, derivation, profile) || verified.contains(&athlete.id) {
                return false;
            }
            let has_performance = performances.contains_key(athlete.id.as_str());
            let roster = athlete.identities().next().is_some()
                && athlete
                    .sports
                    .iter()
                    .any(|sport| profile.input().sports.contains(sport))
                && dataset
                    .schools
                    .get(&athlete.school)
                    .and_then(|school| school.state)
                    .is_some_and(|state| in_state(state, derivation, profile));
            roster || has_performance
        })
        .collect()
}

fn in_cohort(
    athlete: &CanonicalAthlete,
    derivation: &Derivation<'_>,
    profile: &QueryProfileV1,
) -> bool {
    crate::report::in_requested_year(athlete, derivation.grad_year())
        && athlete.derived_cohort_confidence() == Some(census_domain::model::Confidence::HIGH)
        && profile
            .input()
            .graduation_years
            .contains(&athlete.grad_year)
        && profile
            .input()
            .competition_categories
            .contains(&athlete.gender)
}

fn in_state(
    state: census_domain::UsJurisdiction,
    derivation: &Derivation<'_>,
    profile: &QueryProfileV1,
) -> bool {
    profile.admits_jurisdiction(state)
        && (derivation.jurisdictions().is_empty() || derivation.jurisdictions().contains(&state))
}
