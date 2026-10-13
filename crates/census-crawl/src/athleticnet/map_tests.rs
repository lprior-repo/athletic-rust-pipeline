use super::*;
use census_domain::model::{ExactSeconds, GradYear};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn same_identity_athleticnet_rows_preserve_distinct_marks_and_immutable_observations() -> TestResult
{
    let school = CanonicalSchool::mint(UsJurisdiction::Wisconsin, "Test School", "test", None);
    let source_athlete = SourceIdentity::new(
        SourceNamespace::AthleticNet {
            kind: "athlete".into(),
        },
        "111",
    );
    let athlete = CanonicalAthlete::mint(
        &school,
        "Test Athlete",
        GradYear::CO2027,
        Gender::Boys,
        &source_athlete,
    );
    let meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Invite",
        "2026-05-01",
        CompetitionLevel::Unknown,
    );
    let source = SourceRef::new("athleticnet", Some(profile_url(111)));
    let mut accumulated = Accumulator::default();
    let first = Mark::TimeSeconds(ExactSeconds::parse("114.12")?);
    let second = Mark::TimeSeconds(ExactSeconds::parse("115.12")?);
    for mark in [&first, &second, &first, &second] {
        store_performance(
            &mut accumulated,
            &source,
            "2026-10-01",
            PerformanceInput {
                athlete: &athlete,
                source_athlete: source_athlete.clone(),
                school: &school,
                meet: &meet,
                kind: &EventKind::Track800m,
                sport: Some(Sport::OutdoorTrack),
                gender: Gender::Boys,
                school_year: SchoolYear::new(2026).ok_or("season")?,
                performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 1).ok_or("cutoff")?,
                grade: Grade::new(11),
                date: "2026-05-01".into(),
                mark: mark.clone(),
                wind_mps: None,
                place: Some("1"),
                round: None,
                timing: Some(TimingMethod::Fat),
                division: None,
                source_key: "result:1".into(),
                labels: &["800 Meters"],
            },
        )?;
    }
    check!(eq; accumulated.performances.len(), 2);
    let originals: Vec<_> = accumulated.performances.into_values().collect();
    check!(eq; originals.first().ok_or("first result")?.id,
        originals.last().ok_or("second result")?.id);
    check!(originals.iter().any(|row| row.mark == first));
    check!(originals.iter().any(|row| row.mark == second));
    check!(originals
        .iter()
        .all(|row| row.source_athlete.as_ref() == Some(&source_athlete)));
    let directory = tempfile::tempdir()?;
    let store = census_store::Store::open(directory.path())?;
    store.append_many(census_store::Table::Performances, &originals)?;
    let rows: Vec<CanonicalPerformance> = store.scan(census_store::Table::Performances)?;
    check!(eq; rows.len(), 1);
    check!(eq; rows.first().ok_or("canonical result")?.retained_conflicts.len(), 1);
    let mut physical = Vec::new();
    store.snapshot().for_each_observation(
        census_store::Table::Performances,
        |row: CanonicalPerformance| {
            physical.push(row);
            Ok(())
        },
    )?;
    check!(eq; physical.len(), 2);
    check!(originals.iter().all(|row| physical.contains(row)));
    Ok(())
}
