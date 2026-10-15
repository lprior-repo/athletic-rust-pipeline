use super::checks::visible;
use super::{
    CandidateQuery, ParticipationContext, ParticipationInputError, ParticipationObservation,
    ParticipationReason, ParticipationReasons, StatusBasis, StudentStatus,
};
use crate::model::{ContractRefV1, ProfileDate, ProfileError};

pub const PARTICIPATION_POLICY_V2_DOCUMENT: &str = concat!(
    "{\"contract\":\"high_school_participation\",\"revision\":2,\"rules\":[",
    "\"preserve_existing_ownership_cohort_context_status_date_profile_and_frozen_visibility_checks\",",
    "\"require_documented_summer_boundary_by_default\",",
    "\"allow_visible_explicit_scholastic_high_school_status_containing_exact_performance_date_",
    "with_exclusively_visible_scholastic_context_when_no_applicable_window\",",
    "\"retain_applicable_window_boundaries_and_conflicts\",",
    "\"club_open_unattached_require_documented_window\",",
    "\"never_infer_summer_boundary_from_graduation_year\"],\"version\":1}"
);

pub const PARTICIPATION_POLICY_V2_SHA256: &str =
    "fde4b6b731e1de8bd8dec3ec24ab881712bcee1d3522c31ae7d6a61d8c5fd8ee";

pub fn participation_policy_v2() -> Result<ContractRefV1, ProfileError> {
    ContractRefV1::new(1, 2, PARTICIPATION_POLICY_V2_SHA256)
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ParticipationPolicy {
    HistoricalExplicitWindow,
    DateLocalScholastic,
}

impl ParticipationPolicy {
    pub(super) fn resolve(reference: &ContractRefV1) -> Result<Self, ParticipationInputError> {
        if reference.revision().get() != 2 || reference.digest() != PARTICIPATION_POLICY_V2_SHA256 {
            return Err(ParticipationInputError::UnsupportedPolicy);
        }
        Ok(Self::DateLocalScholastic)
    }

    pub(super) fn establishes_date_local_status(
        self,
        query: CandidateQuery<'_>,
        observation: &ParticipationObservation<'_>,
        date: &ProfileDate,
        reasons: ParticipationReasons,
    ) -> Result<bool, ParticipationInputError> {
        match self {
            Self::HistoricalExplicitWindow => Ok(false),
            Self::DateLocalScholastic => Ok(!reasons
                .contains(ParticipationReason::ParticipationEvidenceConflict)
                && !reasons.contains(ParticipationReason::CollegiateParticipation)
                && !reasons.contains(ParticipationReason::MiddleSchoolParticipation)
                && !reasons.contains(ParticipationReason::PostGraduationParticipation)
                && exclusively_scholastic(query, observation)?
                && explicit_scholastic_status(query, observation, date)?),
        }
    }
}

fn exclusively_scholastic(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'_>,
) -> Result<bool, ParticipationInputError> {
    let classes = observation
        .contexts
        .iter()
        .try_fold(0_u8, |classes, fact| {
            if !visible(query, fact.proof)? {
                return Ok(classes);
            }
            let class = if fact.context == ParticipationContext::Scholastic {
                1
            } else {
                2
            };
            Ok::<_, ParticipationInputError>(classes | class)
        })?;
    Ok(classes == 1)
}

fn explicit_scholastic_status(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'_>,
    date: &ProfileDate,
) -> Result<bool, ParticipationInputError> {
    observation
        .high_school_status
        .iter()
        .try_fold(false, |found, fact| {
            Ok(found
                || (fact.status == StudentStatus::HighSchool
                    && fact.basis == StatusBasis::ExplicitScholasticResult
                    && fact.interval.contains(date)
                    && visible(query, fact.proof)?))
        })
}
