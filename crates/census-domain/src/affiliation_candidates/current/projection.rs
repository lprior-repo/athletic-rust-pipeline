use super::{
    CandidateSelection, CurrentAffiliationError, CurrentAffiliationOutcome, FactDisposition,
};
use crate::affiliation_candidates::{CandidateOutcome, Disposition, RelationshipCandidate};

pub(super) fn outcome<'a>(
    selection: &CandidateSelection<'a>,
) -> Result<CurrentAffiliationOutcome<'a>, CurrentAffiliationError> {
    Ok(match selection.outcome {
        CandidateOutcome::SingleCandidate(fact) => match fact.raw().relationship {
            RelationshipCandidate::NonSchool(kind) => CurrentAffiliationOutcome::NonSchool(kind),
            relationship => CurrentAffiliationOutcome::ResolvedSchoolCandidate(
                super::super::relationship::school(relationship)
                    .ok_or(CurrentAffiliationError::InconsistentCandidateReduction)?,
            ),
        },
        CandidateOutcome::MultipleCompatibleCandidates => compatible_school(selection)?,
        CandidateOutcome::Conflicted => CurrentAffiliationOutcome::Conflicted,
        CandidateOutcome::SubjectUnresolved => CurrentAffiliationOutcome::SubjectUnresolved,
        CandidateOutcome::EndedWithoutReplacement => {
            CurrentAffiliationOutcome::EndedWithoutReplacement
        }
        CandidateOutcome::Unknown if unresolved_coop(selection) => {
            CurrentAffiliationOutcome::CoopCampusUnresolved
        }
        CandidateOutcome::Unknown => CurrentAffiliationOutcome::Unknown,
    })
}

fn compatible_school<'a>(
    selection: &CandidateSelection<'a>,
) -> Result<CurrentAffiliationOutcome<'a>, CurrentAffiliationError> {
    let school = selection
        .facts
        .iter()
        .try_fold(None, |school, disposition| {
            let FactDisposition::Assertion {
                fact,
                disposition: Disposition::Candidate,
            } = disposition
            else {
                return Ok(school);
            };
            let target = super::super::relationship::school(fact.raw().relationship)
                .ok_or(CurrentAffiliationError::InconsistentCandidateReduction)?;
            if school.is_some_and(|prior| prior != target) {
                return Err(CurrentAffiliationError::InconsistentCandidateReduction);
            }
            Ok(Some(target))
        })?;
    school
        .map(CurrentAffiliationOutcome::ResolvedSchoolCandidate)
        .ok_or(CurrentAffiliationError::InconsistentCandidateReduction)
}

fn unresolved_coop(selection: &CandidateSelection<'_>) -> bool {
    selection.facts.iter().any(|fact| {
        matches!(
            fact,
            FactDisposition::Assertion {
                disposition: Disposition::CoopMemberUnknown,
                ..
            }
        )
    })
}
