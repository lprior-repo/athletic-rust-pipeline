use super::fixture::{instant, object, revision};
use super::*;
use crate::affiliation_candidates::KnownBounds;
use crate::evidence_subject_attachment::*;
use crate::model::{Id, QueryProfileV1};

mod guards;
mod support;

use support::*;

#[test]
fn documentary_v4_ownership_preserves_distinct_row_interpretation_and_subject_identities(
) -> TestResult {
    let f = v4_fixture()?;
    let document = Document::new()?;
    let mut case = f.case()?;
    case.ownership = document.ownership(&f)?;
    let ResultOwnership::DocumentaryBound {
        source_result,
        subject,
        proof,
        ..
    } = case.ownership
    else {
        return Err("documentary ownership absent".into());
    };
    check!(ne; proof.fact.as_str(), source_result.as_str());
    check!(ne; source_result.as_str(), subject.subject.as_str());
    check!(eq; assessed(&f, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    Ok(())
}

#[test]
fn documentary_ownership_does_not_resolve_missing_student_status_or_season() -> TestResult {
    let f = v4_fixture()?;
    let document = Document::new()?;
    let mut case = f.case()?;
    case.ownership = document.ownership(&f)?;
    case.statuses.clear();
    case.windows.clear();
    check!(eq; assessed(&f, &case)?, (CandidateOutcome::Unresolved,
        vec![Reason::HighSchoolStatusMissing, Reason::SummerBoundaryUnresolved]));
    Ok(())
}

#[test]
fn documentary_ownership_preserves_explicit_negative_status_and_conflicting_ownership() -> TestResult
{
    let f = v4_fixture()?;
    let document = Document::new()?;
    let mut case = f.case()?;
    case.ownership = document.ownership(&f)?;
    case.statuses.first_mut().ok_or("status absent")?.status = StudentStatus::Collegiate;
    let (outcome, reasons) = assessed(&f, &case)?;
    check!(eq; outcome, CandidateOutcome::Excluded);
    check!(reasons.contains(&Reason::CollegiateParticipation));
    case.ownership = ResultOwnership::Conflicting;
    let (outcome, reasons) = assessed(&f, &case)?;
    check!(eq; outcome, CandidateOutcome::Unresolved);
    check!(reasons.contains(&Reason::ResultOwnershipConflict));
    Ok(())
}

#[test]
fn documentary_ownership_hidden_by_knowledge_bounds_remains_unresolved() -> TestResult {
    let f = v4_fixture()?;
    let document = Document::new()?;
    for bounds in [(8, 9), (1, 6)] {
        let mut case = f.case()?;
        case.ownership = document.ownership(&f)?;
        let ResultOwnership::DocumentaryBound { proof, .. } = &mut case.ownership else {
            return Err("documentary ownership absent".into());
        };
        proof.known = KnownBounds::new(
            object("store-generation")?,
            revision(bounds.0)?,
            revision(bounds.1)?,
        )?;
        check!(eq; assessed(&f, &case)?, (CandidateOutcome::Unresolved, vec![Reason::ResultOwnershipUnresolved]));
    }
    Ok(())
}

#[test]
fn documentary_ownership_acquired_after_the_frozen_instant_is_rejected() -> TestResult {
    let f = v4_fixture()?;
    let document = Document::new()?;
    let mut case = f.case()?;
    case.ownership = document.ownership(&f)?;
    let ResultOwnership::DocumentaryBound { proof, .. } = &mut case.ownership else {
        return Err("documentary ownership absent".into());
    };
    proof.acquired_at = instant("2026-10-14T12:00:01Z")?;
    check!(matches!(
        rejected(&f, &case)?,
        ParticipationInputError::AcquisitionAfterFrozenInstant
    ));
    Ok(())
}

#[test]
fn provider_bound_v4_and_documentary_v4_share_eligibility_without_inventing_a_source_athlete(
) -> TestResult {
    let f = v4_fixture()?;
    let document = Document::new()?;
    let provider = f.case()?;
    let expected = assessed(&f, &provider)?;
    let mut documentary = f.case()?;
    documentary.ownership = document.ownership(&f)?;
    check!(eq; assessed(&f, &documentary)?, expected);
    let mut missing = f.case()?;
    missing.ownership = ResultOwnership::Missing;
    check!(eq; assessed(&f, &missing)?, (CandidateOutcome::Unresolved, vec![Reason::ResultOwnershipUnresolved]));
    Ok(())
}
