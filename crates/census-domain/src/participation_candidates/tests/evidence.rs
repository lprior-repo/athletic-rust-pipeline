use super::*;
use crate::affiliation_candidates::KnownBounds;
use fixture::{instant, object, outcome, rejected, revision, Fixture};

#[test]
fn participation_contradictions_are_unresolved_regardless_of_fact_order() -> TestResult {
    let f = Fixture::new()?;
    let mut case = f.case()?;
    case.statuses.push(HighSchoolStatusFact {
        status: StudentStatus::Collegiate,
        basis: StatusBasis::ExplicitStudentStatus,
        interval: &f.status_interval,
        proof: f.proof("college-status")?,
    });
    let expected = (
        CandidateOutcome::Unresolved,
        vec![
            Reason::ParticipationEvidenceConflict,
            Reason::CollegiateParticipation,
        ],
    );
    check!(eq; outcome(&f, &case)?, expected);
    case.statuses.reverse();
    check!(eq; outcome(&f, &case)?, expected);
    Ok(())
}

#[test]
fn participation_context_conflicts_are_not_resolved_by_cohort_or_input_order() -> TestResult {
    let f = Fixture::new()?;
    let mut case = f.case()?;
    case.contexts.push(ContextFact {
        context: ParticipationContext::PostGraduation,
        proof: f.proof("postgraduate-context")?,
    });
    let expected = (
        CandidateOutcome::Unresolved,
        vec![
            Reason::ParticipationEvidenceConflict,
            Reason::PostGraduationParticipation,
        ],
    );
    check!(eq; outcome(&f, &case)?, expected);
    case.contexts.reverse();
    check!(eq; outcome(&f, &case)?, expected);
    Ok(())
}

#[test]
fn participation_explicit_negative_student_status_excludes_a_scholastic_result() -> TestResult {
    let f = Fixture::new()?;
    for (status, reason) in [
        (StudentStatus::Collegiate, Reason::CollegiateParticipation),
        (
            StudentStatus::MiddleSchool,
            Reason::MiddleSchoolParticipation,
        ),
        (
            StudentStatus::PostGraduation,
            Reason::PostGraduationParticipation,
        ),
    ] {
        let mut case = f.case()?;
        case.statuses.first_mut().ok_or("status")?.status = status;
        check!(eq; outcome(&f, &case)?, (CandidateOutcome::Excluded, vec![reason]));
    }
    Ok(())
}

#[test]
fn participation_later_known_retrospective_status_is_independent_of_effective_cutoff() -> TestResult
{
    let f = Fixture::new()?;
    let case = f.case()?;
    check!(f.proof("status")?.acquired_at > instant("2026-08-15T23:59:59Z")?);
    check!(eq; outcome(&f, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    Ok(())
}

#[test]
fn participation_does_not_use_claims_outside_the_declared_knowledge_revision() -> TestResult {
    let f = Fixture::new()?;
    for bounds in [(8, 9), (1, 6)] {
        let mut case = f.case()?;
        case.statuses.first_mut().ok_or("status")?.proof.known = KnownBounds::new(
            object("store-generation")?,
            revision(bounds.0)?,
            revision(bounds.1)?,
        )?;
        check!(eq; outcome(&f, &case)?,
            (CandidateOutcome::Unresolved, vec![Reason::HighSchoolStatusMissing]));
    }
    Ok(())
}

#[test]
fn participation_invisible_later_conflict_does_not_rewrite_an_earlier_snapshot() -> TestResult {
    let f = Fixture::new()?;
    let mut case = f.case()?;
    let mut proof = f.proof("later-collegiate-status")?;
    proof.known = KnownBounds::new(object("store-generation")?, revision(8)?, revision(9)?)?;
    proof.acquired_at = instant("2026-11-01T12:00:00Z")?;
    case.statuses.push(HighSchoolStatusFact {
        status: StudentStatus::Collegiate,
        basis: StatusBasis::ExplicitStudentStatus,
        interval: &f.status_interval,
        proof,
    });
    check!(eq; outcome(&f, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    Ok(())
}

#[test]
fn participation_visible_evidence_after_the_frozen_instant_is_an_input_error() -> TestResult {
    let f = Fixture::new()?;
    let mut case = f.case()?;
    case.statuses.first_mut().ok_or("status")?.proof.acquired_at = instant("2026-10-14T12:00:01Z")?;
    check!(matches!(
        rejected(&f, &case)?,
        ParticipationInputError::AcquisitionAfterFrozenInstant
    ));
    Ok(())
}

#[test]
fn participation_duplicate_identical_facts_do_not_change_the_decision_or_reasons() -> TestResult {
    let f = Fixture::new()?;
    let mut case = f.case()?;
    let status = *case.statuses.first().ok_or("status")?;
    case.statuses.extend([status, status]);
    check!(eq; outcome(&f, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    case.contexts.first_mut().ok_or("context")?.context = ParticipationContext::Collegiate;
    let context = *case.contexts.first().ok_or("context")?;
    case.contexts.extend([context, context]);
    check!(eq; outcome(&f, &case)?,
        (CandidateOutcome::Excluded, vec![Reason::CollegiateParticipation]));
    Ok(())
}

#[test]
fn participation_source_result_and_dated_roster_are_distinct_positive_producer_bases() -> TestResult
{
    let f = Fixture::new()?;
    for basis in [
        StatusBasis::ExplicitScholasticResult,
        StatusBasis::DatedSchoolRoster,
        StatusBasis::ExplicitStudentStatus,
    ] {
        let mut case = f.case()?;
        case.statuses.first_mut().ok_or("status")?.basis = basis;
        check!(eq; outcome(&f, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    }
    Ok(())
}

#[test]
fn participation_a_cohort_proof_does_not_erase_explicit_ownership_or_cohort_conflicts() -> TestResult
{
    let f = Fixture::new()?;
    let mut case = f.case()?;
    case.ownership = ResultOwnership::Conflicting;
    case.cohort = CohortFact::Conflicting;
    check!(eq; outcome(&f, &case)?, (CandidateOutcome::Unresolved, vec![
        Reason::ResultOwnershipConflict, Reason::CohortConflict,
    ]));
    Ok(())
}
