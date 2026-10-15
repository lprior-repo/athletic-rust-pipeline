use super::*;
use crate::model::{ContractRefV1, QueryProfileV1};
use sha2::{Digest, Sha256};

mod evidence;
mod windows;

fn fixture() -> TestResult<Fixture> {
    let mut fixture = Fixture::new()?;
    let mut input = fixture.profile.input().clone();
    input.policies.participation = participation_policy_v2()?;
    fixture.profile = QueryProfileV1::new(input)?;
    fixture.profile_id = fixture.profile.id()?;
    fixture.date = day("2026-05-16")?;
    fixture.status_interval = range("2026-05-16", "2026-05-16")?;
    Ok(fixture)
}

fn direct_case(fixture: &Fixture) -> TestResult<super::fixture::Case<'_>> {
    let mut case = fixture.case()?;
    case.statuses.first_mut().ok_or("status")?.basis = StatusBasis::ExplicitScholasticResult;
    case.windows.clear();
    Ok(case)
}

fn assessed(
    fixture: &Fixture,
    case: &super::fixture::Case<'_>,
) -> TestResult<(CandidateOutcome, Vec<ParticipationReason>)> {
    let observation = case.observation(fixture);
    let assessment = assess_participation_for_policy(fixture.query()?, &observation)?;
    Ok((assessment.outcome, assessment.reasons.iter().collect()))
}

#[test]
fn participation_v2_contract_is_exact_and_content_addressed() -> TestResult {
    let reference = participation_policy_v2()?;
    let actual = format!("{:x}", Sha256::digest(PARTICIPATION_POLICY_V2_DOCUMENT));
    check!(eq; actual, "fde4b6b731e1de8bd8dec3ec24ab881712bcee1d3522c31ae7d6a61d8c5fd8ee");
    check!(eq; reference.revision().get(), 2);
    check!(eq; reference.digest(), actual);
    check!(eq; reference.digest(), PARTICIPATION_POLICY_V2_SHA256);
    check!(eq; serde_json::to_value(reference)?, serde_json::json!({
        "version": 1,
        "revision": 2,
        "digest": actual,
    }));
    Ok(())
}

#[test]
fn participation_v2_accepts_a_dated_explicit_scholastic_result_without_a_summer_window(
) -> TestResult {
    let fixture = fixture()?;
    let case = direct_case(&fixture)?;
    check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    Ok(())
}

#[test]
fn participation_historical_entry_point_keeps_its_explicit_legacy_window_requirement() -> TestResult
{
    let fixture = Fixture::new()?;
    let case = direct_case(&fixture)?;
    let expected = (
        CandidateOutcome::Unresolved,
        vec![Reason::SummerBoundaryUnresolved],
    );
    check!(eq; outcome(&fixture, &case)?, expected);
    let observation = case.observation(&fixture);
    let assessment = assess_historical_participation(fixture.query()?, &observation)?;
    check!(eq; (assessment.outcome, assessment.reasons.iter().collect::<Vec<_>>()), expected);
    Ok(())
}

#[test]
fn participation_policy_dispatch_refuses_unregistered_revisions_and_digest_substitutions(
) -> TestResult {
    for (revision, digest) in [
        (1, PARTICIPATION_POLICY_V2_SHA256),
        (3, PARTICIPATION_POLICY_V2_SHA256),
        (
            2,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        (
            1,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
    ] {
        let mut fixture = fixture()?;
        let mut input = fixture.profile.input().clone();
        input.policies.participation = ContractRefV1::new(1, revision, digest)?;
        fixture.profile = QueryProfileV1::new(input)?;
        fixture.profile_id = fixture.profile.id()?;
        let case = direct_case(&fixture)?;
        let observation = case.observation(&fixture);
        check!(matches!(
            assess_participation_for_policy(fixture.query()?, &observation),
            Err(ParticipationInputError::UnsupportedPolicy)
        ));
    }
    Ok(())
}
