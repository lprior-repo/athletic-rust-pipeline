use super::fixture::{instant, object, revision, Case};
use super::*;
use crate::affiliation_candidates::KnownBounds;
use crate::model::{ContractRefV1, GradYear, PerformanceWindowV1, QueryProfileV1};
use sha2::{Digest, Sha256};

fn synthetic_fixture() -> TestResult<Fixture> {
    let mut fixture = Fixture::new()?;
    let mut input = fixture.profile.input().clone();
    input.policies.participation = participation_policy_v3()?;
    fixture.profile = QueryProfileV1::new(input)?;
    fixture.profile_id = fixture.profile.id()?;
    Ok(fixture)
}

fn assessed(fixture: &Fixture, case: &Case<'_>) -> TestResult<(CandidateOutcome, Vec<Reason>)> {
    let observation = case.observation(fixture);
    let assessment = assess_participation_for_policy(fixture.query()?, &observation)?;
    Ok((assessment.outcome, assessment.reasons.iter().collect()))
}

#[test]
fn participation_v3_contract_is_canonical_and_content_addressed() -> TestResult {
    let reference = participation_policy_v3()?;
    let document: serde_json::Value = serde_json::from_str(PARTICIPATION_POLICY_V3_DOCUMENT)?;
    check!(eq; serde_json::to_string(&document)?, PARTICIPATION_POLICY_V3_DOCUMENT);
    let actual = format!("{:x}", Sha256::digest(PARTICIPATION_POLICY_V3_DOCUMENT));
    check!(eq; actual, "2068e3a252ef111b104117333dcf426e904d89d514fdaa28af9607cd51727882");
    check!(eq; reference.revision().get(), 3);
    check!(eq; reference.digest(), actual);
    check!(eq; reference.digest(), PARTICIPATION_POLICY_V3_SHA256);
    check!(eq; serde_json::to_value(reference)?, serde_json::json!({
        "version": 1, "revision": 3, "digest": actual,
    }));
    Ok(())
}

#[test]
fn participation_v3_synthetic_every_positive_context_requires_a_documented_window() -> TestResult {
    let fixture = synthetic_fixture()?;
    for context in [
        ParticipationContext::Scholastic,
        ParticipationContext::Club,
        ParticipationContext::Open,
        ParticipationContext::Unattached,
    ] {
        let mut case = fixture.case()?;
        case.contexts.first_mut().ok_or("context")?.context = context;
        case.windows.clear();
        check!(eq; assessed(&fixture, &case)?,
            (CandidateOutcome::Unresolved, vec![Reason::SummerBoundaryUnresolved]));
    }
    Ok(())
}

#[test]
fn participation_v3_synthetic_profile_query_window_is_not_participation_evidence() -> TestResult {
    let mut fixture = synthetic_fixture()?;
    let mut input = fixture.profile.input().clone();
    input.performance_window =
        PerformanceWindowV1::Inclusive(fixture.participation_interval.clone());
    fixture.profile = QueryProfileV1::new(input)?;
    fixture.profile_id = fixture.profile.id()?;
    let mut case = fixture.case()?;
    case.windows.clear();
    check!(eq; assessed(&fixture, &case)?,
        (CandidateOutcome::Unresolved, vec![Reason::SummerBoundaryUnresolved]));
    Ok(())
}

#[test]
fn participation_v3_synthetic_cohort_and_scholastic_division_do_not_establish_status() -> TestResult
{
    let fixture = synthetic_fixture()?;
    let mut case = fixture.case()?;
    case.statuses.first_mut().ok_or("status")?.basis = StatusBasis::ExplicitScholasticResult;
    check!(eq; assessed(&fixture, &case)?,
        (CandidateOutcome::Unresolved, vec![Reason::HighSchoolStatusMissing]));
    case.windows.clear();
    check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::Unresolved, vec![
        Reason::HighSchoolStatusMissing, Reason::SummerBoundaryUnresolved,
    ]));
    case.statuses.clear();
    check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::Unresolved, vec![
        Reason::HighSchoolStatusMissing, Reason::SummerBoundaryUnresolved,
    ]));
    Ok(())
}

