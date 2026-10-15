use super::*;

#[test]
fn retained_record_matches_the_reference_built_through_existing_acceptance() -> TestResult {
    let accepted = accepted_application("2026-10-14")?;
    let reference = CandidateDecisionReference::from_accepted(&accepted, ReplayLimits::SMALL)?;
    let original = accepted.record().clone();
    check!(reference.matches_applied(accepted.record(), ReplayLimits::SMALL)?);
    check!(eq; accepted.record(), &original);
    Ok(())
}

#[test]
fn every_reference_field_is_compared_with_retained_application() -> TestResult {
    let accepted = accepted_application("2026-10-14")?;
    let reference = CandidateDecisionReference::from_accepted(&accepted, ReplayLimits::SMALL)?;
    let mut changed_id = reference.clone();
    changed_id.application_id.push_str(":changed");
    let mut changed_digest = reference.clone();
    changed_digest.evidence_digest = "0".repeat(64);
    let mut changed_kind = reference.clone();
    changed_kind.kind = CandidateEvidenceKind::SamePerson;
    let mut changed_member = reference.clone();
    changed_member.members = vec![candidate("other")];
    let mut extra_member = reference.clone();
    extra_member.members.push(candidate("extra"));
    let mut missing_member = reference.clone();
    missing_member.members.clear();
    let mut correction = reference;
    correction.kind = CandidateEvidenceKind::Correction;
    for changed in [
        changed_id,
        changed_digest,
        changed_kind,
        changed_member,
        extra_member,
        missing_member,
        correction,
    ] {
        check!(!changed.matches_applied(accepted.record(), ReplayLimits::SMALL)?);
    }
    Ok(())
}

#[test]
fn canonical_evidence_digest_binds_every_retained_decision_field_except_observation_time(
) -> TestResult {
    let accepted = accepted_application("2026-10-14")?;
    let reference = CandidateDecisionReference::from_accepted(&accepted, ReplayLimits::SMALL)?;
    let original = accepted.record();
    let mut changed_id = original.clone();
    changed_id.id.push_str(":changed");
    let mut changed_policy = original.clone();
    changed_policy.policy = 2;
    let mut changed_kind = original.clone();
    changed_kind.kind = AppliedIdentityKind::SamePerson;
    let mut changed_subject = original.clone();
    changed_subject.members.first_mut().ok_or("member")?.subject = candidate("other");
    let mut changed_evidence = original.clone();
    changed_evidence
        .members
        .first_mut()
        .ok_or("member")?
        .evidence_digest = "0".repeat(64);
    let mut changed_target = original.clone();
    changed_target.canonical_id = Some(candidate("other").cast());
    let mut missing_target = original.clone();
    missing_target.canonical_id = None;
    let mut changed_case = original.clone();
    changed_case.case_id = Some("case:changed".into());
    let mut changed_verdict = original.clone();
    changed_verdict.verdict_digest = Some("1".repeat(64));
    for changed in [
        changed_id,
        changed_policy,
        changed_kind,
        changed_subject,
        changed_evidence,
        changed_target,
        missing_target,
        changed_case,
        changed_verdict,
    ] {
        check!(!reference.matches_applied(&changed, ReplayLimits::SMALL)?);
    }
    Ok(())
}

#[test]
fn historical_observation_time_remains_outside_reference_identity() -> TestResult {
    let accepted = accepted_application("2026-10-14")?;
    let reference = CandidateDecisionReference::from_accepted(&accepted, ReplayLimits::SMALL)?;
    let mut retained = accepted.record().clone();
    retained.observed_at = "2026-10-15".into();
    check!(reference.matches_applied(&retained, ReplayLimits::SMALL)?);
    check!(eq; accepted_reference("2026-10-15")?, reference);
    Ok(())
}

#[test]
fn different_person_is_not_supported_reference_authority() -> TestResult {
    let accepted = accepted_application("2026-10-14")?;
    let reference = CandidateDecisionReference::from_accepted(&accepted, ReplayLimits::SMALL)?;
    let mut retained = accepted.record().clone();
    retained.kind = AppliedIdentityKind::DifferentPerson;
    check!(eq; reference.matches_applied(&retained, ReplayLimits::SMALL), Err(HistoryError::EvidenceMismatch));
    Ok(())
}

#[test]
fn equality_preserves_constructor_admission_and_error_order() -> TestResult {
    let accepted = accepted_application("2026-10-14")?;
    let reference = CandidateDecisionReference::from_accepted(&accepted, ReplayLimits::SMALL)?;
    for limits in [
        ReplayLimits {
            members_per_operation: 0,
            ..ReplayLimits::SMALL
        },
        ReplayLimits {
            text_bytes: 1,
            ..ReplayLimits::SMALL
        },
        ReplayLimits {
            total_text_bytes: 1,
            ..ReplayLimits::SMALL
        },
    ] {
        let expected = CandidateDecisionReference::from_accepted(&accepted, limits)
            .err()
            .ok_or("constructor accepted limit")?;
        check!(eq; reference.matches_applied(accepted.record(), limits).err(), Some(expected));
    }
    let mut retained = accepted.record().clone();
    retained.kind = AppliedIdentityKind::DifferentPerson;
    let limits = ReplayLimits {
        members_per_operation: 0,
        ..ReplayLimits::SMALL
    };
    check!(eq; reference.matches_applied(&retained, limits), Err(HistoryError::Limit("evidence members")));
    Ok(())
}

#[test]
fn invalid_retained_text_is_refused_before_reference_construction() -> TestResult {
    let accepted = accepted_application("2026-10-14")?;
    let reference = CandidateDecisionReference::from_accepted(&accepted, ReplayLimits::SMALL)?;
    for invalid in [
        String::new(),
        "application\ncontrol".into(),
        "a".repeat(513),
    ] {
        let mut retained = accepted.record().clone();
        retained.id = invalid;
        check!(reference
            .matches_applied(&retained, ReplayLimits::SMALL)
            .is_err());
    }
    Ok(())
}
