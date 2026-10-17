use super::{
    defect, ParticipationAssessment, ParticipationDiagnostic, ParticipationExclusion,
    ParticipationExclusionOutcome, ParticipationExclusionReason, ReportResult,
    VerifiedParticipationView,
};
use census_domain::participation_candidates::CandidateOutcome;

pub(super) fn assessment<'a>(
    value: &ParticipationExclusion,
    participation: &'a VerifiedParticipationView,
) -> ReportResult<&'a ParticipationAssessment> {
    let row = participation
        .documentary(value.performance_id())
        .ok_or_else(|| defect("documentary exclusion has no verified source observation"))?;
    if participation.assessment(value.performance_id()).is_some()
        || row.performance_id() != value.performance_id()
        || row.event_id() != value.event_id()
        || row.athlete_id() != value.athlete_id()
    {
        return Err(defect(
            "documentary exclusion identity conflicts with its observation",
        ));
    }
    let (outcome, tag) = match row.domain_outcome() {
        CandidateOutcome::EligibleCandidate => (
            ParticipationExclusionOutcome::Unresolved,
            ParticipationDiagnostic::DocumentaryAssessmentEligible,
        ),
        CandidateOutcome::Excluded => (
            ParticipationExclusionOutcome::Excluded,
            ParticipationDiagnostic::DocumentaryAssessmentExcluded,
        ),
        CandidateOutcome::Unresolved => (
            ParticipationExclusionOutcome::Unresolved,
            ParticipationDiagnostic::DocumentaryAssessmentUnresolved,
        ),
    };
    let assessment = row.assessment();
    let mapped = match row.domain_outcome() {
        CandidateOutcome::EligibleCandidate => crate::export::ParticipationOutcome::Eligible,
        CandidateOutcome::Excluded => crate::export::ParticipationOutcome::Excluded,
        CandidateOutcome::Unresolved => crate::export::ParticipationOutcome::Unresolved,
    };
    let reason = ParticipationExclusionReason::DocumentarySchoolBindingUnresolved {
        assessment: row.domain_outcome(),
        reasons: assessment.reasons(),
    };
    let valid_reasons = match row.domain_outcome() {
        CandidateOutcome::EligibleCandidate => assessment.reasons().is_empty(),
        CandidateOutcome::Excluded | CandidateOutcome::Unresolved => {
            !assessment.reasons().is_empty()
        }
    };
    if value.reason() != reason
        || value.outcome() != outcome
        || assessment.outcome() != mapped
        || !valid_reasons
        || assessment.diagnostics() != [tag, ParticipationDiagnostic::SchoolBindingUnresolved]
    {
        return Err(defect(
            "documentary exclusion contradicts its actual assessment or school gap",
        ));
    }
    Ok(assessment)
}

pub(super) fn require_complete(
    participation: &VerifiedParticipationView,
    outcome: &super::SelectionOutcome,
    budget: &mut super::Admission,
) -> ReportResult<()> {
    for row in participation.documentary_rows() {
        let mut found = None;
        for value in &outcome.participation_exclusions {
            budget.charge(0, 1)?;
            if value.performance_id() == row.performance_id() {
                found = Some(value);
                break;
            }
        }
        let value = found
            .ok_or_else(|| defect("participation exclusions omit a documentary observation"))?;
        assessment(value, participation)?;
    }
    Ok(())
}
