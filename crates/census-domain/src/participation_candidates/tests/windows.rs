use super::*;
use crate::affiliation_candidates::KnownBounds;
use crate::model::{ContractRefV1, GradYear, ProfileDateRangeV1, SchoolYear};
use fixture::{object, revision};

#[test]
fn different_summer_endpoints_agree_in_overlap_and_conflict_only_in_the_disputed_period(
) -> TestResult {
    let fixture = Fixture::new()?;
    let longer = range("2025-09-01", "2026-08-31")?;
    for (date, expected) in [
        ("2026-06-15", (CandidateOutcome::EligibleCandidate, vec![])),
        ("2026-07-31", (CandidateOutcome::EligibleCandidate, vec![])),
        (
            "2026-08-15",
            (
                CandidateOutcome::Unresolved,
                vec![Reason::ParticipationWindowConflict],
            ),
        ),
    ] {
        let date = day(date)?;
        let mut case = fixture.case()?;
        case.date = PerformanceDateFact::Exact(&date);
        let mut alternative = *case.windows.first().ok_or("window")?;
        alternative.interval = &longer;
        alternative.documented_summer_end = longer.through();
        alternative.boundary_proof = fixture.proof("alternative-summer-boundary")?;
        case.windows.push(alternative);
        check!(eq; outcome(&fixture, &case)?, expected);
        case.windows.reverse();
        check!(eq; outcome(&fixture, &case)?, expected);
        let repeated = case.windows.clone();
        case.windows.extend(repeated);
        check!(eq; outcome(&fixture, &case)?, expected);
    }
    Ok(())
}

#[test]
fn different_starts_agree_in_overlap_and_do_not_admit_the_disputed_period() -> TestResult {
    let fixture = Fixture::new()?;
    let later_start = range("2026-01-01", "2026-07-31")?;
    for (date, expected) in [
        (
            "2025-09-01",
            (
                CandidateOutcome::Unresolved,
                vec![Reason::ParticipationWindowConflict],
            ),
        ),
        (
            "2025-12-31",
            (
                CandidateOutcome::Unresolved,
                vec![Reason::ParticipationWindowConflict],
            ),
        ),
        ("2026-01-01", (CandidateOutcome::EligibleCandidate, vec![])),
        ("2026-07-31", (CandidateOutcome::EligibleCandidate, vec![])),
    ] {
        let date = day(date)?;
        let mut case = fixture.case()?;
        case.date = PerformanceDateFact::Exact(&date);
        let mut alternative = *case.windows.first().ok_or("window")?;
        alternative.interval = &later_start;
        alternative.documented_summer_end = later_start.through();
        alternative.boundary_proof = fixture.proof("alternative-start-boundary")?;
        case.windows.push(alternative);
        check!(eq; outcome(&fixture, &case)?, expected);
        case.windows.reverse();
        check!(eq; outcome(&fixture, &case)?, expected);
    }
    Ok(())
}

fn overlapping_case<'a>(
    fixture: &'a Fixture,
    longer: &'a ProfileDateRangeV1,
) -> TestResult<fixture::Case<'a>> {
    let mut case = fixture.case()?;
    let mut alternative = *case.windows.first().ok_or("window")?;
    alternative.interval = longer;
    alternative.documented_summer_end = longer.through();
    alternative.boundary_proof = fixture.proof("alternative-summer-boundary")?;
    case.windows.push(alternative);
    Ok(case)
}

#[test]
fn different_endpoints_cannot_admit_a_date_outside_every_supported_interval() -> TestResult {
    let fixture = Fixture::new()?;
    let longer = range("2025-09-01", "2026-08-31")?;
    for (date, reasons) in [
        ("2025-08-31", vec![Reason::OutsideParticipationWindow]),
        (
            "2026-09-01",
            vec![
                Reason::AfterEffectiveCutoff,
                Reason::OutsideParticipationWindow,
            ],
        ),
    ] {
        let date = day(date)?;
        let mut case = overlapping_case(&fixture, &longer)?;
        case.date = PerformanceDateFact::Exact(&date);
        let expected = (CandidateOutcome::Excluded, reasons);
        check!(eq; outcome(&fixture, &case)?, expected);
        case.windows.reverse();
        check!(eq; outcome(&fixture, &case)?, expected);
    }
    Ok(())
}

#[test]
fn window_consensus_does_not_replace_school_status_on_the_performance_date() -> TestResult {
    let fixture = Fixture::new()?;
    let longer = range("2025-09-01", "2026-08-31")?;
    let mut case = overlapping_case(&fixture, &longer)?;
    let high_school = *case.statuses.first().ok_or("school status")?;
    case.statuses.clear();
    check!(eq; outcome(&fixture, &case)?,
        (CandidateOutcome::Unresolved, vec![Reason::HighSchoolStatusMissing]));
    let mut collegiate = high_school;
    collegiate.status = StudentStatus::Collegiate;
    collegiate.proof = fixture.proof("collegiate-status")?;
    case.statuses.push(collegiate);
    check!(eq; outcome(&fixture, &case)?,
        (CandidateOutcome::Excluded, vec![Reason::CollegiateParticipation]));
    case.statuses.push(high_school);
    let expected = (
        CandidateOutcome::Unresolved,
        vec![
            Reason::ParticipationEvidenceConflict,
            Reason::CollegiateParticipation,
        ],
    );
    check!(eq; outcome(&fixture, &case)?, expected);
    case.statuses.reverse();
    case.windows.reverse();
    check!(eq; outcome(&fixture, &case)?, expected);
    Ok(())
}

