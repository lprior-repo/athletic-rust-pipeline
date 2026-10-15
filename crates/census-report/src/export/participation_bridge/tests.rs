use super::*;
use census_domain::model::{GradYear, ProfileDate, SourceIdentity, SourceNamespace};
use census_domain::participation_candidates::{ParticipationInputError, ParticipationReason};

mod fixture;
mod profile;
use fixture::Fixture;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn authentic_troy_assessment_preserves_missing_status_and_summer_boundary() -> TestResult {
    let fixture = Fixture::prepare(None).await?;
    let result = fixture.assess(SourceCohortAgreement::Resolved(GradYear::CO2027))?;
    let (outcome, reasons, evidence) = result.into_parts();
    assert_eq!(outcome, CandidateOutcome::Unresolved);
    assert!(reasons.contains(ParticipationReason::ParticipationContextUnresolved));
    assert!(reasons.contains(ParticipationReason::HighSchoolStatusMissing));
    assert!(reasons.contains(ParticipationReason::SummerBoundaryUnresolved));
    assert!(!reasons.contains(ParticipationReason::MissingPerformanceDate));
    assert_eq!(evidence.len(), 3);
    Ok(())
}

#[tokio::test]
async fn synthetic_owned_high_school_division_does_not_establish_independent_status_or_window(
) -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let (outcome, reasons, _) = fixture
        .assess(SourceCohortAgreement::Resolved(GradYear::CO2027))?
        .into_parts();
    assert_eq!(outcome, CandidateOutcome::Unresolved);
    assert_eq!(
        reasons.iter().collect::<Vec<_>>(),
        vec![
            ParticipationReason::HighSchoolStatusMissing,
            ParticipationReason::SummerBoundaryUnresolved,
        ]
    );
    Ok(())
}

#[tokio::test]
async fn synthetic_club_open_and_unattached_labels_do_not_invent_a_summer_extension() -> TestResult
{
    for label in ["Club", "Open", "Unattached"] {
        let fixture = Fixture::prepare(Some(label)).await?;
        let (outcome, reasons, _) = fixture
            .assess(SourceCohortAgreement::Resolved(GradYear::CO2027))?
            .into_parts();
        assert_eq!(outcome, CandidateOutcome::Unresolved);
        assert!(reasons.contains(ParticipationReason::HighSchoolStatusMissing));
        assert!(reasons.contains(ParticipationReason::SummerBoundaryUnresolved));
    }
    Ok(())
}

#[tokio::test]
async fn synthetic_non_high_school_divisions_are_excluded() -> TestResult {
    for (label, reason) in [
        (
            "Middle School",
            ParticipationReason::MiddleSchoolParticipation,
        ),
        ("Collegiate", ParticipationReason::CollegiateParticipation),
        (
            "Post Graduation",
            ParticipationReason::PostGraduationParticipation,
        ),
    ] {
        let fixture = Fixture::prepare(Some(label)).await?;
        let (outcome, reasons, _) = fixture
            .assess(SourceCohortAgreement::Resolved(GradYear::CO2027))?
            .into_parts();
        assert_eq!(outcome, CandidateOutcome::Excluded);
        assert!(reasons.contains(reason));
    }
    Ok(())
}

#[tokio::test]
async fn canonical_cohort_conflicts_cannot_be_erased_by_a_published_row() -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    for cohort in [
        SourceCohortAgreement::Conflicting,
        SourceCohortAgreement::Resolved(GradYear::new(2028).ok_or("year")?),
    ] {
        let (outcome, reasons, _) = fixture.assess(cohort)?.into_parts();
        assert_eq!(outcome, CandidateOutcome::Unresolved);
        assert!(reasons.contains(ParticipationReason::CohortConflict));
    }
    let (outcome, reasons, _) = fixture.assess(SourceCohortAgreement::Missing)?.into_parts();
    assert_eq!(outcome, CandidateOutcome::Unresolved);
    assert!(reasons.contains(ParticipationReason::CohortUnresolved));
    Ok(())
}

