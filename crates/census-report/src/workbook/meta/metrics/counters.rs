//! Count helper functions used by the reconciliation block.

use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, GradYear, SourceNamespace,
};

/// Coaches carrying a published address.
pub(super) fn count_coaches_with_email(coaches: &[CanonicalCoach]) -> usize {
    coaches
        .iter()
        .filter(|coach| coach.has_published_email())
        .count()
}

/// The store's class-of-2027 athlete rows.
pub(super) fn count_co2027(athletes: &[CanonicalAthlete]) -> usize {
    athletes
        .iter()
        .filter(|athlete| athlete.grad_year == GradYear::CO2027)
        .count()
}

/// Class-of-2027 athletes carrying at least one grade observation.
pub(super) fn count_grade_evidence(athletes: &[CanonicalAthlete]) -> usize {
    athletes
        .iter()
        .filter(|athlete| {
            athlete.grad_year == GradYear::CO2027 && !athlete.observed_grades.is_empty()
        })
        .count()
}

/// Meets whose source identities include a legacy Athletic.net meet id, the predicate the census's
/// own `with_athletic_net_id` counter uses.
pub(super) fn count_athletic_net_meets(meets: &[CanonicalMeet]) -> usize {
    meets
        .iter()
        .filter(|meet| {
            meet.source_identities.iter().any(|identity| {
                matches!(
                    identity.namespace,
                    SourceNamespace::LegacyAthleticNet { .. }
                )
            })
        })
        .count()
}
