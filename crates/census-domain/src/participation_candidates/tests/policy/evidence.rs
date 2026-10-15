use super::*;
use crate::affiliation_candidates::KnownBounds;
use crate::model::ProfileId;
use crate::participation_candidates::tests::fixture::{instant, object, revision};

#[test]
fn participation_v2_requires_explicit_scholastic_status_not_just_cohort_or_roster() -> TestResult {
    let fixture = fixture()?;
    for basis in [
        StatusBasis::DatedSchoolRoster,
        StatusBasis::ExplicitStudentStatus,
    ] {
        let mut case = direct_case(&fixture)?;
        case.statuses.first_mut().ok_or("status")?.basis = basis;
        check!(eq; assessed(&fixture, &case)?,
            (CandidateOutcome::Unresolved, vec![Reason::SummerBoundaryUnresolved]));
    }
    let mut case = direct_case(&fixture)?;
    case.statuses.clear();
    check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::Unresolved, vec![
        Reason::HighSchoolStatusMissing, Reason::SummerBoundaryUnresolved,
    ]));
    Ok(())
}

#[test]
fn participation_v2_requires_the_exact_date_and_never_extends_scholastic_status_into_summer(
) -> TestResult {
    let fixture = fixture()?;
    for (date, reasons) in [
        (
            PerformanceDateFact::Missing,
            vec![Reason::MissingPerformanceDate],
        ),
        (
            PerformanceDateFact::YearOnly,
            vec![Reason::UnsupportedDatePrecision],
        ),
        (
            PerformanceDateFact::Invalid,
            vec![Reason::InvalidPerformanceDate],
        ),
    ] {
        let mut case = direct_case(&fixture)?;
        case.date = date;
        check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::Unresolved, reasons));
    }
    for date in ["2026-05-15", "2026-05-17", "2026-07-31"] {
        let date = day(date)?;
        let mut case = direct_case(&fixture)?;
        case.date = PerformanceDateFact::Exact(&date);
        check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::Unresolved, vec![
            Reason::StatusNotEstablishedOnPerformanceDate, Reason::SummerBoundaryUnresolved,
        ]));
    }
    Ok(())
}

#[test]
fn participation_v2_requires_a_window_for_club_open_unattached_and_mixed_contexts() -> TestResult {
    let fixture = fixture()?;
    for context in [
        ParticipationContext::Club,
        ParticipationContext::Open,
        ParticipationContext::Unattached,
    ] {
        let mut case = direct_case(&fixture)?;
        case.contexts.first_mut().ok_or("context")?.context = context;
        let expected = (
            CandidateOutcome::Unresolved,
            vec![Reason::SummerBoundaryUnresolved],
        );
        check!(eq; assessed(&fixture, &case)?, expected);
        case.contexts.push(ContextFact {
            context: ParticipationContext::Scholastic,
            proof: fixture.proof("additional-scholastic-context")?,
        });
        check!(eq; assessed(&fixture, &case)?, expected);
        case.contexts.reverse();
        check!(eq; assessed(&fixture, &case)?, expected);
    }
    Ok(())
}

#[test]
fn participation_v2_preserves_missing_unknown_and_invisible_contexts_as_unresolved() -> TestResult {
    let fixture = fixture()?;
    let expected = (
        CandidateOutcome::Unresolved,
        vec![
            Reason::ParticipationContextUnresolved,
            Reason::SummerBoundaryUnresolved,
        ],
    );
    let mut case = direct_case(&fixture)?;
    case.contexts.clear();
    check!(eq; assessed(&fixture, &case)?, expected);
    let mut case = direct_case(&fixture)?;
    case.contexts.first_mut().ok_or("context")?.context = ParticipationContext::Unknown;
    check!(eq; assessed(&fixture, &case)?, expected);
    let mut case = direct_case(&fixture)?;
    case.contexts.first_mut().ok_or("context")?.proof.known =
        KnownBounds::new(object("store-generation")?, revision(8)?, revision(9)?)?;
    check!(eq; assessed(&fixture, &case)?, expected);
    Ok(())
}

#[test]
fn participation_v2_ignores_invisible_status_and_does_not_ignore_visible_negative_status(
) -> TestResult {
    let fixture = fixture()?;
    let mut case = direct_case(&fixture)?;
    case.statuses.first_mut().ok_or("status")?.proof.known =
        KnownBounds::new(object("store-generation")?, revision(8)?, revision(9)?)?;
    check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::Unresolved, vec![
        Reason::HighSchoolStatusMissing, Reason::SummerBoundaryUnresolved,
    ]));
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
        let mut case = direct_case(&fixture)?;
        let mut negative = *case.statuses.first().ok_or("status")?;
        negative.status = status;
        negative.proof = fixture.proof("negative-status")?;
        case.statuses.clear();
        case.statuses.push(negative);
        check!(eq; assessed(&fixture, &case)?,
            (CandidateOutcome::Excluded, vec![Reason::SummerBoundaryUnresolved, reason]));
        let positive = *direct_case(&fixture)?
            .statuses
            .first()
            .ok_or("positive status")?;
        case.statuses.push(positive);
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
    Ok(())
}