#[tokio::test]
async fn foreign_generation_subject_result_and_date_are_refused() -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let mut query = fixture.query()?;
    query.binding.frozen.lineage = ObjectRef::parse("foreign-store")?;
    assert!(matches!(
        assess_retained_source_participation(
            query,
            &fixture.performance,
            &fixture.source,
            SourceCohortAgreement::Resolved(GradYear::CO2027)
        ),
        Err(SourceParticipationError::ForeignSnapshot)
    ));
    // These direct projections reach source validation without append-only merging.
    for field in [
        "result",
        "missing owner",
        "wrong owner",
        "missing date",
        "wrong date",
    ] {
        let mut performance = fixture.performance.clone();
        match field {
            "result" => performance.source_key = "milesplit_result:999".into(),
            "missing owner" => performance.source_athlete = None,
            "wrong owner" => {
                performance.source_athlete = Some(SourceIdentity::new(
                    SourceNamespace::MilesplitAthlete,
                    "14222593",
                ));
            }
            "missing date" => performance.date.clear(),
            "wrong date" => performance.date = "2026-03-28".into(),
            _ => return Err("unknown synthetic mutation".into()),
        }
        assert!(
            matches!(
                assess_retained_source_participation(
                    fixture.query()?,
                    &performance,
                    &fixture.source,
                    SourceCohortAgreement::Resolved(GradYear::CO2027)
                ),
                Err(SourceParticipationError::ForeignPerformance)
            ),
            "{field}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn future_acquisition_and_unknown_policy_fail_closed() -> TestResult {
    let mut fixture = Fixture::prepare(Some("High School")).await?;
    let mut query = fixture.query()?;
    query.binding.frozen.captured_at =
        chrono::DateTime::parse_from_rfc3339("2026-09-30T00:00:00Z")?.with_timezone(&chrono::Utc);
    assert!(matches!(
        assess_retained_source_participation(
            query,
            &fixture.performance,
            &fixture.source,
            SourceCohortAgreement::Resolved(GradYear::CO2027)
        ),
        Err(SourceParticipationError::Assessment(
            ParticipationInputError::AcquisitionAfterFrozenInstant
        ))
    ));
    fixture.profile = profile::profile(census_domain::model::ContractRefV1::new(
        1,
        2,
        &"a".repeat(64),
    )?)?;
    fixture.profile_id = fixture.profile.id()?;
    assert!(matches!(
        fixture.assess(SourceCohortAgreement::Resolved(GradYear::CO2027)),
        Err(SourceParticipationError::Assessment(
            ParticipationInputError::UnsupportedPolicy
        ))
    ));
    Ok(())
}

#[tokio::test]
async fn synthetic_high_school_division_at_a_multi_day_meet_keeps_date_unresolved() -> TestResult {
    let raw = std::str::from_utf8(fixture::RAW)?
        .replace("\"endDate\": \"2026-03-27\"", "\"endDate\": \"2026-03-28\"");
    assert_ne!(raw.as_bytes(), fixture::RAW);
    let fixture = Fixture::prepare_raw(Some("High School"), raw.as_bytes()).await?;
    let (outcome, reasons, _) = fixture
        .assess(SourceCohortAgreement::Resolved(GradYear::CO2027))?
        .into_parts();
    assert_eq!(outcome, CandidateOutcome::Unresolved);
    assert!(reasons.contains(ParticipationReason::MissingPerformanceDate));
    Ok(())
}

#[tokio::test]
async fn historical_v2_source_replay_retains_its_date_local_scholastic_meaning() -> TestResult {
    let mut fixture = Fixture::prepare(Some("High School")).await?;
    fixture.profile =
        profile::profile(census_domain::participation_candidates::participation_policy_v2()?)?;
    fixture.profile_id = fixture.profile.id()?;
    let (outcome, reasons, _) = fixture
        .assess(SourceCohortAgreement::Resolved(GradYear::CO2027))?
        .into_parts();
    assert_eq!(outcome, CandidateOutcome::EligibleCandidate);
    assert!(reasons.is_empty());
    Ok(())
}
