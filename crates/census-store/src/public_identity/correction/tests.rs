use crate::public_identity::{authority, context, operation::HeldWitness, PublicIdentityError};
use crate::public_identity_tests::support::{fixture, retained_state, subject, TestResult};
use crate::{StoreError, Table};
use census_domain::model::AppliedIdentityKind;
use census_domain::public_identity_history::{
    CandidateDecisionReference, PublicAthleteId, ReplayLimits,
};

mod codec;
mod refusal;

fn inputs(value: &str) -> TestResult<context::Inputs> {
    let athletes = vec![subject("reviewed-a", "41")?, subject("reviewed-b", "42")?];
    let members = athletes
        .iter()
        .map(|athlete| athlete.id.cast())
        .collect::<Vec<_>>();
    let mut inputs = context::Inputs {
        athletes,
        observations: Vec::new(),
        cases: Vec::new(),
        verdicts: Vec::new(),
    };
    let (case, verdict) = context::reviewed_fixture::review_inputs(&inputs, &members, value)?;
    inputs.cases.push(case);
    inputs.verdicts.push(verdict);
    Ok(inputs)
}

fn case_id(inputs: &context::Inputs) -> TestResult<&str> {
    Ok(&inputs.cases.first().ok_or("review case")?.id)
}

fn public_id() -> TestResult<PublicAthleteId> {
    Ok(PublicAthleteId::parse(&format!(
        "ath_public_v1_{}",
        "0".repeat(64)
    ))?)
}

#[test]
fn reviewed_authority_roundtrips_real_different_person_context() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let inputs = inputs("different_person")?;
    let original = serde_json::to_value((
        &inputs.athletes,
        &inputs.observations,
        &inputs.cases,
        &inputs.verdicts,
    ))?;
    let accepted = authority::accept_reviewed(&inputs, case_id(&inputs)?, "2026-10-14")?;
    check!(eq; accepted.record().kind, AppliedIdentityKind::DifferentPerson);
    let evidence = CandidateDecisionReference::from_correction(&accepted, ReplayLimits::SMALL)?;
    check!(evidence.matches_correction(accepted.record(), ReplayLimits::SMALL)?);
    let staged = inputs.stage(&store, writer.store_identity())?;
    let before = retained_state(&store)?;
    context::verify_reviewed(
        &store.db.snapshot(),
        &store.journal,
        staged.object_id(),
        accepted.record(),
    )?;
    check!(eq; serde_json::to_value((&inputs.athletes, &inputs.observations, &inputs.cases, &inputs.verdicts))?, original);
    check!(eq; retained_state(&store)?, before);
    check!(eq; writer.head(&store)?.sequence, 0);
    let mut changed = accepted.record().clone();
    changed.verdict_digest = Some("0".repeat(64));
    let refused = context::verify_reviewed(
        &store.db.snapshot(),
        &store.journal,
        staged.object_id(),
        &changed,
    );
    check!(matches!(refused, Err(StoreError::Refused { detail })
        if detail == "retained reviewed authority decision does not replay"));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