#[test]
fn pregraduation_window_consensus_keeps_the_effective_cutoff_inclusive() -> TestResult {
    let mut fixture = Fixture::new()?;
    let mut input = fixture.profile.input().clone();
    input.as_of = day("2026-06-15")?;
    fixture.profile = crate::model::QueryProfileV1::new(input)?;
    fixture.profile_id = fixture.profile.id()?;
    check!(eq; fixture.profile.input().graduation_years, vec![GradYear::CO2027]);
    let longer = range("2025-09-01", "2026-08-31")?;
    for (date, expected) in [
        ("2026-06-15", (CandidateOutcome::EligibleCandidate, vec![])),
        (
            "2026-06-16",
            (
                CandidateOutcome::Excluded,
                vec![Reason::AfterEffectiveCutoff],
            ),
        ),
    ] {
        let date = day(date)?;
        let mut case = overlapping_case(&fixture, &longer)?;
        case.date = PerformanceDateFact::Exact(&date);
        check!(eq; outcome(&fixture, &case)?, expected);
        case.windows.reverse();
        check!(eq; outcome(&fixture, &case)?, expected);
    }
    Ok(())
}

#[test]
fn window_consensus_keeps_the_selected_product_window_independent() -> TestResult {
    let mut fixture = Fixture::new()?;
    let mut input = fixture.profile.input().clone();
    input.performance_window =
        crate::model::PerformanceWindowV1::Inclusive(range("2026-06-01", "2026-06-15")?);
    fixture.profile = crate::model::QueryProfileV1::new(input)?;
    fixture.profile_id = fixture.profile.id()?;
    let longer = range("2025-09-01", "2026-08-31")?;
    for (date, expected) in [
        (
            "2026-05-31",
            (
                CandidateOutcome::Excluded,
                vec![Reason::BeforePerformanceWindow],
            ),
        ),
        ("2026-06-01", (CandidateOutcome::EligibleCandidate, vec![])),
        ("2026-06-15", (CandidateOutcome::EligibleCandidate, vec![])),
        (
            "2026-06-16",
            (
                CandidateOutcome::Excluded,
                vec![Reason::AfterPerformanceWindow],
            ),
        ),
    ] {
        let date = day(date)?;
        let mut case = overlapping_case(&fixture, &longer)?;
        case.date = PerformanceDateFact::Exact(&date);
        check!(eq; outcome(&fixture, &case)?, expected);
    }
    Ok(())
}

#[test]
fn identical_window_evidence_is_idempotent() -> TestResult {
    let fixture = Fixture::new()?;
    let mut case = fixture.case()?;
    let fact = *case.windows.first().ok_or("window")?;
    case.windows.extend([fact, fact]);
    check!(eq; outcome(&fixture, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    Ok(())
}

#[test]
fn unrelated_cohort_or_policy_windows_cannot_extend_eligibility() -> TestResult {
    let fixture = Fixture::new()?;
    let longer = range("2025-09-01", "2026-08-31")?;
    let date = day("2026-08-15")?;
    let different_policy = ContractRefV1::new(1, 2, &"b".repeat(64))?;
    for different_cohort in [true, false] {
        let mut case = fixture.case()?;
        case.date = PerformanceDateFact::Exact(&date);
        let mut unrelated = *case.windows.first().ok_or("window")?;
        unrelated.interval = &longer;
        unrelated.documented_summer_end = longer.through();
        unrelated.boundary_proof = fixture.proof("unrelated-boundary")?;
        if different_cohort {
            unrelated.identity.cohort = GradYear::new(2028).ok_or("cohort")?;
        } else {
            unrelated.identity.participation_rule = &different_policy;
        }
        case.windows.push(unrelated);
        let expected = (
            CandidateOutcome::Excluded,
            vec![Reason::OutsideParticipationWindow],
        );
        check!(eq; outcome(&fixture, &case)?, expected);
        case.windows.reverse();
        check!(eq; outcome(&fixture, &case)?, expected);
    }
    Ok(())
}

#[test]
fn separate_season_windows_do_not_conflict_when_they_do_not_overlap_the_result() -> TestResult {
    let fixture = Fixture::new()?;
    let previous = range("2024-09-01", "2025-07-31")?;
    let conflicting_previous = range("2024-09-01", "2025-08-31")?;
    let mut case = fixture.case()?;
    for (interval, proof) in [
        (&previous, "previous-boundary"),
        (&conflicting_previous, "previous-conflicting-boundary"),
    ] {
        let mut older = *case.windows.first().ok_or("window")?;
        older.identity.school_year = SchoolYear::new(2024).ok_or("school year")?;
        older.interval = interval;
        older.documented_summer_end = interval.through();
        older.boundary_proof = fixture.proof(proof)?;
        case.windows.push(older);
    }
    check!(eq; outcome(&fixture, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    case.windows.reverse();
    check!(eq; outcome(&fixture, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    Ok(())
}

#[test]
fn an_invisible_later_boundary_cannot_extend_or_conflict_with_the_selected_revision() -> TestResult
{
    let fixture = Fixture::new()?;
    let longer = range("2025-09-01", "2026-08-31")?;
    for (date, expected) in [
        ("2026-07-31", (CandidateOutcome::EligibleCandidate, vec![])),
        (
            "2026-08-15",
            (
                CandidateOutcome::Excluded,
                vec![Reason::OutsideParticipationWindow],
            ),
        ),
    ] {
        let date = day(date)?;
        let mut case = fixture.case()?;
        case.date = PerformanceDateFact::Exact(&date);
        let mut alternative = *case.windows.first().ok_or("window")?;
        alternative.interval = &longer;
        alternative.documented_summer_end = longer.through();
        alternative.boundary_proof = fixture.proof("later-boundary")?;
        alternative.boundary_proof.known =
            KnownBounds::new(object("store-generation")?, revision(8)?, revision(9)?)?;
        case.windows.push(alternative);
        check!(eq; outcome(&fixture, &case)?, expected);
    }
    Ok(())
}
