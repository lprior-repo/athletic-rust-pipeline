use super::checks::visible;
use super::{
    CandidateQuery, CohortFact, InSeasonEvidence, ParticipationInputError,
    ParticipationObservation, ParticipationReason, ParticipationReasons, ProofReference,
};
use crate::model::{ContractRefV1, ProfileDate, ProfileError};

pub const PARTICIPATION_POLICY_V4_DOCUMENT: &str = r#"{"contract":"high_school_participation","revision":4,"rules":["preserve_v3_ownership_cohort_context_status_date_profile_and_frozen_visibility_checks","admit_positive_high_school_status_only_from_dated_school_roster_or_explicit_student_status","retain_negative_status_evidence_and_all_applicable_full_window_boundaries_and_conflicts","when_no_applicable_full_window_admit_only_a_visible_source_proven_in_season_interval_containing_the_exact_performance_date","require_checked_issuer_edition_school_cohort_sport_competition_and_date_applicability","retain_conflicting_in_season_evidence_as_unresolved","positive_source_proven_event_days_union_without_inferred_tenure","withhold_unresolved_summer_marks_without_a_documented_applicable_window","never_infer_status_or_summer_endpoint_from_division_cohort_or_profile_window"],"version":1}"#;
pub const PARTICIPATION_POLICY_V4_SHA256: &str =
    "d5e0194f00a120acb28bbba567e4f491871207f92a626c8140366e98e57b418a";

pub fn participation_policy_v4() -> Result<ContractRefV1, ProfileError> {
    ContractRefV1::new(1, 4, PARTICIPATION_POLICY_V4_SHA256)
}

pub(super) fn fact_count(season: InSeasonEvidence<'_>) -> usize {
    let (supported, contrary) = facts(season);
    supported.len().saturating_add(contrary.len())
}

fn facts<'a>(
    season: InSeasonEvidence<'a>,
) -> (
    &'a [super::InSeasonWindowFact<'a>],
    &'a [super::InSeasonWindowFact<'a>],
) {
    match season {
        InSeasonEvidence::Unavailable => (&[], &[]),
        InSeasonEvidence::Uncontested(facts) => (facts, &[]),
        InSeasonEvidence::Conflicted {
            supported,
            contrary,
        } => (supported, contrary),
    }
}

pub(super) fn proofs(season: InSeasonEvidence<'_>) -> impl Iterator<Item = ProofReference<'_>> {
    let (supported, contrary) = facts(season);
    supported
        .iter()
        .chain(contrary)
        .map(|fact| fact.boundary_proof)
}

pub(super) fn admitted(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'_>,
    date: &ProfileDate,
    season: InSeasonEvidence<'_>,
    reasons: &mut ParticipationReasons,
) -> Result<bool, ParticipationInputError> {
    let (supported, contrary) = facts(season);
    let positive = applicable(query, observation, date, supported)?;
    let negative = applicable(query, observation, date, contrary)?;
    if positive && negative {
        reasons.add(ParticipationReason::ParticipationWindowConflict);
    }
    Ok(positive && !negative)
}

fn applicable(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'_>,
    date: &ProfileDate,
    facts: &[super::InSeasonWindowFact<'_>],
) -> Result<bool, ParticipationInputError> {
    facts.iter().try_fold(false, |found, fact| {
        let cohort_matches = match observation.cohort {
            CohortFact::Resolved { year, .. } => year == fact.identity.cohort,
            _ => false,
        };
        Ok(found
            || (visible(query, fact.boundary_proof)?
                && fact.identity.participation_rule == query.binding.participation_rule
                && cohort_matches
                && query
                    .profile
                    .input()
                    .graduation_years
                    .contains(&fact.identity.cohort)
                && fact.interval.contains(date)))
    })
}

pub(super) fn check_facts(season: InSeasonEvidence<'_>) -> Result<(), ParticipationInputError> {
    let (supported, contrary) = facts(season);
    if let InSeasonEvidence::Conflicted { .. } = season {
        if supported.is_empty() || contrary.is_empty() {
            return Err(ParticipationInputError::EmptyInSeasonConflict);
        }
        if supported.iter().any(|positive| {
            contrary
                .iter()
                .any(|negative| positive.boundary_proof.fact == negative.boundary_proof.fact)
        }) {
            return Err(ParticipationInputError::ConflictingProofIdentity);
        }
    }
    supported.iter().chain(contrary).try_for_each(|fact| {
        supported.iter().chain(contrary).try_for_each(|other| {
            if fact.boundary_proof.fact == other.boundary_proof.fact && fact != other {
                Err(ParticipationInputError::ConflictingProofIdentity)
            } else {
                Ok(())
            }
        })
    })
}
