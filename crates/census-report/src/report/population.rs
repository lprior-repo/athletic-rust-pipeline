use census_domain::model::performance_comparison::SurfaceClass;
use census_domain::model::{CanonicalAthlete, CanonicalMeet, Confidence, QueryProfileV1, Sport};
use census_domain::UsJurisdiction;
use std::borrow::Cow;
use std::collections::HashMap;

pub(crate) fn accepted_members<'a>(
    athletes: &'a [CanonicalAthlete],
    aliases: &HashMap<String, String>,
) -> Cow<'a, [CanonicalAthlete]> {
    super::derivation::borrowed_or_collapsed(athletes, aliases)
}

pub(crate) fn cohort(athlete: &CanonicalAthlete, profile: &QueryProfileV1) -> bool {
    athlete.derived_cohort_confidence() == Some(Confidence::HIGH)
        && profile
            .input()
            .graduation_years
            .contains(&athlete.grad_year)
        && profile
            .input()
            .competition_categories
            .contains(&athlete.gender)
}

pub(crate) fn roster(
    athlete: &CanonicalAthlete,
    profile: &QueryProfileV1,
    school_state: Option<UsJurisdiction>,
) -> bool {
    athlete.identities().next().is_some()
        && athlete
            .sports
            .iter()
            .any(|sport| profile.input().sports.contains(sport))
        && school_state.is_some_and(|state| profile.admits_jurisdiction(state))
}

pub(crate) fn meet_sport(meet: &CanonicalMeet) -> Sport {
    match SurfaceClass::resolve(meet) {
        SurfaceClass::Indoor => Sport::IndoorTrack,
        SurfaceClass::Outdoor => Sport::OutdoorTrack,
        SurfaceClass::CrossCountry => Sport::CrossCountry,
        SurfaceClass::Unresolved => Sport::Unknown,
    }
}
