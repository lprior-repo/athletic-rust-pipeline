use super::*;
use crate::affiliation_candidates::KnownBounds;
use crate::model::GradYear;
use crate::participation_candidates::tests::fixture::{object, revision};

#[test]
fn participation_v2_never_overrides_an_applicable_window_with_date_local_status() -> TestResult {
    let mut fixture = fixture()?;
    fixture.status_interval = range("2025-01-01", "2027-12-31")?;
    for (date, expected) in [
        (
            "2025-08-31",
            (
                CandidateOutcome::Excluded,
                vec![Reason::OutsideParticipationWindow],
            ),
        ),
        ("2025-09-01", (CandidateOutcome::EligibleCandidate, vec![])),
        ("2026-07-31", (CandidateOutcome::EligibleCandidate, vec![])),
        (
            "2026-08-01",
            (
                CandidateOutcome::Excluded,
                vec![Reason::OutsideParticipationWindow],
            ),
        ),
    ] {
        let date = day(date)?;
        let mut case = fixture.case()?;
        case.statuses.first_mut().ok_or("status")?.basis = StatusBasis::ExplicitScholasticResult;
        case.date = PerformanceDateFact::Exact(&date);
        check!(eq; assessed(&fixture, &case)?, expected);
        check!(eq; assessed(&fixture, &case)?, outcome(&fixture, &case)?);
    }
    Ok(())
}

#[test]
fn participation_v2_preserves_window_conflicts_and_input_order_independence() -> TestResult {
    let mut fixture = fixture()?;
    fixture.date = day("2026-08-15")?;
    fixture.status_interval = range("2026-08-15", "2026-08-15")?;
    let longer = range("2025-09-01", "2026-08-31")?;
    let mut case = fixture.case()?;
    case.statuses.first_mut().ok_or("status")?.basis = StatusBasis::ExplicitScholasticResult;
    let mut alternative = *case.windows.first().ok_or("window")?;
    alternative.interval = &longer;
    alternative.documented_summer_end = longer.through();
    alternative.boundary_proof = fixture.proof("alternative-boundary")?;
    case.windows.push(alternative);
    let expected = (
        CandidateOutcome::Unresolved,
        vec![Reason::ParticipationWindowConflict],
    );
    check!(eq; assessed(&fixture, &case)?, expected);
    check!(eq; assessed(&fixture, &case)?, outcome(&fixture, &case)?);
    case.windows.reverse();
    check!(eq; assessed(&fixture, &case)?, expected);
    let duplicate = *case.windows.first().ok_or("window")?;
    case.windows.push(duplicate);
    check!(eq; assessed(&fixture, &case)?, expected);
    Ok(())
}

#[test]
fn participation_v2_uses_date_local_status_when_every_window_is_inapplicable() -> TestResult {
    let fixture = fixture()?;
    let other_policy = ContractRefV1::new(1, 1, &"a".repeat(64))?;
    let outside = range("2025-01-01", "2025-07-31")?;
    for identity in [
        EligibilityWindowIdentity {
            participation_rule: &other_policy,
            ..fixture.case()?.windows.first().ok_or("window")?.identity
        },
        EligibilityWindowIdentity {
            cohort: GradYear::new(2028).ok_or("cohort")?,
            ..fixture.case()?.windows.first().ok_or("window")?.identity
        },
    ] {
        let mut case = direct_case(&fixture)?;
        case.windows.push(EligibilityWindowFact {
            identity,
            interval: &outside,
            documented_summer_end: outside.through(),
            boundary_proof: fixture.proof("unrelated-window")?,
        });
        check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    }
    Ok(())
}

#[test]
fn participation_v2_does_not_apply_invisible_windows_to_an_earlier_frozen_view() -> TestResult {
    let fixture = fixture()?;
    let outside = range("2025-01-01", "2025-07-31")?;
    let mut case = fixture.case()?;
    case.statuses.first_mut().ok_or("status")?.basis = StatusBasis::ExplicitScholasticResult;
    let window = case.windows.first_mut().ok_or("window")?;
    window.interval = &outside;
    window.documented_summer_end = outside.through();
    window.boundary_proof.known =
        KnownBounds::new(object("store-generation")?, revision(8)?, revision(9)?)?;
    check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    Ok(())
}

#[test]
fn participation_v2_keeps_documented_summer_extension_behavior_for_every_positive_context(
) -> TestResult {
    let mut fixture = fixture()?;
    fixture.date = day("2026-07-31")?;
    fixture.status_interval = range("2026-01-01", "2026-07-31")?;
    for context in [
        ParticipationContext::Scholastic,
        ParticipationContext::Club,
        ParticipationContext::Open,
        ParticipationContext::Unattached,
    ] {
        for basis in [
            StatusBasis::ExplicitScholasticResult,
            StatusBasis::DatedSchoolRoster,
            StatusBasis::ExplicitStudentStatus,
        ] {
            let mut case = fixture.case()?;
            case.contexts.first_mut().ok_or("context")?.context = context;
            case.statuses.first_mut().ok_or("status")?.basis = basis;
            check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
            check!(eq; assessed(&fixture, &case)?, outcome(&fixture, &case)?);
        }
    }
    Ok(())
}

#[test]
fn participation_v2_does_not_relax_endpoint_metadata_validation_for_scholastic_results(
) -> TestResult {
    let fixture = fixture()?;
    let invalid_end = day("2026-08-31")?;
    let mut case = fixture.case()?;
    case.statuses.first_mut().ok_or("status")?.basis = StatusBasis::ExplicitScholasticResult;
    case.windows
        .first_mut()
        .ok_or("window")?
        .documented_summer_end = &invalid_end;
    let observation = case.observation(&fixture);
    check!(matches!(
        assess_participation_for_policy(fixture.query()?, &observation),
        Err(ParticipationInputError::SummerEndpointMismatch)
    ));
    Ok(())
}