#[test]
fn participation_v2_never_admits_collegiate_middle_school_or_postgraduation_contexts() -> TestResult
{
    let fixture = fixture()?;
    for (context, reason) in [
        (
            ParticipationContext::Collegiate,
            Reason::CollegiateParticipation,
        ),
        (
            ParticipationContext::MiddleSchool,
            Reason::MiddleSchoolParticipation,
        ),
        (
            ParticipationContext::PostGraduation,
            Reason::PostGraduationParticipation,
        ),
    ] {
        let mut case = direct_case(&fixture)?;
        case.contexts.first_mut().ok_or("context")?.context = context;
        check!(eq; assessed(&fixture, &case)?,
            (CandidateOutcome::Excluded, vec![Reason::SummerBoundaryUnresolved, reason]));
        case.contexts.push(ContextFact {
            context: ParticipationContext::Scholastic,
            proof: fixture.proof("additional-scholastic-context")?,
        });
        check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::Unresolved, vec![
            Reason::ParticipationEvidenceConflict, Reason::SummerBoundaryUnresolved, reason,
        ]));
    }
    Ok(())
}

#[test]
fn participation_v2_retains_ownership_and_cohort_requirements_for_date_local_results() -> TestResult
{
    let fixture = fixture()?;
    let mut case = direct_case(&fixture)?;
    case.ownership = ResultOwnership::Missing;
    case.cohort = CohortFact::Missing;
    check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::Unresolved, vec![
        Reason::ResultOwnershipUnresolved, Reason::CohortUnresolved,
    ]));
    case.ownership = ResultOwnership::Conflicting;
    case.cohort = CohortFact::Conflicting;
    check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::Unresolved, vec![
        Reason::ResultOwnershipConflict, Reason::CohortConflict,
    ]));
    Ok(())
}

#[test]
fn participation_v2_uses_existing_profile_binding_and_frozen_visibility_validation() -> TestResult {
    let fixture = fixture()?;
    let foreign_profile = ProfileId::parse(&"b".repeat(64))?;
    let mut case = direct_case(&fixture)?;
    case.binding.profile = &foreign_profile;
    let observation = case.observation(&fixture);
    check!(matches!(
        assess_participation_for_policy(fixture.query()?, &observation),
        Err(ParticipationInputError::ProfileMismatch)
    ));
    let mut case = direct_case(&fixture)?;
    case.statuses.first_mut().ok_or("status")?.proof.acquired_at = instant("2026-10-14T12:00:01Z")?;
    let observation = case.observation(&fixture);
    check!(matches!(
        assess_participation_for_policy(fixture.query()?, &observation),
        Err(ParticipationInputError::AcquisitionAfterFrozenInstant)
    ));
    Ok(())
}

#[test]
fn participation_v2_still_honors_profile_cutoff_and_product_performance_window() -> TestResult {
    let mut fixture = fixture()?;
    let mut input = fixture.profile.input().clone();
    input.as_of = day("2026-05-15")?;
    input.performance_window =
        crate::model::PerformanceWindowV1::Inclusive(range("2026-05-01", "2026-05-15")?);
    fixture.profile = QueryProfileV1::new(input)?;
    fixture.profile_id = fixture.profile.id()?;
    let case = direct_case(&fixture)?;
    check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::Excluded, vec![
        Reason::AfterEffectiveCutoff, Reason::AfterPerformanceWindow,
    ]));
    Ok(())
}

#[test]
fn participation_v2_date_local_status_ignores_invisible_and_out_of_date_negative_facts(
) -> TestResult {
    let fixture = fixture()?;
    let earlier = range("2024-01-01", "2024-12-31")?;
    let mut case = direct_case(&fixture)?;
    let mut later = fixture.proof("later-collegiate-status")?;
    later.known = KnownBounds::new(object("store-generation")?, revision(8)?, revision(9)?)?;
    case.statuses.push(HighSchoolStatusFact {
        status: StudentStatus::Collegiate,
        basis: StatusBasis::ExplicitStudentStatus,
        interval: &fixture.status_interval,
        proof: later,
    });
    case.statuses.push(HighSchoolStatusFact {
        status: StudentStatus::MiddleSchool,
        basis: StatusBasis::ExplicitStudentStatus,
        interval: &earlier,
        proof: fixture.proof("earlier-middle-school-status")?,
    });
    case.contexts.push(ContextFact {
        context: ParticipationContext::Club,
        proof: later,
    });
    check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    case.statuses.reverse();
    case.contexts.reverse();
    check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    Ok(())
}

#[test]
fn participation_v2_date_local_facts_are_idempotent_with_the_existing_admission_bound() -> TestResult
{
    let fixture = fixture()?;
    let mut case = direct_case(&fixture)?;
    let context = *case.contexts.first().ok_or("context")?;
    let status = *case.statuses.first().ok_or("status")?;
    case.contexts.extend([context, context]);
    case.statuses.extend([status, status]);
    check!(eq; assessed(&fixture, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    case.contexts = vec![context; MAX_PARTICIPATION_FACTS];
    let observation = case.observation(&fixture);
    check!(matches!(
        assess_participation_for_policy(fixture.query()?, &observation),
        Err(ParticipationInputError::TooManyFacts)
    ));
    Ok(())
}