#[test]
fn participation_v3_synthetic_matching_documented_window_and_independent_status_admit() -> TestResult
{
    let fixture = synthetic_fixture()?;
    for context in [
        ParticipationContext::Scholastic,
        ParticipationContext::Club,
        ParticipationContext::Open,
        ParticipationContext::Unattached,
    ] {
        for basis in [
            StatusBasis::DatedSchoolRoster,
            StatusBasis::ExplicitStudentStatus,
        ] {
            let mut case = fixture.case()?;
            case.contexts.first_mut().ok_or("context")?.context = context;
            case.statuses.first_mut().ok_or("status")?.basis = basis;
            check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
        }
    }
    Ok(())
}

#[test]
fn participation_v3_synthetic_window_endpoints_remain_exact_and_inclusive() -> TestResult {
    let fixture = synthetic_fixture()?;
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
        case.date = PerformanceDateFact::Exact(&date);
        check!(eq; assessed(&fixture, &case)?, expected);
    }
    let wrong_end = day("2026-08-31")?;
    let mut case = fixture.case()?;
    case.windows
        .first_mut()
        .ok_or("window")?
        .documented_summer_end = &wrong_end;
    let observation = case.observation(&fixture);
    check!(matches!(
        assess_participation_for_policy(fixture.query()?, &observation),
        Err(ParticipationInputError::SummerEndpointMismatch)
    ));
    Ok(())
}

#[test]
fn participation_v3_synthetic_competing_windows_remain_unresolved_in_either_order() -> TestResult {
    let mut fixture = synthetic_fixture()?;
    fixture.date = day("2026-08-15")?;
    let longer = range("2025-09-01", "2026-08-31")?;
    let mut case = fixture.case()?;
    let mut other = *case.windows.first().ok_or("window")?;
    other.interval = &longer;
    other.documented_summer_end = longer.through();
    other.boundary_proof = fixture.proof("competing-window")?;
    case.windows.push(other);
    let expected = (
        CandidateOutcome::Unresolved,
        vec![Reason::ParticipationWindowConflict],
    );
    check!(eq; assessed(&fixture, &case)?, expected);
    case.windows.reverse();
    check!(eq; assessed(&fixture, &case)?, expected);
    Ok(())
}

#[test]
fn participation_v3_synthetic_foreign_cohort_or_policy_window_is_not_applicable() -> TestResult {
    let fixture = synthetic_fixture()?;
    let old_policy = participation_policy_v2()?;
    let mut case = fixture.case()?;
    case.windows
        .first_mut()
        .ok_or("window")?
        .identity
        .participation_rule = &old_policy;
    check!(eq; assessed(&fixture, &case)?,
        (CandidateOutcome::Unresolved, vec![Reason::SummerBoundaryUnresolved]));
    let mut case = fixture.case()?;
    case.windows.first_mut().ok_or("window")?.identity.cohort =
        GradYear::new(2028).ok_or("cohort")?;
    check!(eq; assessed(&fixture, &case)?,
        (CandidateOutcome::Unresolved, vec![Reason::SummerBoundaryUnresolved]));
    Ok(())
}

#[test]
fn participation_v3_synthetic_invisible_window_and_status_cannot_establish_eligibility(
) -> TestResult {
    let fixture = synthetic_fixture()?;
    let later = KnownBounds::new(object("store-generation")?, revision(8)?, revision(9)?)?;
    let mut case = fixture.case()?;
    case.windows
        .first_mut()
        .ok_or("window")?
        .boundary_proof
        .known = later;
    check!(eq; assessed(&fixture, &case)?,
        (CandidateOutcome::Unresolved, vec![Reason::SummerBoundaryUnresolved]));
    case.statuses.first_mut().ok_or("status")?.proof.known = later;
    check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::Unresolved, vec![
        Reason::HighSchoolStatusMissing, Reason::SummerBoundaryUnresolved,
    ]));
    let mut case = fixture.case()?;
    case.windows
        .first_mut()
        .ok_or("window")?
        .boundary_proof
        .acquired_at = instant("2026-10-14T12:00:01Z")?;
    let observation = case.observation(&fixture);
    check!(matches!(
        assess_participation_for_policy(fixture.query()?, &observation),
        Err(ParticipationInputError::AcquisitionAfterFrozenInstant)
    ));
    Ok(())
}

#[test]
fn participation_v3_synthetic_dated_roster_status_is_not_extended_beyond_its_evidence() -> TestResult
{
    let fixture = synthetic_fixture()?;
    let earlier = range("2026-05-16", "2026-05-16")?;
    let mut case = fixture.case()?;
    case.statuses.first_mut().ok_or("status")?.interval = &earlier;
    case.windows.clear();
    check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::Unresolved, vec![
        Reason::StatusNotEstablishedOnPerformanceDate, Reason::SummerBoundaryUnresolved,
    ]));
    Ok(())
}

