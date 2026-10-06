use super::*;
use census_domain::model::{Confidence, PublishedGraduation};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn athlete_fixture() -> Result<(TempDir, Store, CanonicalAthlete), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Published Cohort High",
        "published cohort high",
        None,
    );
    store.append(Table::Schools, &school)?;
    let athlete = CanonicalAthlete::new(
        &school_id,
        "Published Cohort Runner",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("published-cohort"),
    );
    Ok((dir, store, athlete))
}

fn published(year: GradYear, url: &str) -> PublishedGraduation {
    PublishedGraduation {
        grad_year: year,
        source: SourceRef::new("wiaa_results", Some(url.to_owned())),
    }
}

fn assert_public_conflict(store: &Store, athlete: &CanonicalAthlete) -> TestResult {
    check!(athlete.has_cohort_conflict());
    check!(eq; athlete.derived_cohort_confidence(), Some(Confidence::LOW));
    store.append(Table::Athletes, athlete)?;
    let dataset = crate::export::ExportDataset::load(store)?;
    check!(dataset.canonical_aliases.is_empty());
    let report = coverage_report(&dataset, Some(2027))?;
    let wi = row(&report, "WI")?;
    check!(eq; report.read.athletes, 1);
    check!(eq; report.off_cohort_athletes, 0);
    check!(eq; wi.athletes, 1);
    check!(eq; wi.grad_verified, 0);
    check!(eq; wi.grad_unresolved, 1);
    check!(eq; wi.identity_conflicts, 1);
    check!(eq; gap(&report, "WI", GapClass::ConflictingIdentity), Some(1));
    check!(eq; gap(&report, "WI", GapClass::MissingGraduationEvidence),
    Some(1));
    Ok(())
}

#[test]
fn conflicting_direct_graduation_years_publish_identity_conflict_gap() -> TestResult {
    let (_dir, store, mut athlete) = athlete_fixture()?;
    athlete.published_graduations = vec![
        published(GradYear::CO2027, "https://wiaa.test/cohort/2027"),
        published(
            GradYear::new(2028).ok_or("invalid fixture graduation year")?,
            "https://wiaa.test/cohort/2028",
        ),
    ];
    athlete.evidence = athlete
        .published_graduations
        .iter()
        .map(|claim| Evidence::parsed(claim.source.clone(), "2026-09-20"))
        .collect();
    check!(athlete.observed_grades.is_empty());
    assert_public_conflict(&store, &athlete)
}

#[test]
fn matching_grade_and_conflicting_direct_year_publish_identity_conflict_gap() -> TestResult {
    let (_dir, store, mut athlete) = athlete_fixture()?;
    let mut grade = observed_grade(11, 2025, "wiaa_results")?;
    grade.source = SourceRef::new(
        "wiaa_results",
        Some("https://wiaa.test/cohort/grade-11-2025".to_owned()),
    );
    check!(eq; grade.grad_year(), Some(GradYear::CO2027));
    athlete
        .evidence
        .push(Evidence::parsed(grade.source.clone(), "2026-09-20"));
    athlete.observed_grades.push(grade);
    let claim = published(
        GradYear::new(2028).ok_or("invalid fixture graduation year")?,
        "https://wiaa.test/cohort/published-2028",
    );
    athlete
        .evidence
        .push(Evidence::parsed(claim.source.clone(), "2026-09-20"));
    athlete.published_graduations.push(claim);
    assert_public_conflict(&store, &athlete)
}
