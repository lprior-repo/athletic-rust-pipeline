use super::*;
use crate::affiliation_candidates::KnownBounds;
use crate::model::{ContractRefV1, Id, ProfileId};
use fixture::{day, object, outcome, rejected, revision, Fixture};

#[test]
fn participation_refuses_wrong_profile_rule_and_frozen_generation_bindings() -> TestResult {
    let f = Fixture::new()?;
    let foreign_profile = ProfileId::parse(&"b".repeat(64))?;
    let foreign_rule = ContractRefV1::new(1, 2, &"a".repeat(64))?;
    let mut case = f.case()?;
    case.binding.profile = &foreign_profile;
    check!(matches!(
        rejected(&f, &case)?,
        ParticipationInputError::ProfileMismatch
    ));
    case.binding = f.binding()?;
    case.binding.participation_rule = &foreign_rule;
    check!(matches!(
        rejected(&f, &case)?,
        ParticipationInputError::PolicyMismatch
    ));
    case.binding = f.binding()?;
    case.binding.frozen.closure_object = object("foreign-frozen-root")?;
    check!(matches!(
        rejected(&f, &case)?,
        ParticipationInputError::FrozenBindingMismatch
    ));
    case.binding = f.binding()?;
    case.binding.frozen.revision = revision(8)?;
    check!(matches!(
        rejected(&f, &case)?,
        ParticipationInputError::FrozenBindingMismatch
    ));
    Ok(())
}

#[test]
fn participation_verifies_query_profile_identity_and_policy_instead_of_trusting_labels(
) -> TestResult {
    let f = Fixture::new()?;
    let case = f.case()?;
    let observation = case.observation(&f);
    let foreign_profile = ProfileId::parse(&"b".repeat(64))?;
    let mut query = f.query()?;
    query.binding.profile = &foreign_profile;
    check!(matches!(
        assess_historical_participation(query, &observation),
        Err(ParticipationInputError::ProfileMismatch)
    ));
    let foreign_rule = ContractRefV1::new(1, 1, &"c".repeat(64))?;
    let mut query = f.query()?;
    query.binding.participation_rule = &foreign_rule;
    check!(matches!(
        assess_historical_participation(query, &observation),
        Err(ParticipationInputError::PolicyMismatch)
    ));
    Ok(())
}

#[test]
fn participation_refuses_foreign_subject_result_or_known_lineage() -> TestResult {
    let f = Fixture::new()?;
    let foreign = Id::mint("ath", &["foreign"]);
    let mut case = f.case()?;
    case.statuses.first_mut().ok_or("status")?.proof.athlete = &foreign;
    check!(matches!(
        rejected(&f, &case)?,
        ParticipationInputError::ForeignSubject
    ));
    let foreign_result = Id::mint("perf", &["foreign"]);
    let mut case = f.case()?;
    if let ResultOwnership::Bound { performance, .. } = &mut case.ownership {
        *performance = &foreign_result;
    }
    check!(matches!(
        rejected(&f, &case)?,
        ParticipationInputError::ForeignSubject
    ));
    let mut case = f.case()?;
    case.statuses.first_mut().ok_or("status")?.proof.known =
        KnownBounds::new(object("foreign-generation")?, revision(1)?, revision(9)?)?;
    check!(matches!(
        rejected(&f, &case)?,
        ParticipationInputError::Temporal(_)
    ));
    Ok(())
}

#[test]
fn participation_refuses_a_summer_endpoint_not_bound_to_the_explicit_interval() -> TestResult {
    let f = Fixture::new()?;
    let foreign_end = day("2026-08-31")?;
    let mut case = f.case()?;
    case.windows
        .first_mut()
        .ok_or("window")?
        .documented_summer_end = &foreign_end;
    check!(matches!(
        rejected(&f, &case)?,
        ParticipationInputError::SummerEndpointMismatch
    ));
    Ok(())
}

#[test]
fn participation_refuses_proof_identity_reuse_with_different_capture_metadata() -> TestResult {
    let f = Fixture::new()?;
    let mut case = f.case()?;
    let mut duplicate = *case.statuses.first().ok_or("status")?;
    duplicate.proof.capture = object("other-capture")?;
    case.statuses.push(duplicate);
    check!(matches!(
        rejected(&f, &case)?,
        ParticipationInputError::ConflictingProofIdentity
    ));
    Ok(())
}

#[test]
fn participation_fact_admission_has_an_exact_finite_boundary() -> TestResult {
    let f = Fixture::new()?;
    let mut case = f.case()?;
    let context = *case.contexts.first().ok_or("context")?;
    case.contexts = vec![context; 60];
    check!(eq; outcome(&f, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    case.contexts.push(context);
    check!(matches!(
        rejected(&f, &case)?,
        ParticipationInputError::TooManyFacts
    ));
    Ok(())
}

#[test]
fn participation_reason_set_is_bounded_deduplicated_and_in_declared_order() {
    let mut reasons = ParticipationReasons::default();
    for reason in ParticipationReason::ALL.into_iter().rev() {
        reasons.add(reason);
        reasons.add(reason);
    }
    assert_eq!(
        reasons.iter().collect::<Vec<_>>(),
        ParticipationReason::ALL.to_vec()
    );
}