#[test]
fn participation_v3_synthetic_negative_status_and_conflicts_survive_every_basis() -> TestResult {
    let fixture = synthetic_fixture()?;
    for basis in [
        StatusBasis::ExplicitScholasticResult,
        StatusBasis::DatedSchoolRoster,
        StatusBasis::ExplicitStudentStatus,
    ] {
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
            let mut case = fixture.case()?;
            let mut negative = *case.statuses.first().ok_or("status")?;
            negative.status = status;
            negative.basis = basis;
            negative.proof = fixture.proof("negative-status")?;
            case.statuses.push(negative);
            let expected = (
                CandidateOutcome::Unresolved,
                vec![Reason::ParticipationEvidenceConflict, reason],
            );
            check!(eq; assessed(&fixture, &case)?, expected);
            case.statuses.reverse();
            check!(eq; assessed(&fixture, &case)?, expected);
            case.statuses
                .retain(|fact| fact.status != StudentStatus::HighSchool);
            check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::Excluded, vec![reason]));
        }
    }
    Ok(())
}

#[test]
fn participation_v3_synthetic_dispatch_rejects_substituted_digests() -> TestResult {
    for (revision, digest) in [
        (2, PARTICIPATION_POLICY_V3_SHA256),
        (3, PARTICIPATION_POLICY_V2_SHA256),
    ] {
        let mut fixture = synthetic_fixture()?;
        let mut input = fixture.profile.input().clone();
        input.policies.participation = ContractRefV1::new(1, revision, digest)?;
        fixture.profile = QueryProfileV1::new(input)?;
        fixture.profile_id = fixture.profile.id()?;
        let case = fixture.case()?;
        let observation = case.observation(&fixture);
        check!(matches!(
            assess_participation_for_policy(fixture.query()?, &observation),
            Err(ParticipationInputError::UnsupportedPolicy)
        ));
    }
    Ok(())
}

#[test]
fn participation_v3_synthetic_unsupported_positive_status_preserves_observed_contradictions(
) -> TestResult {
    let fixture = synthetic_fixture()?;
    for positive_basis in [
        StatusBasis::ExplicitScholasticResult,
        StatusBasis::DatedSchoolRoster,
        StatusBasis::ExplicitStudentStatus,
    ] {
        for negative_basis in [
            StatusBasis::ExplicitScholasticResult,
            StatusBasis::DatedSchoolRoster,
            StatusBasis::ExplicitStudentStatus,
        ] {
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
                let mut case = fixture.case()?;
                case.statuses.first_mut().ok_or("status")?.basis = positive_basis;
                let mut negative = *case.statuses.first().ok_or("status")?;
                negative.status = status;
                negative.basis = negative_basis;
                negative.proof = fixture.proof("negative-status")?;
                case.statuses.push(negative);
                let expected = (
                    CandidateOutcome::Unresolved,
                    vec![Reason::ParticipationEvidenceConflict, reason],
                );
                check!(eq; assessed(&fixture, &case)?, expected);
                case.statuses.reverse();
                check!(eq; assessed(&fixture, &case)?, expected);
                case.windows.clear();
                let expected = (
                    CandidateOutcome::Unresolved,
                    vec![
                        Reason::ParticipationEvidenceConflict,
                        Reason::SummerBoundaryUnresolved,
                        reason,
                    ],
                );
                check!(eq; assessed(&fixture, &case)?, expected);
                case.statuses.reverse();
                check!(eq; assessed(&fixture, &case)?, expected);
            }
        }
    }
    Ok(())
}

#[test]
fn participation_historical_entry_point_cannot_bypass_exact_versioned_policy_dispatch() -> TestResult
{
    for policy in [participation_policy_v2()?, participation_policy_v3()?] {
        let mut fixture = synthetic_fixture()?;
        let mut input = fixture.profile.input().clone();
        input.policies.participation = policy;
        fixture.profile = QueryProfileV1::new(input)?;
        fixture.profile_id = fixture.profile.id()?;
        let case = fixture.case()?;
        let observation = case.observation(&fixture);
        check!(matches!(
            assess_historical_participation(fixture.query()?, &observation),
            Err(ParticipationInputError::UnsupportedPolicy)
        ));
        check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    }
    Ok(())
}
