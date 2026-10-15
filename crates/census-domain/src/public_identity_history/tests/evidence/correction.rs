use super::*;

mod fixture;

#[test]
fn correction_requires_a_real_accepted_different_person_review() -> TestResult {
    let accepted = fixture::accepted("different_person")?;
    check!(eq; accepted.record().kind, AppliedIdentityKind::DifferentPerson);
    let reference = CandidateDecisionReference::from_correction(&accepted, ReplayLimits::SMALL)?;
    check!(eq; reference.kind, CandidateEvidenceKind::Correction);
    check!(reference.matches_correction(accepted.record(), ReplayLimits::SMALL)?);
    check!(eq; CandidateDecisionReference::from_accepted(&accepted, ReplayLimits::SMALL), Err(HistoryError::EvidenceMismatch));
    check!(eq; reference.matches_applied(accepted.record(), ReplayLimits::SMALL), Err(HistoryError::EvidenceMismatch));
    check!(eq; reference.application_id, accepted.record().id);
    check!(eq; reference.members, accepted.record().members.iter().map(|member| member.subject.clone()).collect::<Vec<_>>());
    Ok(())
}

#[test]
fn positive_acceptance_cannot_be_used_as_correction_authority() -> TestResult {
    for accepted in [
        accepted_application("2026-10-14")?,
        fixture::accepted("same_person")?,
    ] {
        check!(eq; CandidateDecisionReference::from_correction(&accepted, ReplayLimits::SMALL), Err(HistoryError::EvidenceMismatch));
    }
    Ok(())
}

#[test]
fn correction_reference_binds_review_and_members_but_preserves_observation_time_semantics(
) -> TestResult {
    let accepted = fixture::accepted("different_person")?;
    let reference = CandidateDecisionReference::from_correction(&accepted, ReplayLimits::SMALL)?;
    let mut changed_case = accepted.record().clone();
    changed_case.case_id = Some("different-case".into());
    let mut changed_verdict = accepted.record().clone();
    changed_verdict.verdict_digest = Some("0".repeat(64));
    let mut changed_member = accepted.record().clone();
    changed_member.members.reverse();
    for changed in [changed_case, changed_verdict, changed_member] {
        check!(!reference.matches_correction(&changed, ReplayLimits::SMALL)?);
    }
    let mut later = accepted.record().clone();
    later.observed_at = "2026-10-15".into();
    check!(reference.matches_correction(&later, ReplayLimits::SMALL)?);
    let limits = ReplayLimits {
        members_per_operation: 0,
        ..ReplayLimits::SMALL
    };
    check!(eq; reference.matches_correction(accepted.record(), limits), Err(HistoryError::Limit("evidence members")));
    Ok(())
}
