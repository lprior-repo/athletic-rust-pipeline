use super::*;
use ParticipationReason as Reason;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod evidence;
mod fixture;
mod metadata;
mod policy;
mod windows;

use fixture::{day, outcome, range, Fixture};

#[test]
fn participation_accepts_explicit_scholastic_club_open_and_unattached_candidates() -> TestResult {
    let f = Fixture::new()?;
    for context in [
        ParticipationContext::Scholastic,
        ParticipationContext::Club,
        ParticipationContext::Open,
        ParticipationContext::Unattached,
    ] {
        let mut case = f.case()?;
        case.contexts.first_mut().ok_or("context")?.context = context;
        check!(eq; outcome(&f, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    }
    Ok(())
}

#[test]
fn participation_excludes_explicit_collegiate_middle_school_and_post_graduation_results(
) -> TestResult {
    let f = Fixture::new()?;
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
        let mut case = f.case()?;
        case.contexts.first_mut().ok_or("context")?.context = context;
        check!(eq; outcome(&f, &case)?, (CandidateOutcome::Excluded, vec![reason]));
    }
    Ok(())
}

#[test]
fn participation_cannot_substitute_accepted_cohort_for_independent_school_status() -> TestResult {
    let f = Fixture::new()?;
    let mut case = f.case()?;
    case.statuses.clear();
    check!(eq; outcome(&f, &case)?,
        (CandidateOutcome::Unresolved, vec![Reason::HighSchoolStatusMissing]));
    Ok(())
}

#[test]
fn participation_accepts_only_explicit_inclusive_window_and_summer_endpoints() -> TestResult {
    let f = Fixture::new()?;
    for (date, expected) in [
        ("2025-09-01", (CandidateOutcome::EligibleCandidate, vec![])),
        ("2026-07-31", (CandidateOutcome::EligibleCandidate, vec![])),
        (
            "2025-08-31",
            (
                CandidateOutcome::Excluded,
                vec![Reason::OutsideParticipationWindow],
            ),
        ),
        (
            "2026-08-01",
            (
                CandidateOutcome::Excluded,
                vec![Reason::OutsideParticipationWindow],
            ),
        ),
    ] {
        let date = day(date)?;
        let mut case = f.case()?;
        case.date = PerformanceDateFact::Exact(&date);
        check!(eq; outcome(&f, &case)?, expected);
    }
    Ok(())
}

#[test]
fn participation_does_not_invent_a_summer_window() -> TestResult {
    let f = Fixture::new()?;
    let mut case = f.case()?;
    case.windows.clear();
    check!(eq; outcome(&f, &case)?,
        (CandidateOutcome::Unresolved, vec![Reason::SummerBoundaryUnresolved]));
    Ok(())
}

#[test]
fn participation_requires_school_status_on_the_actual_performance_day() -> TestResult {
    let f = Fixture::new()?;
    let earlier = range("2024-01-01", "2024-12-31")?;
    let mut case = f.case()?;
    case.statuses.first_mut().ok_or("status")?.interval = &earlier;
    check!(eq; outcome(&f, &case)?,
        (CandidateOutcome::Unresolved, vec![Reason::StatusNotEstablishedOnPerformanceDate]));
    Ok(())
}

#[test]
fn participation_preserves_missing_year_only_and_invalid_dates_as_distinct_gaps() -> TestResult {
    let f = Fixture::new()?;
    for (date, reason) in [
        (PerformanceDateFact::Missing, Reason::MissingPerformanceDate),
        (
            PerformanceDateFact::YearOnly,
            Reason::UnsupportedDatePrecision,
        ),
        (PerformanceDateFact::Invalid, Reason::InvalidPerformanceDate),
    ] {
        let mut case = f.case()?;
        case.date = date;
        check!(eq; outcome(&f, &case)?, (CandidateOutcome::Unresolved, vec![reason]));
    }
    check!(ProfileDateForTest::parse("2026-02-29").is_err());
    check!(ProfileDateForTest::parse("2024-02-29").is_ok());
    Ok(())
}

use crate::model::ProfileDate as ProfileDateForTest;

#[test]
fn participation_applies_selected_effective_cutoff_and_bounded_product_window() -> TestResult {
    let mut f = Fixture::new()?;
    let mut input = f.profile.input().clone();
    input.performance_window =
        crate::model::PerformanceWindowV1::Inclusive(range("2026-06-01", "2026-07-15")?);
    f.profile = crate::model::QueryProfileV1::new(input)?;
    f.profile_id = f.profile.id()?;
    for (date, expected) in [
        ("2026-06-01", (CandidateOutcome::EligibleCandidate, vec![])),
        ("2026-07-15", (CandidateOutcome::EligibleCandidate, vec![])),
        (
            "2026-05-31",
            (
                CandidateOutcome::Excluded,
                vec![Reason::BeforePerformanceWindow],
            ),
        ),
        (
            "2026-07-16",
            (
                CandidateOutcome::Excluded,
                vec![Reason::AfterPerformanceWindow],
            ),
        ),
        (
            "2026-08-16",
            (
                CandidateOutcome::Excluded,
                vec![
                    Reason::AfterEffectiveCutoff,
                    Reason::AfterPerformanceWindow,
                    Reason::OutsideParticipationWindow,
                ],
            ),
        ),
    ] {
        let date = day(date)?;
        let mut case = f.case()?;
        case.date = PerformanceDateFact::Exact(&date);
        check!(eq; outcome(&f, &case)?, expected);
    }
    Ok(())
}

#[test]
fn participation_preserves_all_ordered_reasons_without_promoting_provisional_records() -> TestResult
{
    let mut f = Fixture::new()?;
    let mut input = f.profile.input().clone();
    input.include_provisional = crate::model::ProvisionalInclusionV1::Include;
    f.profile = crate::model::QueryProfileV1::new(input)?;
    f.profile_id = f.profile.id()?;
    let mut case = f.case()?;
    case.ownership = ResultOwnership::Missing;
    case.cohort = CohortFact::Missing;
    case.contexts.clear();
    case.statuses.clear();
    case.windows.clear();
    check!(eq; outcome(&f, &case)?, (CandidateOutcome::Unresolved, vec![
        Reason::ResultOwnershipUnresolved, Reason::CohortUnresolved,
        Reason::ParticipationContextUnresolved, Reason::HighSchoolStatusMissing,
        Reason::SummerBoundaryUnresolved,
    ]));
    Ok(())
}

#[test]
fn participation_rejects_cohort_outside_profile_without_changing_status_evidence() -> TestResult {
    let f = Fixture::new()?;
    let mut case = f.case()?;
    let original_status = case.statuses.clone();
    case.cohort = CohortFact::Resolved {
        year: crate::model::GradYear::new(2028).ok_or("cohort")?,
        proof: f.proof("cohort")?,
    };
    // A cohort-2027 window cannot establish the boundary for a cohort-2028 athlete.
    check!(eq; outcome(&f, &case)?,
    (CandidateOutcome::Excluded, vec![
        Reason::CohortOutsideSelection, Reason::SummerBoundaryUnresolved,
    ]));
    check!(eq; case.statuses, original_status);
    Ok(())
}

#[test]
fn participation_effective_cutoff_is_inclusive_even_inside_a_later_explicit_summer_window(
) -> TestResult {
    let mut f = Fixture::new()?;
    f.participation_interval = range("2025-09-01", "2026-08-16")?;
    for (date, expected) in [
        ("2026-08-15", (CandidateOutcome::EligibleCandidate, vec![])),
        (
            "2026-08-16",
            (
                CandidateOutcome::Excluded,
                vec![Reason::AfterEffectiveCutoff],
            ),
        ),
    ] {
        let date = day(date)?;
        let mut case = f.case()?;
        case.date = PerformanceDateFact::Exact(&date);
        check!(eq; outcome(&f, &case)?, expected);
    }
    Ok(())
}

#[test]
fn participation_unknown_context_does_not_inherit_a_scholastic_classification() -> TestResult {
    let f = Fixture::new()?;
    let mut case = f.case()?;
    case.contexts.first_mut().ok_or("context")?.context = ParticipationContext::Unknown;
    check!(eq; outcome(&f, &case)?,
        (CandidateOutcome::Unresolved, vec![Reason::ParticipationContextUnresolved]));
    Ok(())
}
