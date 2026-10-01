use crate::export::athletic_net_meet_identity;
use census_domain::model::{CanonicalAthlete, CanonicalCoach, CanonicalMeet, Confidence, GradYear};

pub(super) fn count_coaches_with_email(coaches: &[CanonicalCoach]) -> usize {
    coaches
        .iter()
        .filter(|coach| coach.has_published_email())
        .count()
}

pub(super) fn count_co2027(athletes: &[CanonicalAthlete]) -> usize {
    athletes
        .iter()
        .filter(|athlete| athlete.grad_year == GradYear::CO2027)
        .count()
}

pub(super) fn count_grade_evidence(athletes: &[CanonicalAthlete]) -> usize {
    athletes
        .iter()
        .filter(|athlete| {
            athlete.grad_year == GradYear::CO2027
                && athlete.derived_cohort_confidence() == Some(Confidence::HIGH)
        })
        .count()
}

pub(super) fn count_athletic_net_meets(meets: &[CanonicalMeet]) -> usize {
    meets
        .iter()
        .filter(|meet| athletic_net_meet_identity(meet).is_some())
        .count()
}
